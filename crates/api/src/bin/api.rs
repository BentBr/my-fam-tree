//! Production `api` binary: loads config, initializes tracing, serves `build_app`.

use actix_web::HttpServer;
use anyhow::Context;
use my_family_api::{AppEnv, Config, build_app, init_tracing};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        // `.env` is optional in development; ignore both missing-file and parse
        // errors here because tracing isn't initialized yet and the binary
        // lints forbid `eprintln!`/`println!`. Any real misconfiguration will
        // surface from `Config::load_from_env` below.
        let _ = dotenvy::dotenv();
    }

    let cfg = Config::load_from_env().context("load config from environment")?;

    init_tracing(cfg.log_format, &cfg.rust_log);

    tracing::info!(
        app_env = ?cfg.app_env,
        host = %cfg.api_host,
        port = cfg.api_port,
        "starting my-family api",
    );

    let bind = format!("{}:{}", cfg.api_host, cfg.api_port);
    let cfg_for_factory = cfg.clone();

    HttpServer::new(move || build_app(&cfg_for_factory)).bind(&bind)?.run().await?;

    if matches!(cfg.app_env, AppEnv::Production) {
        tracing::info!("api shutdown clean");
    }
    Ok(())
}
