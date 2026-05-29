//! `/persons/{id}/contacts` + `/contacts/{id}` — per-person contact CRUD.
//!
//! Role gates:
//! - `list`, `get`: any member of the active family. `admins_only`
//!   rows are filtered out for `user` role.
//! - `create`, `update`, `delete`: admin/owner on any person; `user`
//!   role only when `person.linked_user_id == self.user_id`. A `user`
//!   that tries to touch someone else's contact gets
//!   [`ApiError::ContactNotEditable`].
//!
//! Value shape — **named field per kind** so every row's JSONB carries a
//! self-documenting key:
//!
//! - `email`   → `{ "email":  "<addr>"  }`
//! - `phone`   → `{ "number": "<num>"   }`
//! - `url`     → `{ "url":    "<href>"  }`
//! - `other`   → `{ "text":   "<value>" }`
//! - `address` → `{ "street": …, "house_number": …, "zip": …, "city": …, "country": … }`
//!
//! Multiple rows per kind on one person are supported — `(person_id,
//! kind)` is **not** unique. The `label` field distinguishes them
//! ("Work" / "Mobile" / "Private" / "Home" / …).
//!
//! Email validation: when `kind == "email"`, `value.email` must look like
//! an email; we still accept the legacy `value.v` and bare-string shapes
//! for a transitional grace window so older clients don't break mid-deploy.
//!
//! Every mutating handler records an audit-log entry via
//! [`crate::services::audit::record`]; failures are swallowed so an
//! audit hiccup never blocks the request.

use actix_web::{HttpRequest, delete, get, patch, post, web};
use my_family_domain::{
    ContactDraft, ContactKind, ContactRepoError, ContactVisibility, PersonId, Role,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims_with_family;
use crate::response::{ApiResponse, NullResponseBody};
use crate::services::audit;
use crate::validation::{email_invalid, looks_like_email, string_too_long};
use crate::{ApiError, AppState, FieldViolation, response_body};

// ---------------------------------------------------------------------------
// DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct ContactInput {
    /// One of `email`, `phone`, `address`, `url`, `other`.
    pub kind: String,
    #[serde(default)]
    pub label: String,
    pub value: Value,
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

fn default_visibility() -> String {
    "family".into()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ContactView {
    pub id: Uuid,
    pub person_id: Uuid,
    pub kind: String,
    pub label: String,
    pub value: Value,
    pub visibility: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ContactListRes {
    pub contacts: Vec<ContactView>,
}

response_body!(pub ContactViewResponseBody, ContactView);
response_body!(pub ContactListResponseBody, ContactListRes);

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn parse_kind(s: &str) -> Option<ContactKind> {
    match s {
        "email" => Some(ContactKind::Email),
        "phone" => Some(ContactKind::Phone),
        "address" => Some(ContactKind::Address),
        "url" => Some(ContactKind::Url),
        "other" => Some(ContactKind::Other),
        _ => None,
    }
}

fn parse_visibility(s: &str) -> Option<ContactVisibility> {
    match s {
        "family" => Some(ContactVisibility::Family),
        "admins_only" => Some(ContactVisibility::AdminsOnly),
        _ => None,
    }
}

/// Extract the underlying string from a contact `value` of `kind == email`.
///
/// We accept three shapes for backwards-compat:
/// - canonical (current): `{ "email": "foo@bar" }`
/// - legacy generic: `{ "v": "foo@bar" }` (from an early Phase 3 draft;
///   the FE was briefly emitting this and pre-existing seed rows may
///   still carry it)
/// - bare JSON string: `"foo@bar"`
///
/// Returns `None` for any other shape so the caller can surface a clean
/// validation error.
fn email_value_as_str(value: &Value) -> Option<&str> {
    if let Some(s) = value.as_str() {
        return Some(s);
    }
    if let Some(s) = value.get("email").and_then(Value::as_str) {
        return Some(s);
    }
    value.get("v").and_then(Value::as_str)
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

// Free-text length caps (security audit MEDIUM).
const LABEL_MAX: u32 = 100;
const VALUE_MAX: u32 = 500;

fn input_to_draft(i: ContactInput) -> Result<ContactDraft, ApiError> {
    let kind = parse_kind(&i.kind).ok_or_else(|| {
        ApiError::Validation(vec![FieldViolation::new(
            "/kind",
            "validation.enum",
            "must be one of: email, phone, address, url, other",
        )])
    })?;
    let visibility = parse_visibility(&i.visibility).ok_or_else(|| {
        ApiError::Validation(vec![FieldViolation::new(
            "/visibility",
            "validation.enum",
            "must be one of: family, admins_only",
        )])
    })?;
    if u32::try_from(i.label.chars().count()).unwrap_or(u32::MAX) > LABEL_MAX {
        return Err(string_too_long("/label", LABEL_MAX));
    }
    // `value` is a JSON `Value` shape — stringify before counting. Caps the
    // total JSON character width of the field; a structured address with
    // 5 fields × 100 chars still fits comfortably under 500.
    let value_str = i.value.to_string();
    if u32::try_from(value_str.chars().count()).unwrap_or(u32::MAX) > VALUE_MAX {
        return Err(string_too_long("/value", VALUE_MAX));
    }
    if kind == ContactKind::Email {
        match email_value_as_str(&i.value) {
            Some(s) if looks_like_email(s) => {}
            _ => return Err(email_invalid("/value")),
        }
    }
    Ok(ContactDraft { kind, label: i.label, value: i.value, visibility })
}

fn to_view(c: my_family_domain::Contact) -> ContactView {
    ContactView {
        id: c.id,
        person_id: c.person_id.into_uuid(),
        kind: c.kind.as_db().to_string(),
        label: c.label,
        value: c.value,
        visibility: c.visibility.as_db().to_string(),
    }
}

fn map_repo_err(e: ContactRepoError, id: Option<Uuid>) -> ApiError {
    match e {
        ContactRepoError::NotFound => ApiError::ContactNotFound { id },
        ContactRepoError::Db(_) => internal(e),
    }
}

// ---------------------------------------------------------------------------
// GET /persons/{id}/contacts
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}/contacts",
    operation_id = "contacts_list_for_person",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Contacts for the person", body = ContactListResponseBody),
        (status = 401, description = "No session"),
        (status = 404, description = "Person not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "contacts",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/persons/{id}/contacts")]
pub async fn list_for_person(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<ContactListRes>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    let person_id = PersonId::from_uuid(path.into_inner());
    let _person = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| internal(e.to_string()))?
        .ok_or(ApiError::PersonNotFound { id: Some(person_id.into_uuid()) })?;
    let mut contacts =
        state.contacts.list_for_person(person_id).await.map_err(|e| map_repo_err(e, None))?;
    if active.role == Role::User {
        contacts.retain(|c| c.visibility == ContactVisibility::Family);
    }
    Ok(ApiResponse::ok(ContactListRes { contacts: contacts.into_iter().map(to_view).collect() }))
}

// ---------------------------------------------------------------------------
// POST /persons/{id}/contacts
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/persons/{id}/contacts",
    operation_id = "contacts_create",
    request_body = ContactInput,
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Contact created", body = ContactViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit contacts on this person"),
        (status = 404, description = "Person not found in this family"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "contacts",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/persons/{id}/contacts")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ContactInput>,
) -> Result<ApiResponse<ContactView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let person_id = PersonId::from_uuid(path.into_inner());
    let person = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| internal(e.to_string()))?
        .ok_or(ApiError::PersonNotFound { id: Some(person_id.into_uuid()) })?;
    if active.role == Role::User && person.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::ContactNotEditable);
    }
    let draft = input_to_draft(body.into_inner())?;
    let created =
        state.contacts.create(person_id, draft).await.map_err(|e| map_repo_err(e, None))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "create",
        "contact",
        Some(created.id),
        serde_json::json!({
            "person_id": person_id.into_uuid(),
            "kind": created.kind.as_db(),
        }),
    )
    .await;
    Ok(ApiResponse::ok(to_view(created)))
}

// ---------------------------------------------------------------------------
// PATCH /contacts/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/contacts/{id}",
    operation_id = "contacts_update",
    request_body = ContactInput,
    params(("id" = Uuid, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Contact updated", body = ContactViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this contact"),
        (status = 404, description = "Contact not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "contacts",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/contacts/{id}")]
pub async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<ContactInput>,
) -> Result<ApiResponse<ContactView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let existing = state
        .contacts
        .find_by_id(id)
        .await
        .map_err(|e| map_repo_err(e, Some(id)))?
        .ok_or(ApiError::ContactNotFound { id: Some(id) })?;
    let person = state
        .persons
        .find_in_family(active.id, existing.person_id)
        .await
        .map_err(|e| internal(e.to_string()))?
        .ok_or(ApiError::ContactNotFound { id: Some(id) })?;
    if active.role == Role::User && person.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::ContactNotEditable);
    }
    let draft = input_to_draft(body.into_inner())?;
    let updated = state.contacts.update(id, draft).await.map_err(|e| map_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "update",
        "contact",
        Some(id),
        serde_json::json!({
            "person_id": existing.person_id.into_uuid(),
            "kind": updated.kind.as_db(),
        }),
    )
    .await;
    Ok(ApiResponse::ok(to_view(updated)))
}

// ---------------------------------------------------------------------------
// DELETE /contacts/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/contacts/{id}",
    operation_id = "contacts_delete",
    params(("id" = Uuid, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Contact deleted", body = NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this contact"),
        (status = 404, description = "Contact not found"),
    ),
    security(("cookie_access" = [])),
    tag = "contacts",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/contacts/{id}")]
pub async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let existing = state
        .contacts
        .find_by_id(id)
        .await
        .map_err(|e| map_repo_err(e, Some(id)))?
        .ok_or(ApiError::ContactNotFound { id: Some(id) })?;
    let person = state
        .persons
        .find_in_family(active.id, existing.person_id)
        .await
        .map_err(|e| internal(e.to_string()))?
        .ok_or(ApiError::ContactNotFound { id: Some(id) })?;
    if active.role == Role::User && person.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::ContactNotEditable);
    }
    state.contacts.delete(id).await.map_err(|e| map_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "delete",
        "contact",
        Some(id),
        serde_json::json!({
            "person_id": existing.person_id.into_uuid(),
        }),
    )
    .await;
    Ok(ApiResponse::ok(Value::Null))
}
