use crate::io::util::read_line::finish_string_read;
use crate::io::util::read_to_end::read_to_end_internal;
use crate::io::util::vec_with_initialized::VecWithInitialized;
use crate::io::AsyncRead;

use pin_project::{pin_project, pinned_drop};
use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io, mem};

/// Future for the [`read_to_string`](super::AsyncReadExt::read_to_string) method.
#[pin_project(PinnedDrop)]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadToString<'a, R> where R: AsyncRead + Unpin + ?Sized {
    reader: &'a mut R,
    // This is the buffer we were provided. It will be replaced with an empty string
    // while reading to postpone utf-8 handling until after reading.
    output: &'a mut String,
    // The actual allocation of the string is moved into this vector instead.
    buf: VecWithInitialized<Vec<u8>>,
    // The number of bytes appended to buf. This can be less than buf.len() if
    // the buffer was not empty when the operation was started.
    read: usize,
    // Make this future `!Unpin` for compatibility with async trait methods.
    #[pin]
    _pin: PhantomPinned,
}

pub(crate) fn read_to_string<'a, R>(
    reader: &'a mut R,
    string: &'a mut String,
) -> ReadToString<'a, R>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let buf = mem::take(string).into_bytes();
    ReadToString {
        reader,
        buf: VecWithInitialized::new(buf),
        output: string,
        read: 0,
        _pin: PhantomPinned,
    }
}

fn read_to_string_internal<R: AsyncRead + ?Sized>(
    reader: Pin<&mut R>,
    output: &mut String,
    buf: &mut VecWithInitialized<Vec<u8>>,
    read: &mut usize,
    cx: &mut Context<'_>,
) -> Poll<io::Result<usize>> {
    let io_res = ready!(read_to_end_internal(buf, reader, read, cx));
    let utf8_res = String::from_utf8(buf.take());

    // At this point both buf and output are empty. The allocation is in utf8_res.

    debug_assert!(buf.is_empty());
    debug_assert!(output.is_empty());
    finish_string_read(io_res, utf8_res, *read, output, true)
}

impl<A> Future for ReadToString<'_, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();

        read_to_string_internal(Pin::new(*me.reader), me.output, me.buf, me.read, cx)
    }
}

#[pinned_drop]
impl<R> PinnedDrop for ReadToString<'_, R> where R: AsyncRead + Unpin + ?Sized {
    fn drop(self: Pin<&mut Self>) {
        let me = self.project();
        Pin::new(&mut **me.reader).cancel_pending_reads();
    }
}
