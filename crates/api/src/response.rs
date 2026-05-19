//! Success envelope for every API handler.
//!
//! The runtime type is `ApiResponse<T>`. utoipa 5 cannot derive `ToSchema` on
//! generics directly, so each endpoint also declares a named wrapper via the
//! `response_body!` macro for `OpenAPI` schema purposes.

use actix_web::body::BoxBody;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder};
use serde::Serialize;
use utoipa::ToSchema;

/// Runtime envelope. Wire shape: `{ "data": <T>, "meta": <ResponseMeta>? }`.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

/// Per-response metadata. Currently carries pagination cursors and the
/// `request_id` for tracing-friendly client diagnostics.
#[derive(Debug, Default, Serialize, ToSchema)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Cursor-based pagination block. `next_cursor` is `None` on the last page.
#[derive(Debug, Serialize, ToSchema)]
pub struct Pagination {
    pub next_cursor: Option<String>,
    pub limit: u32,
    pub returned: u32,
}

/// Declares a named envelope wrapper that utoipa CAN derive `ToSchema` for.
///
/// ```ignore
/// response_body!(pub HealthResponseBody, Health);
/// // then in #[utoipa::path(...)] write: body = HealthResponseBody
/// ```
#[macro_export]
macro_rules! response_body {
    ($vis:vis $name:ident, $inner:ty) => {
        #[derive(Debug, ::serde::Serialize, ::utoipa::ToSchema)]
        $vis struct $name {
            pub data: $inner,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub meta: Option<$crate::response::ResponseMeta>,
        }
    };
}

// Shared wrapper for DELETE handlers that return `{ "data": null }`.
response_body!(pub NullResponseBody, serde_json::Value);

impl<T: Serialize> ApiResponse<T> {
    pub const fn ok(data: T) -> Self {
        Self { data, meta: None }
    }

    #[must_use]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "ergonomic builder API: `impl Into<String>` accepts both &str and String"
    )]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        let mut meta = self.meta.take().unwrap_or_default();
        meta.request_id = Some(request_id.into());
        self.meta = Some(meta);
        self
    }
}

impl<T: Serialize> ApiResponse<Vec<T>> {
    pub const fn page(items: Vec<T>, pagination: Pagination) -> Self {
        Self {
            data: items,
            meta: Some(ResponseMeta { pagination: Some(pagination), request_id: None }),
        }
    }
}

impl<T: Serialize> Responder for ApiResponse<T> {
    type Body = BoxBody;
    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().insert_header((header::CONTENT_TYPE, "application/json")).json(self)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::test;

    use super::*;

    #[derive(Serialize, ToSchema)]
    struct Hello {
        msg: &'static str,
    }

    #[actix_web::test]
    async fn ok_envelope_serializes_with_only_data() {
        let resp = ApiResponse::ok(Hello { msg: "hi" });
        let req = test::TestRequest::default().to_http_request();
        let http = resp.respond_to(&req);
        assert_eq!(http.status(), 200);
        let body = to_bytes(http.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "data": { "msg": "hi" } }));
    }

    #[actix_web::test]
    async fn page_envelope_includes_pagination_meta() {
        let resp = ApiResponse::page(
            vec![Hello { msg: "a" }, Hello { msg: "b" }],
            Pagination { next_cursor: Some("abc".into()), limit: 50, returned: 2 },
        );
        let req = test::TestRequest::default().to_http_request();
        let http = resp.respond_to(&req);
        let body = to_bytes(http.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["data"].as_array().unwrap().len(), 2);
        assert_eq!(v["meta"]["pagination"]["next_cursor"], "abc");
        assert_eq!(v["meta"]["pagination"]["limit"], 50);
        assert_eq!(v["meta"]["pagination"]["returned"], 2);
    }

    #[actix_web::test]
    async fn request_id_appears_in_meta_when_set() {
        let resp = ApiResponse::ok(Hello { msg: "hi" }).with_request_id("req-123");
        let req = test::TestRequest::default().to_http_request();
        let http = resp.respond_to(&req);
        let body = to_bytes(http.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["meta"]["request_id"], "req-123");
    }
}
