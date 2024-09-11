use anyhow::Error;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
};
use tracing::info;

pub async fn proxy(
    mut in_rd: impl AsyncRead + Unpin,
    mut in_wr: impl AsyncWrite + Unpin,
    mut out_rd: impl AsyncRead + Unpin,
    mut out_wr: impl AsyncWrite + Unpin,
) -> Result<(), Error> {
    let mut buf_in = vec![0; 4096];
    let mut buf_out = vec![0; 4096];
    let mut total_in = 0;
    let mut total_out = 0;

    loop {
        select! {
            n = in_rd.read(&mut buf_in) => {
                let n = n?;
                total_in += n;
                if n == 0 {
                    out_wr.shutdown().await?;
                    info!("shutdown from in (in {}/out {})", total_in, total_out);
                    return Ok(());
                }
                out_wr.write_all(&buf_in[..n]).await?;
            },
            n = out_rd.read(&mut buf_out) => {
                let n = n?;
                total_out += n;
                if n == 0 {
                    in_wr.shutdown().await?;
                    info!("shutdown from out (in {}/out {})", total_in, total_out);
                    return Ok(());
                }
                in_wr.write_all(&buf_out[..n]).await?;
            },
        }
    }
}
