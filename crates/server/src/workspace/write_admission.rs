use std::time::Duration;
use tokio::sync::{Semaphore, SemaphorePermit, TryAcquireError};

const MAX_WAIT: Duration = Duration::from_millis(250);

pub(super) enum AdmissionError {
    Busy,
    Closed,
}

pub(super) async fn acquire<'a>(
    active: &'a Semaphore,
    waiting: &Semaphore,
) -> Result<SemaphorePermit<'a>, AdmissionError> {
    match active.try_acquire() {
        Ok(permit) => return Ok(permit),
        Err(TryAcquireError::Closed) => return Err(AdmissionError::Closed),
        Err(TryAcquireError::NoPermits) => {}
    }
    let queued = waiting.try_acquire().map_err(|error| match error {
        TryAcquireError::Closed => AdmissionError::Closed,
        TryAcquireError::NoPermits => AdmissionError::Busy,
    })?;
    let permit = tokio::time::timeout(MAX_WAIT, active.acquire())
        .await
        .map_err(|_| AdmissionError::Busy)?
        .map_err(|_| AdmissionError::Closed)?;
    drop(queued);
    Ok(permit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::{Future, poll_fn},
        pin::Pin,
        task::Poll,
    };

    async fn assert_waiting<F: Future>(mut future: Pin<&mut F>) {
        poll_fn(|context| {
            assert!(future.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn test_write_admission_should_admit_a_waiter_when_an_active_slot_is_released() {
        let active = Semaphore::new(1);
        let waiting = Semaphore::new(2);
        let held = active.try_acquire().unwrap();
        let mut queued = Box::pin(acquire(&active, &waiting));
        assert_waiting(queued.as_mut()).await;
        assert_eq!(waiting.available_permits(), 1);
        drop(held);
        let admitted = queued
            .await
            .ok()
            .expect("the queued request should be admitted");
        assert_eq!(waiting.available_permits(), 2);
        assert_eq!(active.available_permits(), 0);
        drop(admitted);
        assert_eq!(active.available_permits(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_write_admission_should_bound_waiters_and_release_a_cancelled_request() {
        let active = Semaphore::new(1);
        let waiting = Semaphore::new(1);
        let held = active.try_acquire().unwrap();
        let mut queued = Box::pin(acquire(&active, &waiting));
        assert_waiting(queued.as_mut()).await;
        assert!(matches!(
            acquire(&active, &waiting).await,
            Err(AdmissionError::Busy)
        ));
        drop(queued);
        assert_eq!(waiting.available_permits(), 1);
        drop(held);
        assert!(acquire(&active, &waiting).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn test_write_admission_should_expire_without_leaking_slots_or_waiters() {
        let active = Semaphore::new(1);
        let waiting = Semaphore::new(1);
        let _held = active.try_acquire().unwrap();
        let mut queued = Box::pin(acquire(&active, &waiting));
        assert_waiting(queued.as_mut()).await;
        tokio::time::advance(MAX_WAIT).await;
        assert!(matches!(queued.await, Err(AdmissionError::Busy)));
        assert_eq!(waiting.available_permits(), 1);
        assert_eq!(active.available_permits(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn test_write_admission_should_keep_immediate_rejection_when_the_queue_is_disabled() {
        let active = Semaphore::new(1);
        let waiting = Semaphore::new(0);
        let _held = active.try_acquire().unwrap();
        assert!(matches!(
            acquire(&active, &waiting).await,
            Err(AdmissionError::Busy)
        ));
    }
}
