mod http;
mod invitation;
mod jwt;
mod party;
mod store;
mod user;

use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use log::info;

use crate::{http::HTTPServer, store::MockStore};

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    dotenv()?;

    let server = HTTPServer::new(MockStore::debug_new());

    info!("Starting HTTP server on 0.0.0.0:4646");
    server.serve("0.0.0.0:4646").await
}
