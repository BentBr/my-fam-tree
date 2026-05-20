//! Shared validation helpers.
//!
//! Every error here carries a stable i18n key in the form
//! `validation.<snake_case>`. The FE looks up the key in its catalog and may
//! interpolate any `params` we attach via [`FieldViolation::with_param`]. The
//! English `message` is the fallback for non-FE clients.
//!
//! Helpers live here (not in `routes/auth.rs`) so the same email-syntax check
//! and the same validation envelopes are used by every route that needs them
//! — `/auth/magic-link`, `/families`, `/families/{id}/invites`, etc.

use crate::{ApiError, FieldViolation};

/// Lightweight email syntax check.
///
/// NOT RFC 5322 — full validation belongs at the SMTP send layer. Here we
/// just reject typos so the rate-limiter key isn't polluted with garbage and
/// so the user sees the "must be an email" error before we issue a magic
/// link or an invite.
///
/// Rules enforced:
///  - Exactly one `@`.
///  - Local part: 1–64 ASCII chars from `[A-Za-z0-9._%+-]`; no leading/trailing
///    `.` and no consecutive `..`.
///  - Domain: 1+ ASCII labels separated by `.`; each label 1–63 chars from
///    `[A-Za-z0-9-]`, no leading/trailing `-`.
///  - TLD: ≥ 2 ASCII letters (no digits, no hyphens). This rejects `a@b.c`
///    and `a@b.c1` but accepts `a@b.co`, `a@b.museum`, etc.
#[must_use]
pub fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > 64 || domain.is_empty() {
        return false;
    }
    if domain.contains('@') {
        return false;
    }
    if !is_valid_local_part(local) {
        return false;
    }
    is_valid_domain(domain)
}

fn is_valid_local_part(local: &str) -> bool {
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    local.bytes().all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~.-".contains(&b))
}

fn is_valid_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    if !labels.iter().all(|label| is_valid_domain_label(label)) {
        return false;
    }
    // TLD must be ≥ 2 ASCII letters.
    let Some(tld) = labels.last() else {
        return false;
    };
    tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

fn is_valid_domain_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// `validation.email_invalid` violation on `path`.
#[must_use]
pub fn email_invalid(path: &str) -> ApiError {
    ApiError::Validation(vec![FieldViolation::new(
        path,
        "validation.email_invalid",
        "must be an email",
    )])
}

/// `validation.value_required` violation on `path`. Use this whenever a
/// required field arrived empty/blank.
#[must_use]
pub fn value_required(path: &str) -> ApiError {
    ApiError::Validation(vec![FieldViolation::new(
        path,
        "validation.value_required",
        "this field is required",
    )])
}

/// `validation.role_invalid` violation on `path`. Use for "cannot invite as
/// owner" and similar role-shape errors that aren't an `InsufficientRole`
/// authorisation failure.
#[must_use]
pub fn role_invalid(path: &str, msg: &str) -> ApiError {
    ApiError::Validation(vec![FieldViolation::new(path, "validation.role_invalid", msg)])
}

/// `validation.invite_email_mismatch` violation. Surfaced by `/invites/accept`
/// when the signed-in user's email doesn't match the address the invite was
/// originally sent to.
#[must_use]
pub fn invite_email_mismatch(path: &str) -> ApiError {
    ApiError::Validation(vec![FieldViolation::new(
        path,
        "validation.invite_email_mismatch",
        "invite was sent to a different email",
    )])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_email_accepts_well_formed_addresses() {
        assert!(looks_like_email("a@b.co"));
        assert!(looks_like_email("user@example.org"));
        assert!(looks_like_email("user.name+tag@sub.example.org"));
        assert!(looks_like_email("user_123@example.co.uk"));
    }

    #[test]
    fn looks_like_email_rejects_malformed_addresses() {
        // No @ / empty / multiple @
        assert!(!looks_like_email(""));
        assert!(!looks_like_email("nope"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("user@"));
        assert!(!looks_like_email("a@@b.co"));
        assert!(!looks_like_email("a@b@c.co"));
        // No TLD or single-letter TLD
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("a@b.c"));
        // TLD with digits or hyphens
        assert!(!looks_like_email("a@b.c1"));
        assert!(!looks_like_email("a@b.c-d"));
        // Leading/trailing dot or consecutive dots in local part
        assert!(!looks_like_email(".user@example.com"));
        assert!(!looks_like_email("user.@example.com"));
        assert!(!looks_like_email("us..er@example.com"));
        // Domain label leading/trailing hyphen
        assert!(!looks_like_email("user@-example.com"));
        assert!(!looks_like_email("user@example-.com"));
        // Non-ASCII in local/domain
        assert!(!looks_like_email("üser@example.com"));
        assert!(!looks_like_email("user@exämple.com"));
        // Whitespace
        assert!(!looks_like_email("us er@example.com"));
        assert!(!looks_like_email("user@example .com"));
    }

    #[test]
    fn email_invalid_uses_stable_path_and_code() {
        match email_invalid("/email") {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/email");
                assert_eq!(v[0].code, "validation.email_invalid");
                assert!(v[0].params.is_empty());
            }
            _ => panic!("expected Validation"),
        }
    }

    #[test]
    fn value_required_uses_stable_path_and_code() {
        match value_required("/name") {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/name");
                assert_eq!(v[0].code, "validation.value_required");
            }
            _ => panic!("expected Validation"),
        }
    }

    #[test]
    fn role_invalid_uses_stable_path_and_carries_message() {
        match role_invalid("/role", "cannot invite as owner") {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/role");
                assert_eq!(v[0].code, "validation.role_invalid");
                assert_eq!(v[0].message, "cannot invite as owner");
            }
            _ => panic!("expected Validation"),
        }
    }

    #[test]
    fn invite_email_mismatch_uses_stable_path_and_code() {
        match invite_email_mismatch("/token") {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/token");
                assert_eq!(v[0].code, "validation.invite_email_mismatch");
            }
            _ => panic!("expected Validation"),
        }
    }
}
