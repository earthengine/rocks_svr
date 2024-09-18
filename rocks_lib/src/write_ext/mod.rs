use tokio::io::AsyncWrite;

pub trait WriteExt {
    fn with(self, f: impl FnMut(&[u8]) -> Vec<u8> + Unpin) -> impl AsyncWrite;
}

#[pin_project::pin_project]
struct WithWrite<W, F> {
    #[pin]
    inner: W,
    f: F,
}

impl<W> WriteExt for W
where
    W: AsyncWrite,
{
    fn with(self, f: impl FnMut(&[u8]) -> Vec<u8> + Unpin) -> impl AsyncWrite {
        WithWrite { inner: self, f }
    }
}

impl<W, F> AsyncWrite for WithWrite<W, F>
where
    W: AsyncWrite,
    F: FnMut(&[u8]) -> Vec<u8> + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.project();
        this.inner.poll_write(cx, (this.f)(buf).as_slice())
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_flush(cx)
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}
