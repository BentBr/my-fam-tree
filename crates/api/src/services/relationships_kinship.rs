//! Kinship assembly for `GET /api/v1/relationships/kinship`.
//!
//! Loads the family's parent links + partnerships and delegates to the pure
//! `my_fam_tree_domain::compute_kinship`. The handler has already resolved
//! both persons against the active family. No SQL here.

use std::sync::Arc;

use my_fam_tree_domain::{
    FamilyId, InLawSide, Kinship, ParentLinkRepo, PartnershipRepo, Person, PersonId,
    compute_kinship,
};
use serde::Serialize;
use time::Date;
use utoipa::ToSchema;
use uuid::Uuid;

/// One of the two compared persons.
#[derive(Debug, Serialize, ToSchema)]
pub struct KinshipPerson {
    pub id: Uuid,
    /// Free-form gender as stored on the person. Labels use `male` /
    /// `female`; clients treat every other value as neutral.
    pub gender: String,
}

/// Which side the single partner hop of an in-law relation sits on.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KinshipInLaw {
    /// B is a blood relative of A's partner (mother-in-law, partner's sibling).
    ViaOwnPartner,
    /// B is the partner of A's blood relative (son-in-law, sibling's partner).
    ViaRelativesPartner,
}

/// The partnership behind a direct-partner relation.
#[derive(Debug, Serialize, ToSchema)]
pub struct KinshipPartnership {
    /// `marriage`, `civil_union` or `partnership`.
    pub kind: String,
    /// End date; `null` while the partnership is open.
    pub ended_on: Option<Date>,
    /// `divorce`, `separation` or `death`; `null` while open.
    pub end_reason: Option<String>,
}

/// One relation in descriptor form, read from A's perspective and naming B.
/// The client maps it to a localized label.
#[derive(Debug, Serialize, ToSchema)]
pub struct KinshipRelation {
    /// Generations from A up to the common ancestor (0 when B descends from A).
    pub up: u8,
    /// Generations from the common ancestor down to B (0 when B is A's ancestor).
    pub down: u8,
    /// Known half-siblings: one shared parent and each has another recorded
    /// parent. Only ever set for `up = down = 1`.
    pub half: bool,
    /// The relation runs through a step parent-link or a parent's partner.
    pub step: bool,
    /// Set when the relation crosses one partnership; `null` for blood relations.
    pub in_law: Option<KinshipInLaw>,
    /// Set when A and B are direct partners (`up = down = 0`).
    pub partnership: Option<KinshipPartnership>,
    /// Id of the one partnership the path crosses — the partnership itself
    /// for direct partners, the connecting one for in-law and step relations;
    /// `null` for blood relations. Every other step of `path` is a parent link.
    pub via_partnership_id: Option<Uuid>,
    /// Person ids from A to B, both ends included, no person twice.
    pub path: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KinshipPayload {
    pub person_a: KinshipPerson,
    pub person_b: KinshipPerson,
    /// Blood relations first, then direct partners, then in-law / step
    /// relations; by distance within each group. Empty when unrelated.
    pub relations: Vec<KinshipRelation>,
}

impl From<Kinship> for KinshipRelation {
    fn from(k: Kinship) -> Self {
        Self {
            up: k.up,
            down: k.down,
            half: k.half,
            step: k.step,
            in_law: k.in_law.map(|side| match side {
                InLawSide::ViaOwnPartner => KinshipInLaw::ViaOwnPartner,
                InLawSide::ViaRelativesPartner => KinshipInLaw::ViaRelativesPartner,
            }),
            partnership: k.partnership.map(|p| KinshipPartnership {
                kind: p.kind.as_db().to_owned(),
                ended_on: p.ended_on,
                end_reason: p.end_reason.map(|r| r.as_db().to_owned()),
            }),
            via_partnership_id: k.via_partnership,
            path: k.path.into_iter().map(PersonId::into_uuid).collect(),
        }
    }
}

/// Compute the relations between two persons of `family_id`.
///
/// # Errors
/// Returns any error surfaced by the underlying repos.
pub async fn build_kinship(
    parent_links: &Arc<dyn ParentLinkRepo>,
    partnerships: &Arc<dyn PartnershipRepo>,
    family_id: FamilyId,
    a: &Person,
    b: &Person,
) -> anyhow::Result<KinshipPayload> {
    let links = parent_links.list_for_family(family_id).await?;
    let parts = partnerships.list_for_family(family_id).await?;
    let relations = compute_kinship(&links, &parts, a.id, b.id)
        .into_iter()
        .map(KinshipRelation::from)
        .collect();
    Ok(KinshipPayload {
        person_a: KinshipPerson { id: a.id.into_uuid(), gender: a.gender.clone() },
        person_b: KinshipPerson { id: b.id.into_uuid(), gender: b.gender.clone() },
        relations,
    })
}
