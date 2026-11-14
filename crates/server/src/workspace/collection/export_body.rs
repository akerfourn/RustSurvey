use axum::body::Bytes;
use http_body::{Body, Frame};
use std::{
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    sync::{OwnedSemaphorePermit, mpsc},
    task::JoinHandle,
};

#[cfg(test)]
mod tests;

pub(super) enum Message {
    Data(Vec<u8>),
    Finished,
}

/// Backpressure and cancellation bound the download independently of campaign size.
pub(super) struct ExportBody {
    receiver: mpsc::Receiver<Message>,
    pending: Bytes,
    finished: bool,
    task: JoinHandle<()>,
    _permit: Arc<OwnedSemaphorePermit>,
}

impl ExportBody {
    pub fn new(
        receiver: mpsc::Receiver<Message>,
        task: JoinHandle<()>,
        permit: Arc<OwnedSemaphorePermit>,
    ) -> Self {
        Self {
            receiver,
            pending: Bytes::new(),
            finished: false,
            task,
            _permit: permit,
        }
    }
}

impl Drop for ExportBody {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Body for ExportBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        if self.finished {
            return Poll::Ready(None);
        }
        if self.pending.is_empty() {
            match self.receiver.poll_recv(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Message::Data(bytes))) => self.pending = bytes.into(),
                Poll::Ready(Some(Message::Finished)) => {
                    self.finished = true;
                    return Poll::Ready(None);
                }
                Poll::Ready(None) => {
                    self.finished = true;
                    return Poll::Ready(Some(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "response export interrupted",
                    ))));
                }
            }
        }
        let length = self.pending.len().min(64 * 1024);
        Poll::Ready(Some(Ok(Frame::data(self.pending.split_to(length)))))
    }
}
