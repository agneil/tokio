use crate::io::AsyncBufRead;

use pin_project::{pin_project, pinned_drop};
use std::future::Future;
use std::io;
use std::marker::PhantomPinned;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Future for the [`read_until`](crate::io::AsyncBufReadExt::read_until) method.
/// The delimeter is included in the resulting vector.
#[pin_project(PinnedDrop)]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadUntil<'a, R> where R: AsyncBufRead + Unpin + ?Sized {
    reader: &'a mut R,
    delimeter: u8,
    buf: &'a mut Vec<u8>,
    // The number of bytes appended to buf. This can be less than buf.len() if
    // the buffer was not empty when the operation was started.
    read: usize,
    // Make this future `!Unpin` for compatibility with async trait methods.
    #[pin]
    _pin: PhantomPinned,
}

pub(crate) fn read_until<'a, R>(
    reader: &'a mut R,
    delimeter: u8,
    buf: &'a mut Vec<u8>,
) -> ReadUntil<'a, R>
where
    R: AsyncBufRead + Unpin + ?Sized,
{
    ReadUntil {
        reader,
        delimeter,
        buf,
        read: 0,
        _pin: PhantomPinned,
    }
}

pub(super) fn read_until_internal<R: AsyncBufRead + ?Sized>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    delimeter: u8,
    buf: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    loop {
        let (done, used) = {
            let available = ready!(reader.as_mut().poll_fill_buf(cx))?;
            if let Some(i) = memchr::memchr(delimeter, available) {
                buf.extend_from_slice(&available[..=i]);
                (true, i + 1)
            } else {
                buf.extend_from_slice(available);
                (false, available.len())
            }
        };
        reader.as_mut().consume(used);
        *read += used;
        if done || used == 0 {
            return Poll::Ready(Ok(mem::replace(read, 0)));
        }
    }
}

impl<R> Future for ReadUntil<'_, R> where R: AsyncBufRead + Unpin + ?Sized {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        read_until_internal(Pin::new(*me.reader), cx, *me.delimeter, me.buf, me.read)
    }
}

#[pinned_drop]
impl<R> PinnedDrop for ReadUntil<'_, R> where R: AsyncBufRead + Unpin + ?Sized {
    fn drop(self: Pin<&mut Self>) {
        let me = self.project();
        Pin::new(&mut **me.reader).cancel_pending_reads();
    }
}
