use super::*;
use http_body_util::BodyExt;
use tokio::sync::{Semaphore, oneshot};

#[tokio::test]
async fn test_export_body_should_report_unexpected_producer_termination_as_a_read_error() {
    let semaphore = Arc::new(Semaphore::new(1));
    let permit = Arc::new(semaphore.clone().acquire_owned().await.unwrap());
    let (sender, receiver) = mpsc::channel(1);
    let task = tokio::spawn(async move {
        sender.send(Message::Data(vec![1; 65_537])).await.unwrap();
    });
    let body = ExportBody::new(receiver, task, permit);
    assert!(
        body.collect().await.is_err(),
        "a missing completion marker must fail the download"
    );
    assert_eq!(semaphore.available_permits(), 1);
}

#[tokio::test]
async fn test_export_body_should_cancel_the_producer_and_release_its_slot_after_disconnect() {
    struct Signal(Option<oneshot::Sender<()>>);
    impl Drop for Signal {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }
    let semaphore = Arc::new(Semaphore::new(1));
    let permit = Arc::new(semaphore.clone().acquire_owned().await.unwrap());
    let (sender, receiver) = mpsc::channel(1);
    let (started, ready) = oneshot::channel();
    let (stopped, ended) = oneshot::channel();
    let guard = permit.clone();
    let task = tokio::spawn(async move {
        let _guard = guard;
        let _signal = Signal(Some(stopped));
        started.send(()).unwrap();
        sender.send(Message::Data(vec![1])).await.unwrap();
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    let body = ExportBody::new(receiver, task, permit);
    assert_eq!(semaphore.available_permits(), 0);
    drop(body);
    tokio::time::timeout(std::time::Duration::from_secs(1), ended)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(semaphore.available_permits(), 1);
}
