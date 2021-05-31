use crate::io::{AsyncRead, AsyncWrite, Interest, PollEvented, ReadBuf};

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::pin::Pin;
use std::task::{Context, Poll};

cfg_net_windows! {
    /// A structure representing a connected named pipe.
    ///
    /// This pipe can be connected with [NamedPipe::connect].
    ///
    /// To shut down the stream in the write direction, you can call the
    /// [`disconnect()`][NamedPipe::disconnect] method.
    pub struct NamedPipe {
        io: PollEvented<mio::windows::NamedPipe>,
    }
}

impl NamedPipe {
    /// Connects the named pipe to the given address.
    pub async fn connect<P>(addr: P) -> io::Result<Self>
    where
        P: AsRef<OsStr>,
    {
        let pipe = PollEvented::new(mio::windows::NamedPipe::new(addr)?)?;

        pipe.registration()
            .async_io(Interest::READABLE, || pipe.connect())
            .await?;

        Ok(Self { io: pipe })
    }
}

impl AsyncRead for NamedPipe {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_read_priv(cx, buf)
    }
}

impl AsyncWrite for NamedPipe {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_write_priv(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.poll_write_vectored_priv(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.io.disconnect()?;
        Poll::Ready(Ok(()))
    }
}

impl NamedPipe {
    // == Poll IO functions that takes `&self` ==
    //
    // They are not public because (taken from the doc of `PollEvented`):
    //
    // While `PollEvented` is `Sync` (if the underlying I/O type is `Sync`), the
    // caller must ensure that there are at most two tasks that use a
    // `PollEvented` instance concurrently. One for reading and one for writing.
    // While violating this requirement is "safe" from a Rust memory model point
    // of view, it will result in unexpected behavior in the form of lost
    // notifications and tasks hanging.

    pub(crate) fn poll_read_priv(
        &self,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // Safety: read functions correctly handles reads into uninitialized
        // memory
        unsafe { self.io.poll_read(cx, buf) }
    }

    pub(crate) fn poll_write_priv(
        &self,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.io.poll_write(cx, buf)
    }

    pub(super) fn poll_write_vectored_priv(
        &self,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.io.poll_write_vectored(cx, bufs)
    }
}

impl fmt::Debug for NamedPipe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.io.fmt(f)
    }
}

impl AsRawHandle for NamedPipe {
    fn as_raw_handle(&self) -> RawHandle {
        self.io.as_raw_handle()
    }
}
