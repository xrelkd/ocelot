use std::{os::unix::io::OwnedFd, time::Duration};

use nix::{
    fcntl::OFlag,
    unistd::{self, pipe2},
};
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::splice_relay::{Builder, Destination, Error, RelayRegistration, SpliceRelay};

// Helper to create a non-blocking pipe pair
fn create_pipe() -> (OwnedFd, OwnedFd) {
    let (r, w) = pipe2(OFlag::O_NONBLOCK).expect("pipe2 failed");
    (r, w)
}

// Helper to create a pipe with some data written to the writer end
fn create_data_pipe(data: &[u8]) -> (OwnedFd, OwnedFd) {
    let (r, w) = create_pipe();
    let _ = unistd::write(&w, data);
    (r, w)
}

// Test harness that starts the executor in a background task
struct TestExecutor {
    relay: SpliceRelay,
    cancel_token: CancellationToken,
    serve_task: tokio::task::JoinHandle<Result<(), Error>>,
}

impl TestExecutor {
    async fn new() -> Self {
        let cancel_token = CancellationToken::new();
        let (relay, executor) = Builder::new().build().expect("build failed");

        let cancel_clone = cancel_token.clone();
        let serve_task = tokio::spawn(async move { executor.serve(cancel_clone).await });

        // Give the worker thread time to initialize and register eventfd with epoll
        time::sleep(Duration::from_millis(10)).await;

        Self { relay, cancel_token, serve_task }
    }

    async fn shutdown(self) {
        self.cancel_token.cancel();
        drop(self.serve_task.await);
    }

    fn relay(&self) -> &SpliceRelay { &self.relay }
}

// ============== SpliceRelay API Tests ==============

#[tokio::test]
async fn test_register_success() {
    let test = TestExecutor::new().await;

    let (src, dst) = create_pipe();
    let RelayRegistration { id, .. } = test
        .relay()
        .register(src, Destination::OwnedFd { fd: dst })
        .await
        .expect("register should succeed");

    assert!(id > 0, "Registered ID should be positive");

    let list = test.relay().list().await;
    assert_eq!(list.len(), 1, "Should have one relay");
    assert_eq!(list[0].id, id);

    test.shutdown().await;
}

#[tokio::test]
async fn test_register_multiple() {
    let test = TestExecutor::new().await;

    let mut ids = Vec::new();
    for _ in 0..3 {
        let (src, dst) = create_pipe();
        let RelayRegistration { id, .. } = test
            .relay()
            .register(src, Destination::OwnedFd { fd: dst })
            .await
            .expect("register should succeed");
        ids.push(id);
    }

    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0] + 1, ids[1]);
    assert_eq!(ids[1] + 1, ids[2]);

    let list = test.relay().list().await;
    assert_eq!(list.len(), 3);

    test.shutdown().await;
}

#[tokio::test]
async fn test_remove_relay() {
    let test = TestExecutor::new().await;

    let (src, dst) = create_pipe();
    let RelayRegistration { id, .. } = test
        .relay()
        .register(src, Destination::OwnedFd { fd: dst })
        .await
        .expect("register should succeed");

    test.relay().remove(id);

    let list = test.relay().list().await;
    assert_eq!(list.len(), 0, "Relay should be removed from list");

    test.shutdown().await;
}

#[tokio::test]
async fn test_remove_nonexistent() {
    let test = TestExecutor::new().await;

    test.relay().remove(99999);

    let status = test.relay().get_status().await.expect("get_status should succeed");
    assert_eq!(status.active_relays, 0);

    test.shutdown().await;
}

#[tokio::test]
async fn test_get_status_empty() {
    let test = TestExecutor::new().await;

    let status = test.relay().get_status().await.expect("get_status should succeed");

    assert_eq!(status.active_relays, 0);
    assert_eq!(status.total_added, 0);
    assert_eq!(status.total_removed, 0);
    assert_eq!(status.bytes_transferred, 0);

    test.shutdown().await;
}

#[tokio::test]
async fn test_get_status_after_operations() {
    let test = TestExecutor::new().await;

    let (src1, dst1) = create_pipe();
    let (src2, dst2) = create_pipe();

    let registration1 = test
        .relay()
        .register(src1, Destination::OwnedFd { fd: dst1 })
        .await
        .expect("register 1 failed");
    let id1 = registration1.id;
    let _id2 = test
        .relay()
        .register(src2, Destination::OwnedFd { fd: dst2 })
        .await
        .expect("register 2 failed");

    let status1 = test.relay().get_status().await.expect("get_status failed");
    assert_eq!(status1.active_relays, 2);
    assert_eq!(status1.total_added, 2);
    assert_eq!(status1.total_removed, 0);

    test.relay().remove(id1);

    let status2 = test.relay().get_status().await.expect("get_status failed after remove");
    assert_eq!(status2.active_relays, 1);
    assert_eq!(status2.total_added, 2);
    assert_eq!(status2.total_removed, 1);

    test.shutdown().await;
}

#[tokio::test]
async fn test_list_relays_empty() {
    let test = TestExecutor::new().await;

    let list = test.relay().list().await;
    assert_eq!(list.len(), 0);

    test.shutdown().await;
}

#[tokio::test]
async fn test_list_relays_with_entries() {
    let test = TestExecutor::new().await;

    let (src1, dst1) = create_pipe();
    let (src2, dst2) = create_pipe();
    let (src3, dst3) = create_pipe();

    let registration1 = test
        .relay()
        .register(src1, Destination::OwnedFd { fd: dst1 })
        .await
        .expect("register 1 failed");
    let id1 = registration1.id;
    let registration2 = test
        .relay()
        .register(src2, Destination::OwnedFd { fd: dst2 })
        .await
        .expect("register 2 failed");
    let id2 = registration2.id;
    let _id3 = test
        .relay()
        .register(src3, Destination::OwnedFd { fd: dst3 })
        .await
        .expect("register 3 failed");

    let list = test.relay().list().await;
    assert_eq!(list.len(), 3);

    let ids: Vec<u64> = list.iter().map(|info| info.id).collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));

    test.shutdown().await;
}

#[tokio::test]
async fn test_register_after_shutdown() {
    let test = TestExecutor::new().await;

    // Cancel token to trigger shutdown
    test.cancel_token.cancel();
    // Give executor time to shut down and close the channel
    time::sleep(Duration::from_millis(50)).await;

    let (src, dst) = create_pipe();
    let result = test.relay().register(src, Destination::OwnedFd { fd: dst }).await;
    assert!(result.is_none(), "Register after shutdown should return None");

    // Clean up
    test.shutdown().await;
}

// ============== Error Condition Tests ==============

#[tokio::test]
async fn test_get_status_send_error_after_shutdown() {
    let test = TestExecutor::new().await;

    test.cancel_token.cancel();
    // Give executor time to shut down and close the channel
    time::sleep(Duration::from_millis(50)).await;

    let status = test.relay().get_status().await;
    assert!(status.is_none(), "get_status after shutdown should return None");

    test.shutdown().await;
}

#[tokio::test]
async fn test_list_send_error_after_shutdown() {
    let test = TestExecutor::new().await;

    test.cancel_token.cancel();

    let list = test.relay().list().await;
    assert_eq!(list.len(), 0, "list after shutdown should return empty vec");

    test.shutdown().await;
}

// ============== Concurrency Tests ==============

#[tokio::test]
async fn test_concurrent_registers() {
    let test = TestExecutor::new().await;

    let mut handles = Vec::new();
    for _ in 0..10 {
        let relay_clone = test.relay().clone();
        let handle = tokio::spawn(async move {
            let (src, dst) = create_pipe();
            let RelayRegistration { id, .. } = relay_clone
                .register(src, Destination::OwnedFd { fd: dst })
                .await
                .expect("register should succeed");
            id
        });
        handles.push(handle);
    }

    let mut ids = Vec::new();
    for h in handles {
        let id = h.await.expect("task should complete");
        ids.push(id);
    }

    let unique_ids: std::collections::HashSet<u64> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique_ids.len(), "All registered IDs should be unique");

    assert_eq!(ids[0], 1);
    assert_eq!(ids[9], 10);

    let list = test.relay().list().await;
    assert_eq!(list.len(), 10);

    test.shutdown().await;
}

#[tokio::test]
async fn test_concurrent_register_and_remove() {
    let test = TestExecutor::new().await;

    let mut handles = Vec::new();
    for _ in 0..5 {
        let relay_clone = test.relay().clone();
        let h = tokio::spawn(async move {
            let (src, dst) = create_pipe();
            let registration =
                relay_clone.register(src, Destination::OwnedFd { fd: dst }).await.unwrap();
            let id = registration.id;
            time::sleep(Duration::from_millis(10)).await;
            relay_clone.remove(id);
            id
        });
        handles.push(h);
    }

    for h in handles {
        let _ = h.await.expect("task should complete");
    }

    let list = test.relay().list().await;
    assert_eq!(list.len(), 0, "All relays should be removed");

    let status = test.relay().get_status().await.unwrap();
    assert_eq!(status.active_relays, 0);

    test.shutdown().await;
}

#[tokio::test]
async fn test_list_during_concurrent_modifications() {
    let test = TestExecutor::new().await;

    let mut register_handles = Vec::new();
    for _ in 0..5 {
        let relay_clone = test.relay().clone();
        let h = tokio::spawn(async move {
            let (src, dst) = create_pipe();
            relay_clone.register(src, Destination::OwnedFd { fd: dst }).await.unwrap()
        });
        register_handles.push(h);
    }

    let relay_clone = test.relay().clone();
    let list_handle = tokio::spawn(async move {
        let mut counts = Vec::new();
        for _ in 0..3 {
            let list = relay_clone.list().await;
            counts.push(list.len() as u64);
            time::sleep(Duration::from_millis(5)).await;
        }
        counts
    });

    for h in register_handles {
        drop(h.await.expect("register task failed"));
    }

    let list_counts: Vec<u64> = list_handle.await.expect("list task failed");

    assert!(list_counts[0] <= list_counts[1]);
    assert!(list_counts[1] <= list_counts[2]);
    assert_eq!(list_counts[2], 5);

    test.shutdown().await;
}

// ============== Status Tracking Tests ==============

#[tokio::test]
async fn test_status_tracks_add_remove() {
    let test = TestExecutor::new().await;

    let mut ids = Vec::new();
    for _ in 0..3 {
        let (src, dst) = create_pipe();
        let registration =
            test.relay().register(src, Destination::OwnedFd { fd: dst }).await.unwrap();
        let id = registration.id;
        ids.push(id);
    }

    let mut status = test.relay().get_status().await.unwrap();
    assert_eq!(status.active_relays, 3);
    assert_eq!(status.total_added, 3);
    assert_eq!(status.total_removed, 0);

    test.relay().remove(ids[0]);
    time::sleep(Duration::from_millis(10)).await;
    status = test.relay().get_status().await.unwrap();
    assert_eq!(status.active_relays, 2);
    assert_eq!(status.total_added, 3);
    assert_eq!(status.total_removed, 1);

    test.relay().remove(ids[1]);
    time::sleep(Duration::from_millis(10)).await;
    status = test.relay().get_status().await.unwrap();
    assert_eq!(status.active_relays, 1);
    assert_eq!(status.total_removed, 2);

    test.shutdown().await;
}

#[tokio::test]
async fn test_bytes_transferred_increments() {
    let test = TestExecutor::new().await;

    let test_data = b"Hello, SpliceRelay! This is test data.";
    let (src_r, _src_w) = create_data_pipe(test_data);
    let (dst_r, dst_w) = create_pipe();

    let registration =
        test.relay().register(src_r, Destination::OwnedFd { fd: dst_w }).await.unwrap();
    let id = registration.id;

    time::sleep(Duration::from_millis(50)).await;

    let mut buf = vec![0u8; test_data.len()];
    let n = unistd::read(&dst_r, &mut buf).expect("read from dst pipe failed");

    assert_eq!(n, test_data.len(), "Should have spliced all data");
    assert_eq!(&buf, test_data, "Spliced data should match original");

    let status = test.relay().get_status().await.unwrap();
    assert_eq!(status.bytes_transferred, test_data.len() as u64);

    test.relay().remove(id);
    test.shutdown().await;
}

#[tokio::test]
async fn test_bytes_transferred_multiple_splices() {
    let test = TestExecutor::new().await;

    let mut relays = Vec::new();
    let data1 = b"Test data 1";
    let data2 = b"Test data 2 is longer";
    let data3 = b"X";

    let (src1_r, _src1_w) = create_data_pipe(data1);
    let (dst1_r, dst1_w) = create_pipe();
    let registration1 =
        test.relay().register(src1_r, Destination::OwnedFd { fd: dst1_w }).await.unwrap();
    let id1 = registration1.id;
    relays.push((id1, dst1_r, data1.len()));

    let (src2_r, _src2_w) = create_data_pipe(data2);
    let (dst2_r, dst2_w) = create_pipe();
    let registration2 =
        test.relay().register(src2_r, Destination::OwnedFd { fd: dst2_w }).await.unwrap();
    let id2 = registration2.id;
    relays.push((id2, dst2_r, data2.len()));

    let (src3_r, _src3_w) = create_data_pipe(data3);
    let (dst3_r, dst3_w) = create_pipe();
    let registration3 =
        test.relay().register(src3_r, Destination::OwnedFd { fd: dst3_w }).await.unwrap();
    let id3 = registration3.id;
    relays.push((id3, dst3_r, data3.len()));

    time::sleep(Duration::from_millis(150)).await;

    for (id, dst_r, expected_len) in &relays {
        let mut buf = vec![0u8; *expected_len];
        let n = unistd::read(dst_r, &mut buf).expect("read failed");
        assert_eq!(n, *expected_len, "Relay {id} incomplete transfer");
    }

    let status = test.relay().get_status().await.unwrap();
    let total_expected: usize = relays.iter().map(|(_, _, len)| len).sum();
    assert_eq!(status.bytes_transferred, total_expected as u64);
    assert_eq!(status.active_relays, 3);

    for (id, ..) in relays {
        test.relay().remove(id);
    }
    test.shutdown().await;
}

#[tokio::test]
async fn test_splice_eof_removes_relay() {
    let test = TestExecutor::new().await;

    let (src_r, src_w) = create_pipe();
    drop(src_w);
    let (_dst_r, dst_w) = create_pipe();
    let registration =
        test.relay().register(src_r, Destination::OwnedFd { fd: dst_w }).await.unwrap();
    let id = registration.id;

    time::sleep(Duration::from_millis(50)).await;

    let list = test.relay().list().await;
    assert!(!list.iter().any(|info| info.id == id), "Relay {id} should be removed after EOF");

    test.shutdown().await;
}

#[tokio::test]
async fn test_shutdown_during_active_splice() {
    let test = TestExecutor::new().await;

    let large_data = vec![0xAA; 1024 * 1024];
    let (src_r, _src_w) = create_data_pipe(&large_data);
    let (_dst_r, dst_w) = create_pipe();

    let registration =
        test.relay().register(src_r, Destination::OwnedFd { fd: dst_w }).await.unwrap();
    let id = registration.id;

    test.cancel_token.cancel();

    time::sleep(Duration::from_millis(50)).await;

    let list = test.relay().list().await;
    assert!(!list.iter().any(|info| info.id == id), "Relay {id} should be removed after shutdown");

    test.shutdown().await;
}

// ============== Destination Tests ==============

#[tokio::test]
async fn test_register_with_stdout() {
    let test = TestExecutor::new().await;

    let (src, _src_w) = create_pipe();
    let RelayRegistration { id, .. } = test
        .relay()
        .register(src, Destination::Stdout)
        .await
        .expect("register with stdout should succeed");

    assert!(id > 0, "Registered ID should be positive");

    let list = test.relay().list().await;
    assert_eq!(list.len(), 1, "Should have one relay");
    assert_eq!(list[0].id, id);

    test.shutdown().await;
}

#[tokio::test]
async fn test_register_with_stderr() {
    let test = TestExecutor::new().await;

    let (src, _src_w) = create_pipe();
    let RelayRegistration { id, .. } = test
        .relay()
        .register(src, Destination::Stderr)
        .await
        .expect("register with stderr should succeed");

    assert!(id > 0, "Registered ID should be positive");

    let list = test.relay().list().await;
    assert_eq!(list.len(), 1, "Should have one relay");
    assert_eq!(list[0].id, id);

    test.shutdown().await;
}

#[tokio::test]
async fn test_splice_to_stdout() {
    let test = TestExecutor::new().await;

    let test_data = b"Hello, stdout!";
    let (src_r, _src_w) = create_data_pipe(test_data);

    // Register relay to stdout
    let RelayRegistration { id, .. } =
        test.relay().register(src_r, Destination::Stdout).await.unwrap();

    // Wait for data to be spliced
    time::sleep(Duration::from_millis(50)).await;

    // Remove the relay
    test.relay().remove(id);

    // Check bytes transferred status
    let status = test.relay().get_status().await.unwrap();
    assert_eq!(status.bytes_transferred, test_data.len() as u64);

    test.shutdown().await;
}

#[tokio::test]
async fn test_splice_to_stderr() {
    let test = TestExecutor::new().await;

    let test_data = b"Hello, stderr!";
    let (src_r, _src_w) = create_data_pipe(test_data);

    // Register relay to stderr
    let RelayRegistration { id, .. } =
        test.relay().register(src_r, Destination::Stderr).await.unwrap();

    // Wait for data to be spliced
    time::sleep(Duration::from_millis(50)).await;

    // Remove the relay
    test.relay().remove(id);

    // Check bytes transferred status
    let status = test.relay().get_status().await.unwrap();
    assert_eq!(status.bytes_transferred, test_data.len() as u64);

    test.shutdown().await;
}

// ============== Start Notification Tests ==============

#[tokio::test]
async fn test_start_notification_received() {
    let test = TestExecutor::new().await;

    let test_data = b"Test data for notification";
    let (src_r, _src_w) = create_data_pipe(test_data);
    let (_dst_r, dst_w) = create_pipe();

    // Register with a start notification receiver
    let RelayRegistration { id, mut started } = test
        .relay()
        .register(src_r, Destination::OwnedFd { fd: dst_w })
        .await
        .expect("register should succeed");

    // Wait for data to be spliced
    time::sleep(Duration::from_millis(50)).await;

    // Now the notification should be received
    started.try_recv().expect("Should receive start notification");

    // Remove the relay
    test.relay().remove(id);
    test.shutdown().await;
}

#[tokio::test]
async fn test_start_notification_sent_only_once() {
    let test = TestExecutor::new().await;

    let test_data = b"Test data for single notification";
    let (src_r, _src_w) = create_data_pipe(test_data);
    let (_dst_r, dst_w) = create_pipe();

    let RelayRegistration { id, mut started } = test
        .relay()
        .register(src_r, Destination::OwnedFd { fd: dst_w })
        .await
        .expect("register should succeed");

    // Wait for first notification
    time::sleep(Duration::from_millis(50)).await;
    started.try_recv().expect("Should receive first notification");

    // Try to receive again - should be Err because sender was taken
    assert!(started.try_recv().is_err(), "Should not receive second notification");

    test.relay().remove(id);
    test.shutdown().await;
}

#[tokio::test]
async fn test_start_notification_with_stdout() {
    let test = TestExecutor::new().await;

    let (src, _src_w) = create_pipe();
    let RelayRegistration { id: _id, mut started } =
        test.relay().register(src, Destination::Stdout).await.unwrap();

    // Wait for potential splice (there's no data, so no notification expected)
    time::sleep(Duration::from_millis(30)).await;

    // Since there's no data to splice, notification should not be sent
    assert!(started.try_recv().is_err(), "Should not receive notification without data");

    test.shutdown().await;
}

#[tokio::test]
async fn test_start_notification_with_stderr() {
    let test = TestExecutor::new().await;

    let (src, _src_w) = create_pipe();
    let RelayRegistration { id: _id, mut started } =
        test.relay().register(src, Destination::Stderr).await.unwrap();

    // Wait for potential splice (there's no data, so no notification expected)
    time::sleep(Duration::from_millis(30)).await;

    // Since there's no data to splice, notification should not be sent
    assert!(started.try_recv().is_err(), "Should not receive notification without data");

    test.shutdown().await;
}

// ============== Cloning and Send/Sync Tests ==============

#[test]
fn test_splice_relay_is_send_sync() {
    fn is_send<T: Send>(_t: T) {}
    fn is_sync<T: Sync>(_t: T) {}

    let (relay, _executor) = Builder::new().build().unwrap();
    is_send(relay.clone());
    is_sync(&relay);
}

#[test]
fn test_executor_is_send() {
    fn is_send<T: Send>(_t: T) {}
    let (_relay, executor) = Builder::new().build().unwrap();
    is_send(executor);
}
