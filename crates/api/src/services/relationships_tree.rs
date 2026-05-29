//! Tree assembly for `GET /api/v1/relationships`.
//!
//! Pulls all persons, parent links, and partnerships for a family and folds
//! them into a single `TreePayload` the FE can render without further round
//! trips. Pure orchestration over repo traits — no SQL here.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::NaiveDate;
use my_family_domain::{
    FamilyId, ParentLink, ParentLinkRepo, Partnership, PartnershipRepo, PersonFavouriteRepo,
    PersonId, PersonRepo, UserId,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// One person plus their relationship adjacency lists.
///
/// `parent_ids` and `partner_ids` are denormalized so the FE's SVG layout
/// doesn't have to join `parent_edges` / `partner_edges` itself.
#[derive(Debug, Serialize, ToSchema)]
pub struct TreeNode {
    pub id: Uuid,
    pub given_name: String,
    pub family_name: String,
    pub birth_date: Option<NaiveDate>,
    pub death_date: Option<NaiveDate>,
    pub parent_ids: Vec<Uuid>,
    pub partner_ids: Vec<Uuid>,
    /// Set when this person row maps to a `users.id`. Used by the FE to
    /// resolve "the signed-in user's own node" so the tree can auto-center
    /// on them on first load.
    pub linked_user_id: Option<Uuid>,
    /// Time-limited presigned URL for this person's photo, or `null` when
    /// no photo is set. Re-presigned on every read; do NOT cache. The FE
    /// uses this directly to render the avatar bubble on the tree node so
    /// the initials fallback only shows when the column is truly empty.
    pub photo_url: Option<String>,
    /// Per-user mark for the signed-in caller. Two users seeing the same
    /// tree see independent values — favouriting is private. Resolved
    /// against `person_favourites` at request time.
    pub is_favourite_for_me: bool,
}

/// A parent → child edge with kind metadata.
///
/// `a` is the child, `b` is the parent — matches the historical
/// `EdgePair` orientation the FE layout code expects. `kind` powers
/// the drawer's inline "change parent-link kind" affordance.
#[derive(Debug, Serialize, ToSchema)]
pub struct EdgePair {
    pub a: Uuid,
    pub b: Uuid,
    pub kind: String,
}

/// A partnership edge with id, kind, and lifecycle dates.
///
/// The id is what `PATCH /partnerships/{id}` keys on — without it the
/// FE has no way to edit or end an existing partnership inline.
#[derive(Debug, Serialize, ToSchema)]
pub struct PartnerEdge {
    pub id: Uuid,
    pub a: Uuid,
    pub b: Uuid,
    pub kind: String,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    pub end_reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TreePayload {
    pub nodes: Vec<TreeNode>,
    pub parent_edges: Vec<EdgePair>,
    pub partner_edges: Vec<PartnerEdge>,
}

/// Phase 2a returns the entire tree per request. Phase 4+ may switch to a
/// paginated/center-on-id variant; the FE then needs an explicit cursor in
/// the response — for now `nodes` is bounded by `MAX_NODES`.
const MAX_NODES: u32 = 1_000;

/// Wall-clock TTL for the per-node presigned photo URLs. Matches the
/// `PersonView` TTL — one hour, re-presigned on every tree fetch.
const PHOTO_URL_TTL: std::time::Duration = std::time::Duration::from_hours(1);

/// Assemble the full tree payload for a family.
///
/// # Errors
/// Returns any error surfaced by the underlying repos (DB connectivity,
/// query failure).
pub async fn build_tree(
    persons: &Arc<dyn PersonRepo>,
    parent_links: &Arc<dyn ParentLinkRepo>,
    partnerships: &Arc<dyn PartnershipRepo>,
    favourites: &Arc<dyn PersonFavouriteRepo>,
    object_store: &Arc<dyn my_family_storage::ObjectStore>,
    family_id: FamilyId,
    user_id: UserId,
) -> anyhow::Result<TreePayload> {
    let people = persons.list_for_family(family_id, None, MAX_NODES).await?;
    let parents = parent_links.list_for_family(family_id).await?;
    let parts = partnerships.list_for_family(family_id).await?;
    let fav_set: HashSet<PersonId> = favourites.list_for_user(user_id, family_id).await?;

    let mut parents_by_child: HashMap<PersonId, Vec<PersonId>> = HashMap::new();
    let mut partners_of: HashMap<PersonId, Vec<PersonId>> = HashMap::new();
    for ParentLink { child_id, parent_id, .. } in &parents {
        parents_by_child.entry(*child_id).or_default().push(*parent_id);
    }
    for Partnership { partner_a_id: a, partner_b_id: b, .. } in &parts {
        partners_of.entry(*a).or_default().push(*b);
        partners_of.entry(*b).or_default().push(*a);
    }

    // Presign in a sequential pass — `presigned_get` is fast (SigV4 only,
    // no network I/O) and the await keeps the future Send. A concurrent
    // join_all here would shave milliseconds but force a Send-bound
    // closure that the actix arbiter doesn't satisfy.
    let mut nodes = Vec::with_capacity(people.len());
    for p in &people {
        let photo_url = match p.photo_key.as_deref() {
            None => None,
            Some(key) => match object_store.presigned_get(key, PHOTO_URL_TTL).await {
                Ok(url) => Some(url),
                Err(e) => {
                    tracing::warn!(error = ?e, photo_key = %key, "could not presign tree-node photo");
                    None
                }
            },
        };
        nodes.push(TreeNode {
            id: p.id.into_uuid(),
            given_name: p.given_name.clone(),
            family_name: p.family_name.clone(),
            birth_date: p.birth_date,
            death_date: p.death_date,
            parent_ids: parents_by_child
                .get(&p.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(PersonId::into_uuid)
                .collect(),
            partner_ids: partners_of
                .get(&p.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(PersonId::into_uuid)
                .collect(),
            linked_user_id: p.linked_user_id.map(my_family_domain::UserId::into_uuid),
            photo_url,
            is_favourite_for_me: fav_set.contains(&p.id),
        });
    }

    Ok(TreePayload {
        nodes,
        parent_edges: parents
            .into_iter()
            .map(|p| EdgePair {
                a: p.child_id.into_uuid(),
                b: p.parent_id.into_uuid(),
                kind: p.kind.as_db().to_owned(),
            })
            .collect(),
        partner_edges: parts
            .into_iter()
            .map(|p| PartnerEdge {
                id: p.id,
                a: p.partner_a_id.into_uuid(),
                b: p.partner_b_id.into_uuid(),
                kind: p.kind.as_db().to_owned(),
                started_on: p.started_on,
                ended_on: p.ended_on,
                end_reason: p.end_reason.map(|r| r.as_db().to_owned()),
            })
            .collect(),
    })
}
