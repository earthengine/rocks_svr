mod buffer_parser;
// mod config;
mod tcp;
mod vless;
mod websocket;
mod write_ext;

use anyhow::{anyhow, Error};
use std::future::ready;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use websocket::handle_stream_sink;

pub use buffer_parser::*;
use futures::{SinkExt, StreamExt};
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

pub async fn run_vless_over_tungstenite_ws() -> Result<(), Error> {
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:34080").await?;
    info!("started listening on {}", tcp_listener.local_addr()?);

    while let Ok((incoming, addr)) = tcp_listener.accept().await {
        info!("New connection from: {} -> ", addr);
        let cb = |req: &Request, resp: Response| {
            let p = req.uri().path();
            info!("{}", p);

            Ok(resp)
        };

        let ws_stream = tokio_tungstenite::accept_hdr_async(incoming, cb).await?;
        tokio::spawn(async move {
            let (sink, stream) = ws_stream.split();
            let stream = stream
                .filter(|msg| {
                    msg.as_ref()
                        .clone()
                        .map(|msg| ready(msg.is_binary()))
                        .unwrap_or(ready(true))
                })
                .map(|msg| {
                    msg.map(|msg| msg.into_data())
                        .map_err(|e| anyhow!("Error reading from ws: {:?}", e))
                });
            let sink = sink.with(|msg: Vec<u8>| {
                futures::future::ready(Ok(tokio_tungstenite::tungstenite::Message::Binary(msg)))
            });
            handle_stream_sink(stream, sink, addr)
                .await
                .unwrap_or_else(|e| info!("Error: {:?}", e));
        });
    }

    Ok(())
}
