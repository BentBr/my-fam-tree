//! Hardcoded UUIDs for the deterministic dev seed.
//!
//! Structured hex blocks make the seeded rows immediately recognisable in
//! `psql` inspection:
//! - users:        `0x…0001_…`
//! - family:       `0x…0002_…`
//! - persons:      `0x…0003_…`
//! - partnerships: `0x…0004_…`
//!
//! Hardcoding all foreign-key ids (including partnership rows) lets the
//! seeder use `ON CONFLICT (id) DO UPDATE` everywhere, which keeps the
//! seed truly idempotent even for partnerships that have a non-null
//! `ended_on` (the partial unique index on `(a, b, kind) WHERE ended_on
//! IS NULL` can't be used as a conflict target for closed rows).

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Users + family
// ---------------------------------------------------------------------------

/// Seeded admin user (owner of the seeded family).
pub const SEED_ADMIN_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0001);
/// Seeded user "Alice" (admin role).
pub const SEED_ALICE_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0002);
/// Seeded user "Bob" (user role).
pub const SEED_BOB_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0003);

/// The single seeded family.
pub const SEED_FAMILY_ID: Uuid = Uuid::from_u128(0x0000_0002_0000_0000_0000_0000_0000_0001);

// ---------------------------------------------------------------------------
// Persons — original 8
// ---------------------------------------------------------------------------

// G1 Müller line.
pub const SEED_PERSON_OTTO_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0001);
pub const SEED_PERSON_HANNELORE_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0002);
// G1 Schmidt line.
pub const SEED_PERSON_WERNER_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0003);
pub const SEED_PERSON_GRETA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0004);
// G2 Müller (Klaus is the seeded admin's linked person).
pub const SEED_PERSON_KLAUS_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0005);
pub const SEED_PERSON_ANNA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0006);
// G3 Müller.
pub const SEED_PERSON_LINA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0007);
pub const SEED_PERSON_MAX_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0008);

// ---------------------------------------------------------------------------
// Persons — edge-case extensions
// ---------------------------------------------------------------------------

// G1 Bauer line — a third grandparent couple, Lotte widowed by Friedrich.
pub const SEED_PERSON_FRIEDRICH_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0009);
pub const SEED_PERSON_LOTTE_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000a);
// G2 Klaus's first wife — divorced; Brigitte's own parents aren't seeded
// (she "married in"), so she's a tree root.
pub const SEED_PERSON_BRIGITTE_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000b);
// G2 Anna's younger sister.
pub const SEED_PERSON_SABINE_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000c);
// G2 Sabine's same-sex partner.
pub const SEED_PERSON_JULIA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000d);
// G2 Markus — Friedrich+Lotte's son; single-parent to Tom.
pub const SEED_PERSON_MARKUS_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000e);
// G3 half-sibling Felix — Klaus + Brigitte's son, Anna step-mother.
pub const SEED_PERSON_FELIX_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_000f);
// G3 youngest Müller — Klaus + Anna.
pub const SEED_PERSON_MIA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0010);
// G3 adopted daughter of Sabine + Julia.
pub const SEED_PERSON_LENA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0011);
// G3 Tom — Markus's son.
pub const SEED_PERSON_TOM_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0012);
// G4 Lina's children.
pub const SEED_PERSON_EMMA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0013);
pub const SEED_PERSON_NOAH_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0014);
// Two more Klaus partners: one historical (Karin), one current/concurrent
// (Yuki). Round out the ex-spouse + multi-partner edge cases the FE tree
// layout exercises.
pub const SEED_PERSON_KARIN_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0015);
pub const SEED_PERSON_YUKI_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0016);

// Standalone couples — no children, no other relations in the seed — so
// the tree canvas can show isolated examples of the partnership-glyph
// treatments without dragging in the Müller / Schmidt graph context.
//   Sven  + Maren  — active marriage (golden rings glyph)
//   Heinz + Ursula — divorced marriage (greyed rings + muted line)
//   Lars  + Mette  — separated non-marriage partnership (greyed heart)
pub const SEED_PERSON_SVEN_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0017);
pub const SEED_PERSON_MAREN_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0018);
pub const SEED_PERSON_HEINZ_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0019);
pub const SEED_PERSON_URSULA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001a);
pub const SEED_PERSON_LARS_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001b);
pub const SEED_PERSON_METTE_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001c);

// "Krause" subtree — 8 persons that reproduce the layout edge cases
// the user pointed out from his real family tree. Pinned in the seed so
// he can sign in, view /tree, and visually confirm the current
// (broken) layout vs the expected one after the layout pipeline is
// updated. The cases:
//   1. Three siblings (Lars 1985, Marie 1987, Tim 1989) where adding
//      Tim's spouse (Mia) currently shuffles Tim out of the right end
//      of the sibling row.
//   2. Two unpartnered mothers (Greta 1912, Anneliese 1921) whose
//      children (Hubert, Bernhard) sit on opposite sides of the row
//      below — their cross order causes the parent edges to cross.
//   3. An in-married couple (Tim + Mia) where each spouse has parents
//      on opposite sides; the couple's own order on its row should
//      put each spouse closer to their own parents to avoid crossings.
pub const SEED_PERSON_K_GRETA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001d);
pub const SEED_PERSON_K_ANNELIESE_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001e);
pub const SEED_PERSON_K_HUBERT_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_001f);
pub const SEED_PERSON_K_SARA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0020);
pub const SEED_PERSON_K_BERNHARD_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0021);
pub const SEED_PERSON_K_HELGA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0022);
pub const SEED_PERSON_K_LARS_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0023);
pub const SEED_PERSON_K_MARIE_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0024);
pub const SEED_PERSON_K_TIM_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0025);
pub const SEED_PERSON_K_MIA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0026);

// ---------------------------------------------------------------------------
// Partnerships — hardcoded so the seed can keep `ON CONFLICT (id) DO UPDATE`
// semantics for closed (ended_on IS NOT NULL) rows too.
// ---------------------------------------------------------------------------

pub const SEED_PARTNERSHIP_OTTO_HANNELORE_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0001);
pub const SEED_PARTNERSHIP_WERNER_GRETA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0002);
pub const SEED_PARTNERSHIP_KLAUS_ANNA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0003);
pub const SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0004);
pub const SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0005);
pub const SEED_PARTNERSHIP_SABINE_JULIA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0006);
// Klaus's additional partnerships — separation (ended, non-divorce) and a
// concurrent open civil_union alongside the Klaus + Anna marriage.
pub const SEED_PARTNERSHIP_KLAUS_KARIN_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0007);
pub const SEED_PARTNERSHIP_KLAUS_YUKI_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0008);
// Standalone glyph demonstrations: active marriage, divorced marriage,
// and separated non-marriage partnership. Used by the tree-canvas
// glyph treatment so the seed can render each state without graph
// context (no children, no other relations).
pub const SEED_PARTNERSHIP_SVEN_MAREN_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0009);
pub const SEED_PARTNERSHIP_HEINZ_URSULA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_000a);
pub const SEED_PARTNERSHIP_LARS_METTE_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_000b);
// Krause subtree partnerships — see the K_* person ids above.
pub const SEED_PARTNERSHIP_K_HUBERT_SARA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_000c);
pub const SEED_PARTNERSHIP_K_BERNHARD_HELGA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_000d);
pub const SEED_PARTNERSHIP_K_TIM_MIA_ID: Uuid =
    Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_000e);

/// Expected counts for the deterministic seed — surfaced for the test
/// asserts so they don't drift from the actual data.
pub const SEED_PERSON_COUNT: usize = 38;
