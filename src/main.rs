mod app;
mod media;
mod session;

use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tracing::info;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID must be set");

    let running = Arc::new(AtomicBool::new(true));
    let ctrl_c = Arc::clone(&running);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutting down");
        Arc::clone(&ctrl_c).store(false, Ordering::SeqCst);
    });

    // tracing_subscriber::fmt()
    //     .with_env_filter(EnvFilter::new(tracing::Level::DEBUG.to_string()))
    //     .init();

    tracing_subscriber::fmt().init();

    App::new(client_id).run(running).await
}
