mod gui;
mod http;
mod invitation;
mod party;
mod store;
mod user;

use std::sync::Arc;

use dotenv::dotenv;
use eframe::NativeOptions;
use env_logger::Env;
use log::info;

use crate::{
    gui::DebugApp,
    http::{AppState, HTTPServer},
};

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

// fn main() {
    init_logging();
    dotenv().unwrap();

    let state = Arc::new(AppState::default());
    let server_state = state.clone();
    let gui_state = state.clone();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.spawn(async move {
        info!("Starting HTTP server on 0.0.0.0:4646");
        HTTPServer::new(server_state)
            .serve("0.0.0.0:4646")
            .await
            .unwrap();
    });

    eframe::run_native(
        "FriendlyFire Debug",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(DebugApp::new(gui_state)))),
    )
    .unwrap();
}
