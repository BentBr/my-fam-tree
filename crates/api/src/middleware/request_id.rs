//! `X-Request-Id` middleware.
//!
//! Generates a UUID per request if the header is missing, preserves it if the
//! client supplied one. The value is mirrored into the response headers and
//! recorded on the current tracing span so `TracingLogger` includes it in span
//! fields.

use std::rc::Rc;
use std::sync::LazyLock;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{Error, HttpMessage};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use uuid::Uuid;

/// Header name `x-request-id`. `HeaderName::from_static` is not const-fn in
/// `http` 1.x, so we use `LazyLock` to initialize once.
pub static HEADER: LazyLock<HeaderName> = LazyLock::new(|| HeaderName::from_static("x-request-id"));

#[derive(Debug, Clone, Copy, Default)]
pub struct RequestId;

#[derive(Debug)]
pub struct RequestIdMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware { service: Rc::new(service) }))
    }
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = Rc::clone(&self.service);
        Box::pin(async move {
            let incoming =
                req.headers().get(&*HEADER).and_then(|v| v.to_str().ok()).map(str::to_string);
            let id = incoming.unwrap_or_else(|| Uuid::new_v4().to_string());
            tracing::Span::current().record("request_id", tracing::field::display(&id));
            req.extensions_mut().insert(RequestIdValue(id.clone()));
            let mut res = svc.call(req).await?;
            if let Ok(val) = HeaderValue::from_str(&id) {
                res.headers_mut().insert(HEADER.clone(), val);
            }
            Ok(res)
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequestIdValue(pub String);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::future_not_send)]
mod tests {
    use actix_web::{App, HttpMessage, HttpRequest, HttpResponse, test, web};

    use super::*;

    async fn echo(req: HttpRequest) -> HttpResponse {
        let id = req.extensions().get::<RequestIdValue>().map(|v| v.0.clone()).unwrap_or_default();
        HttpResponse::Ok().body(id)
    }

    #[actix_web::test]
    async fn generates_id_when_missing() {
        let app =
            test::init_service(App::new().wrap(RequestId).route("/", web::get().to(echo))).await;
        let req = test::TestRequest::default().to_request();
        let res = test::call_service(&app, req).await;
        let header = res.headers().get(&*HEADER).unwrap().to_str().unwrap().to_string();
        assert!(!header.is_empty());
        let body = test::read_body(res).await;
        assert_eq!(std::str::from_utf8(&body).unwrap(), header);
    }

    #[actix_web::test]
    async fn preserves_incoming_id() {
        let app =
            test::init_service(App::new().wrap(RequestId).route("/", web::get().to(echo))).await;
        let req =
            test::TestRequest::default().insert_header((HEADER.clone(), "rid-fixed")).to_request();
        let res = test::call_service(&app, req).await;
        let header = res.headers().get(&*HEADER).unwrap().to_str().unwrap();
        assert_eq!(header, "rid-fixed");
    }
}
