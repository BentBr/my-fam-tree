//! `GET /api/v1/health` — liveness probe + request-id sanity check.

use actix_web::{HttpMessage, get, web};
use serde::Serialize;
use utoipa::ToSchema;

use crate::middleware::RequestIdValue;
use crate::{ApiError, ApiResponse, response_body};

#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
}

response_body!(pub HealthResponseBody, Health);

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponseBody),
    ),
    tag = "health",
)]
// `actix_web::HttpRequest` holds an `Rc`, so the returned future is `!Send`;
// this is the canonical actix-web handler signature.
#[allow(clippy::future_not_send)]
// The `#[get("/health")]` proc-macro replaces this function with a `struct health`
// that implements `HttpServiceFactory`, which trips `unreachable_pub` on the fn.
// The `pub` is needed so the `openapi` crate can name it in `paths(...)`.
#[allow(unreachable_pub)]
#[get("/health")]
pub async fn health(req: actix_web::HttpRequest) -> Result<ApiResponse<Health>, ApiError> {
    let rid = req.extensions().get::<RequestIdValue>().map(|v| v.0.clone());
    let mut resp = ApiResponse::ok(Health { status: "ok", version: env!("CARGO_PKG_VERSION") });
    if let Some(rid) = rid {
        resp = resp.with_request_id(rid);
    }
    Ok(resp)
}

#[must_use]
pub fn scope() -> actix_web::Scope {
    web::scope("/api/v1").service(health)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::future_not_send,
    clippy::indexing_slicing
)]
mod tests {
    use actix_web::{App, test};

    use super::*;
    use crate::middleware::RequestId;

    #[actix_web::test]
    async fn health_returns_envelope_and_request_id_in_meta() {
        let app = test::init_service(App::new().wrap(RequestId).service(scope())).await;
        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .insert_header(("x-request-id", "rid-test"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        let body = test::read_body(res).await;
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["data"]["status"], "ok");
        assert!(v["data"]["version"].is_string());
        assert_eq!(v["meta"]["request_id"], "rid-test");
    }
}
