use axum::body::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::OwnedSemaphorePermit;

/// Retain the read slot until the response ends or the client disconnects.
pub(super) struct ObjectBody {
    remaining: Bytes,
    _permit: OwnedSemaphorePermit,
}
impl ObjectBody {
    pub(super) fn new(bytes: Vec<u8>, permit: OwnedSemaphorePermit) -> Self {
        Self {
            remaining: bytes.into(),
            _permit: permit,
        }
    }
}
impl Body for ObjectBody {
    type Data = Bytes;
    type Error = Infallible;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.remaining.is_empty() {
            return Poll::Ready(None);
        }
        let size = self.remaining.len().min(64 * 1024);
        Poll::Ready(Some(Ok(Frame::data(self.remaining.split_to(size)))))
    }
    fn is_end_stream(&self) -> bool {
        self.remaining.is_empty()
    }
    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.remaining.len() as u64)
    }
}
