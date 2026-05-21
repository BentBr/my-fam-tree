//! Tree assembly for `GET /api/v1/relationships`.
//!
//! Pulls all persons, parent links, and partnerships for a family and folds
//! them into a single `TreePayload` the FE can render without further round
//! trips. Pure orchestration over repo traits — no SQL here.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use my_family_domain::{
    FamilyId, ParentLink, ParentLinkRepo, Partnership, PartnershipRepo, Person, PersonId,
    PersonRepo,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// One person + their relationship adjacency lists, in the shape the FE
/// consumes. `parent_ids` and `partner_ids` are denormalized so the SVG
/// layout doesn't have to join `parent_edges` / `partner_edges` itself.
#[derive(Debug, Serialize, ToSchema)]
pub struct TreeNode {
    pub id: Uuid,
    pub given_name: String,
    pub family_name: String,
    pub birth_date: Option<NaiveDate>,
    pub death_date: Option<NaiveDate>,
    pub parent_ids: Vec<Uuid>,
    pub partner_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EdgePair {
    pub a: Uuid,
    pub b: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TreePayload {
    pub nodes: Vec<TreeNode>,
    pub parent_edges: Vec<EdgePair>,
    pub partner_edges: Vec<EdgePair>,
}

/// Phase 2a returns the entire tree per request. Phase 4+ may switch to a
/// paginated/center-on-id variant; the FE then needs an explicit cursor in
/// the response — for now `nodes` is bounded by `MAX_NODES`.
const MAX_NODES: u32 = 1_000;

pub async fn build_tree(
    persons: &Arc<dyn PersonRepo>,
    parent_links: &Arc<dyn ParentLinkRepo>,
    partnerships: &Arc<dyn PartnershipRepo>,
    family_id: FamilyId,
) -> anyhow::Result<TreePayload> {
    let people = persons.list_for_family(family_id, None, MAX_NODES).await?;
    let parents = parent_links.list_for_family(family_id).await?;
    let parts = partnerships.list_for_family(family_id).await?;

    let mut parents_by_child: HashMap<PersonId, Vec<PersonId>> = HashMap::new();
    let mut partners_of: HashMap<PersonId, Vec<PersonId>> = HashMap::new();
    for ParentLink { child_id, parent_id, .. } in &parents {
        parents_by_child.entry(*child_id).or_default().push(*parent_id);
    }
    for Partnership { partner_a_id: a, partner_b_id: b, .. } in &parts {
        partners_of.entry(*a).or_default().push(*b);
        partners_of.entry(*b).or_default().push(*a);
    }

    let nodes = people
        .iter()
        .map(|p: &Person| TreeNode {
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
        })
        .collect();

    Ok(TreePayload {
        nodes,
        parent_edges: parents
            .into_iter()
            .map(|p| EdgePair { a: p.child_id.into_uuid(), b: p.parent_id.into_uuid() })
            .collect(),
        partner_edges: parts
            .into_iter()
            .map(|p| EdgePair { a: p.partner_a_id.into_uuid(), b: p.partner_b_id.into_uuid() })
            .collect(),
    })
}
