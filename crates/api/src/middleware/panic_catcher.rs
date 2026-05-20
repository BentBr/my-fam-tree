//! Panic-catching middleware.
//!
//! Catches handler panics and converts them into sanitized internal-error
//! responses so the `application/problem+json` contract still holds even when
//! a handler explodes.

use std::panic::AssertUnwindSafe;
use std::rc::Rc;

use actix_web::Error;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use futures_util::future::{FutureExt, LocalBoxFuture, Ready, ready};

use crate::ApiError;

#[derive(Debug, Clone, Copy, Default)]
pub struct PanicCatcher;

#[derive(Debug)]
pub struct PanicCatcherMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for PanicCatcher
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = PanicCatcherMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PanicCatcherMiddleware { service: Rc::new(service) }))
    }
}

impl<S, B> Service<ServiceRequest> for PanicCatcherMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = Rc::clone(&self.service);
        Box::pin(async move {
            let fut = AssertUnwindSafe(svc.call(req)).catch_unwind();
            match fut.await {
                Ok(Ok(resp)) => Ok(resp.map_into_boxed_body()),
                Ok(Err(e)) => Err(e),
                Err(panic) => {
                    let detail = panic_to_string(&panic);
                    let err = ApiError::Internal(anyhow::anyhow!("panic: {detail}"));
                    Err(actix_web::Error::from(err))
                }
            }
        })
    }
}

// The borrowed `Box` signature is intentional: `catch_unwind` yields
// `Box<dyn Any + Send>` and we want to inspect it without consuming so the
// caller keeps ownership for further diagnostics if needed.
#[allow(clippy::borrowed_box)]
fn panic_to_string(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = panic.downcast_ref::<String>() {
        return s.clone();
    }
    "<non-string panic payload>".to_string()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::future_not_send,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use actix_web::{App, HttpResponse, test, web};

    use super::*;

    async fn boom() -> HttpResponse {
        panic!("deliberate test panic");
    }

    #[actix_web::test]
    async fn panic_becomes_internal_error_response() {
        let app =
            test::init_service(App::new().wrap(PanicCatcher).route("/boom", web::get().to(boom)))
                .await;
        let req = test::TestRequest::get().uri("/boom").to_request();
        // The middleware turns the panic into an `actix_web::Error`. In a live
        // server the framework converts that into the same problem+json body
        // that `ApiError::Internal` produces via `ResponseError`. We assert on
        // that conversion directly to keep the test free of HTTP plumbing.
        let err = test::try_call_service(&app, req).await.expect_err("panic must surface as Err");
        let resp = err.error_response();
        assert_eq!(resp.status(), 500);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.starts_with("application/problem+json"));
        let body = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["code"], "internal");
        assert!(!v["detail"].as_str().unwrap().contains("deliberate"), "panic detail leaked");
    }
}
