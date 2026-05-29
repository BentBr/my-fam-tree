//! `S3ObjectStore` — talks to any `S3`-compatible endpoint. `MinIO` in dev,
//! real `S3` in prod (just leave `endpoint_url` unset). The SDK handles
//! `AWS` `SigV4` + retries + connection pooling for us.

use std::time::Duration;

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{Region, SharedAsyncSleep};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ObjectCannedAcl;
use aws_sdk_s3::{Client, Config as SdkConfig};
use aws_smithy_async::rt::sleep::TokioSleep;
use bytes::Bytes;

use crate::{ObjectStore, StorageError};

/// Configuration for the `S3` backend.
///
/// Every field is required because we read it from env vars at startup —
/// failing loud at config time beats failing mid-upload.
#[derive(Debug, Clone)]
pub struct S3Config {
    /// Target bucket. Must exist (we don't auto-create).
    pub bucket: String,
    /// `AWS` region. For `MinIO` any value works (we recommend `us-east-1`).
    pub region: String,
    /// Override the endpoint URL (e.g. `http://minio:9000`). Leave `None`
    /// to hit real `AWS` `S3` at the default endpoint for the region.
    pub endpoint_url: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    /// `MinIO` needs path-style addressing (`http://host/bucket/key`);
    /// real `S3` uses virtual-host-style by default. Set `true` for `MinIO`.
    pub force_path_style: bool,
}

#[derive(Debug, Clone)]
pub struct S3ObjectStore {
    client: Client,
    bucket: String,
}

impl S3ObjectStore {
    /// Build the SDK client from `cfg`. Doesn't probe the bucket — that
    /// happens lazily on the first request; we want the worker / api to
    /// start cleanly even when `MinIO` is briefly unreachable on boot.
    #[must_use]
    pub fn new(cfg: S3Config) -> Self {
        let creds =
            Credentials::from_keys(cfg.access_key_id.clone(), cfg.secret_access_key.clone(), None);
        // The SDK uses `sleep_impl` for retries + timeouts. When we build
        // `SdkConfig` directly (vs `aws_config::load`), it isn't auto-wired
        // — without it the client panics at first use with "An async sleep
        // implementation is required for retry to work". `TokioSleep` is
        // a unit-struct constructor so we don't need a fallible helper.
        let mut builder = SdkConfig::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(Region::new(cfg.region.clone()))
            .sleep_impl(SharedAsyncSleep::new(TokioSleep::new()))
            // Configure a finite timeout so the SDK actually exercises the
            // sleep_impl above; without an explicit `TimeoutConfig` the
            // builder leaves them disabled and the retry runtime never
            // calls sleep at all.
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_attempt_timeout(std::time::Duration::from_secs(30))
                    .build(),
            )
            .force_path_style(cfg.force_path_style);
        if let Some(ep) = cfg.endpoint_url.as_ref() {
            builder = builder.endpoint_url(ep);
        }
        let client = Client::from_conf(builder.build());
        Self { client, bucket: cfg.bucket }
    }
}

fn backend<E: std::fmt::Display>(e: E) -> StorageError {
    StorageError::Backend(e.to_string())
}

#[async_trait]
impl ObjectStore for S3ObjectStore {
    async fn put(&self, key: &str, content_type: &str, bytes: Bytes) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            // ACL stays private by default; presigned URLs are how readers fetch.
            .acl(ObjectCannedAcl::Private)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, StorageError> {
        let resp =
            self.client.get_object().bucket(&self.bucket).key(key).send().await.map_err(|e| {
                // Distinguish "the object doesn't exist" from "transport
                // exploded" — handlers map the former to 404, the latter
                // to 500.
                if let Some(svc) = e.as_service_error()
                    && svc.is_no_such_key()
                {
                    StorageError::NotFound(key.to_string())
                } else {
                    backend(e)
                }
            })?;
        let body = resp.body.collect().await.map_err(backend)?;
        Ok(body.into_bytes())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client.delete_object().bucket(&self.bucket).key(key).send().await.map_err(backend)?;
        Ok(())
    }

    fn presigned_get(&self, key: &str, expires_in: Duration) -> Result<String, StorageError> {
        // The SDK's PresigningConfig builder requires the request to be
        // .presigned()'d which is async, but build only requires sync
        // config. We use the synchronous variant: PresigningConfig::expires_in
        // returns the config; the actual URL build is async in the SDK, so
        // for a truly sync API we'd need to block. Instead, expose this as
        // async-by-spawn: tokio::runtime::Handle::current().block_on(...).
        //
        // For our caller model the request handler is already in a tokio
        // runtime, so we can build via Handle::current() block_on the
        // ready future safely. PresigningConfig::builder().expires_in(...)
        // returns Result, not a future.
        let presign_cfg =
            PresigningConfig::builder().expires_in(expires_in).build().map_err(backend)?;
        let req = self.client.get_object().bucket(&self.bucket).key(key);
        let fut = req.presigned(presign_cfg);
        let presigned = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fut).map_err(backend)
        })?;
        Ok(presigned.uri().to_string())
    }
}
