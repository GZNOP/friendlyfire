mod gui;
mod http;
mod invitation;
mod jwt;
mod party;
mod store;
mod user;

use std::sync::Arc;

use dotenv::dotenv;
use eframe::NativeOptions;
use env_logger::Env;
use log::info;
use tokio::sync::RwLock;

use crate::{gui::DebugApp, http::HTTPServer, store::MockStore};

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

fn main() {
    init_logging();
    dotenv().unwrap();

    let store = Arc::new(RwLock::new(MockStore::default()));
    let server_store = store.clone();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .spawn(async move {
            info!("Starting HTTP server on 0.0.0.0:4646");
            HTTPServer::new(server_store)
                .serve("0.0.0.0:4646")
                .await
                .unwrap();
        });

    eframe::run_native(
        "FriendlyFire Debug",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(DebugApp::new(store.clone())))),
    )
    .unwrap();
}
