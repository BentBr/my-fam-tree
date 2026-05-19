//! Exhaustive guard: every `ErrorCode` variant must map to a non-success HTTP
//! status, have a non-empty title and slug, and slugs must be unique. Add a new
//! variant and forget to update `http_status`/`title`/`slug`/`ALL` and this
//! test fails — the compiler doesn't enforce exhaustiveness across separate
//! `match` blocks so this safety net does.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use my_family_api::ErrorCode;

#[test]
fn every_code_has_status_title_and_slug() {
    for code in ErrorCode::ALL {
        let status = code.http_status();
        assert!(status.as_u16() >= 400, "code {code:?} maps to non-error status {status}");
        assert!(!code.title().is_empty(), "code {code:?} has empty title");
        assert!(!code.slug().is_empty(), "code {code:?} has empty slug");
        assert!(
            code.slug().chars().all(|c| c.is_ascii_lowercase() || c == '.' || c == '_'),
            "slug {} has illegal chars",
            code.slug(),
        );
    }
}

#[test]
fn slugs_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for code in ErrorCode::ALL {
        assert!(seen.insert(code.slug()), "duplicate slug: {}", code.slug());
    }
}

#[test]
fn all_array_length_matches_variant_count() {
    // Update this constant whenever an `ErrorCode` variant is added or removed.
    // The compiler doesn't enforce this; this test does.
    assert_eq!(
        ErrorCode::ALL.len(),
        20,
        "ErrorCode::ALL must list every variant exactly once",
    );
}
