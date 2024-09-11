mod address;
mod request;
mod response;

use std::net::SocketAddr;

pub use address::*;
use anyhow::Error;
pub use request::*;
pub use response::*;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;
use uuid::Uuid;

use crate::{
    buffer_parser::Protocol, tcp::proxy, BufferParseResult, BufferParser, UnsendDataWrite,
};

#[derive(Debug, Error)]
pub enum VlessHeaderParseError {
    #[error("Invalid command")]
    InvalidCommand,
    #[error("Invalid version")]
    InvalidVersion,
    #[error("Addon is not supported")]
    AddonIsNotSupported,
    #[error("Invalid address")]
    InvalidAddress,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VlessProtocol {
    user_id: [u8; 16],
}

impl VlessProtocol {
    pub fn new(user: &str) -> Self {
        let mut user_id = [0u8; 16];
        let bs: &[u8] = user.as_bytes();
        let l = if bs.len() > 16usize {
            16usize
        } else {
            bs.len()
        };
        user_id[..l].copy_from_slice(&bs[..l]);
        Self { user_id }
    }
}

impl Protocol for VlessProtocol {
    async fn handle(
        &self,
        connection: impl AsyncRead + AsyncWrite + Send + Sync + Unpin,
        remote_addr: SocketAddr,
    ) -> Result<(), Error> {
        let mut buffer = [0u8; 1024];
        let mut offset = 0;
        let (mut in_rd, in_wr) = tokio::io::split(connection);
        let (header, len) = loop {
            match VlessRequestHeader::parse(&buffer[0..offset]) {
                BufferParseResult::Incomplete { needed } => {
                    let s = in_rd.read(&mut buffer[offset..]).await?;
                    info!("need {} read {} bytes", needed, s);
                    offset += s;
                }
                BufferParseResult::Error(e) => Err(e)?,
                BufferParseResult::Parsed { value, size } => break (value, size),
            }
        };
        info!(
            "user_id: {:?}, this user_id {:?}",
            header.user,
            Uuid::from_bytes(self.user_id)
        );
        let host = header.address.lookup_host().await?[0];
        info!("{} -> ({}){}", remote_addr, header.address, host);
        let stream = TcpStream::connect(&host).await?;
        let (out_rd, mut out_wr) = tokio::io::split(stream);
        out_wr.write(&buffer[len..offset]).await?;

        let in_wr = UnsendDataWrite::new(in_wr, Some(&[0; 2]));

        proxy(in_rd, in_wr, out_rd, out_wr).await?;
        Ok(())
    }
}
