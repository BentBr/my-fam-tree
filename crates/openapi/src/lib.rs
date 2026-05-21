//! Thin shim that re-exports `my_family_api::ApiDoc` so the `openapi-dump`
//! binary and external consumers (CI, FE codegen) keep the
//! `my_family_openapi::ApiDoc` path stable.
//!
//! The actual `ApiDoc` definition lives in the api crate
//! (`crates/api/src/openapi_doc.rs`) so [`build_app`] can mount
//! `utoipa-swagger-ui` against the same `OpenApi` value without introducing a
//! circular `openapi → api → openapi` dependency.
//!
//! [`build_app`]: my_family_api::build_app

pub use my_family_api::ApiDoc;
