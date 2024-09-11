mod buffer_parser;
// mod config;
mod tcp;
mod vless;

use std::io::Error;

pub use buffer_parser::*;
// pub use config::*;

use crate::buffer_parser::Protocol;
use tracing::info;

pub use vless::*;

pub async fn run_vless_over_tcp() -> Result<(), Error> {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:34434").await?;

    while let Ok((incoming, addr)) = tcp_listener.accept().await {
        info!("New connection from: {} -> ", addr);
        tokio::spawn(async move {
            let proto = VlessProtocol::new("test");
            Protocol::handle(&proto, incoming, addr)
                .await
                .unwrap_or_else(|e| info!("Error: {:?}", e));
        });
    }

    Ok(())
}
