//! Extractor configs that turn parse failures into RFC 7807 errors.
//!
//! Body / query / path parse failures go through [`ApiError`] so they get
//! the `application/problem+json` envelope like every other error, instead
//! of Actix's plain-text 400.

use actix_web::error::JsonPayloadError;
use actix_web::{HttpRequest, web};

use crate::error::{ApiError, RequestPart};

fn json_error(err: JsonPayloadError, _req: &HttpRequest) -> actix_web::Error {
    match err {
        // Size limits keep actix's own 413 — not a "malformed" request.
        JsonPayloadError::Overflow { .. } | JsonPayloadError::OverflowKnownLength { .. } => {
            err.into()
        }
        other => {
            ApiError::MalformedRequest { location: RequestPart::Body, detail: other.to_string() }
                .into()
        }
    }
}

/// `web::Json<T>` config (actix's default size limit is kept).
#[must_use]
pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default().error_handler(json_error)
}

/// `web::Query<T>` config.
#[must_use]
pub fn query_config() -> web::QueryConfig {
    web::QueryConfig::default().error_handler(|err, _req| {
        ApiError::MalformedRequest { location: RequestPart::Query, detail: err.to_string() }.into()
    })
}

/// `web::Path<T>` config.
#[must_use]
pub fn path_config() -> web::PathConfig {
    web::PathConfig::default().error_handler(|err, _req| {
        ApiError::MalformedRequest { location: RequestPart::Path, detail: err.to_string() }.into()
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::future_not_send
)]
mod tests {
    use actix_web::{App, HttpResponse, test};
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Input {
        n: u8,
    }

    async fn body_ok(b: web::Json<Input>) -> HttpResponse {
        HttpResponse::Ok().body(b.n.to_string())
    }
    async fn query_ok(q: web::Query<Input>) -> HttpResponse {
        HttpResponse::Ok().body(q.n.to_string())
    }
    async fn path_ok(p: web::Path<u8>) -> HttpResponse {
        HttpResponse::Ok().body(p.into_inner().to_string())
    }

    macro_rules! app {
        () => {
            test::init_service(
                App::new()
                    .app_data(json_config())
                    .app_data(query_config())
                    .app_data(path_config())
                    .route("/b", web::post().to(body_ok))
                    .route("/q", web::get().to(query_ok))
                    .route("/p/{n}", web::get().to(path_ok)),
            )
            .await
        };
    }

    async fn problem(
        app: &impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
        req: actix_http::Request,
    ) -> (u16, String, serde_json::Value) {
        let res = test::call_service(app, req).await;
        let status = res.status().as_u16();
        let ct = res
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_owned())
            .unwrap_or_default();
        let body = test::read_body(res).await;
        (status, ct, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
    }

    fn assert_malformed(status: u16, ct: &str, body: &serde_json::Value, part: &str) {
        assert_eq!(status, 400, "{body}");
        assert_eq!(ct, "application/problem+json");
        assert_eq!(body["code"], "request_malformed");
        assert_eq!(body["status"], 400);
        let detail = body["detail"].as_str().unwrap();
        assert!(detail.starts_with(&format!("malformed {part}")), "{detail}");
    }

    #[actix_web::test]
    async fn bad_json_body_is_problem_json() {
        let app = app!();
        let req = test::TestRequest::post()
            .uri("/b")
            .set_json(serde_json::json!({"n": 999}))
            .to_request();
        let (s, ct, b) = problem(&app, req).await;
        assert_malformed(s, &ct, &b, "request body");
    }

    #[actix_web::test]
    async fn non_json_body_is_problem_json() {
        let app = app!();
        let req = test::TestRequest::post()
            .uri("/b")
            .insert_header(("content-type", "application/json"))
            .set_payload("{not json")
            .to_request();
        let (s, ct, b) = problem(&app, req).await;
        assert_malformed(s, &ct, &b, "request body");
    }

    #[actix_web::test]
    async fn bad_query_is_problem_json() {
        let app = app!();
        let req = test::TestRequest::get().uri("/q?n=abc").to_request();
        let (s, ct, b) = problem(&app, req).await;
        assert_malformed(s, &ct, &b, "query string");
    }

    #[actix_web::test]
    async fn bad_path_is_problem_json() {
        let app = app!();
        let req = test::TestRequest::get().uri("/p/abc").to_request();
        let (s, ct, b) = problem(&app, req).await;
        assert_malformed(s, &ct, &b, "path parameter");
    }

    #[actix_web::test]
    async fn valid_input_still_passes() {
        let app = app!();
        let body_req =
            test::TestRequest::post().uri("/b").set_json(serde_json::json!({"n": 1})).to_request();
        assert_eq!(test::call_service(&app, body_req).await.status(), 200);
        let query_req = test::TestRequest::get().uri("/q?n=1").to_request();
        assert_eq!(test::call_service(&app, query_req).await.status(), 200);
        let path_req = test::TestRequest::get().uri("/p/1").to_request();
        assert_eq!(test::call_service(&app, path_req).await.status(), 200);
    }
}
