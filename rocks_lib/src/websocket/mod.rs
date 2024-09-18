use std::{future::ready, net::SocketAddr};

use anyhow::{anyhow, Error};
use futures::{Sink, SinkExt, Stream, StreamExt};
use hex_display::HexDisplayExt;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    select,
};
use tracing::info;

use crate::{BufferParseResult, BufferParser, VlessRequestHeader};

pub async fn handle_stream_sink(
    mut in_rd: impl Stream<Item = Result<Vec<u8>, Error>> + Send + Sync + Unpin,
    in_wr: impl Sink<Vec<u8>, Error = Error> + Send + Sync + Unpin,
    remote_addr: SocketAddr,
) -> Result<(), anyhow::Error> {
    let mut data = in_rd
        .next()
        .await
        .unwrap_or_else(|| Err(anyhow!("Unexpected disconnection")))?;
    let (header, s) = loop {
        match VlessRequestHeader::parse(&data) {
            BufferParseResult::Incomplete { .. } => {
                let mut more = in_rd
                    .next()
                    .await
                    .unwrap_or_else(|| Err(anyhow!("Unexpected disconnection")))?;
                data.append(&mut more);
                continue;
            }
            BufferParseResult::Parsed { value, size } => {
                break (value, size);
            }
            BufferParseResult::Error(e) => {
                info!("Error parsing buffer: {:?}", e);
                return Err(anyhow::Error::msg("Error parsing buffer"));
            }
        }
    };

    info!("user_id: {:?}", header.user);

    let host = header.address.lookup_host().await?[0];
    info!("{} -> ({}){}", remote_addr, header.address, host);
    let stream = TcpStream::connect(&host).await?;
    let (out_rd, mut out_wr) = tokio::io::split(stream);
    out_wr.write(&data[s..]).await?;
    let mut first = true;
    let in_wr = in_wr.with(|msg: Vec<u8>| {
        if first {
            info!("first message: {}", msg.hex());
            first = false;
            let mut msg_to_send = vec![0u8; 2];
            msg_to_send.extend_from_slice(&msg);
            return ready(Ok(msg_to_send));
        }
        ready(Ok(msg))
    });

    proxy_sink_stream(in_rd, in_wr, out_rd, out_wr).await
}

async fn proxy_sink_stream(
    mut in_rd: impl Stream<Item = Result<Vec<u8>, Error>> + Send + Sync + Unpin,
    mut in_wr: impl Sink<Vec<u8>, Error = Error> + Send + Sync + Unpin,
    mut out_rd: impl AsyncRead + Send + Sync + Unpin,
    mut out_wr: impl AsyncWrite + Send + Sync + Unpin,
) -> Result<(), Error> {
    let mut total_in = 0;
    let mut total_out = 0;

    loop {
        let buf_out_read = &mut [0; 1024];
        select! {
            msg = in_rd.next() => {
                let msg = match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        info!("Error reading from in: {:?}", e);
                        break;
                    }
                    None => {
                        info!("in stream ended");
                        break;
                    }
                };
                total_in += msg.len();
                out_wr.write_all(&msg).await.unwrap();
            }
            msg = out_rd.read(buf_out_read) => {
                let n = match msg {
                    Ok(n) => n,
                    Err(e) => {
                        info!("Error reading from out: {:?}", e);
                        break;
                    }
                };
                if n == 0 {
                    info!("out stream ended");
                    in_wr.send(vec![]).await?;
                    break;
                }
                total_out += n;
                in_wr.send(buf_out_read[..n].to_vec()).await?;
            }
        }
    }

    info!("shutdown from in (in {}/out {})", total_in, total_out);
    Ok(())
}
