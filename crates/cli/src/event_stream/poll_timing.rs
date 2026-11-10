use super::EventStream;
use agent_platform_agent::store::TaskStore;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test(start_paused = true)]
async fn poll_waits_only_for_an_empty_follow_page() {
    let folder = tempfile::tempdir().unwrap();
    let mut store = TaskStore::open(&folder.path().join("tasks.sqlite3")).unwrap();
    let (task_id, _) = store.start().unwrap();
    let mut stream = EventStream {
        store,
        task_id,
        after_seq: 0,
        follow: true,
    };
    let mut output = Vec::new();
    let start = Instant::now();
    stream.read_page(&mut output).await.unwrap();
    assert_eq!(Instant::now(), start);
    let event: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(event["seq"], 1);
    let first_output = output.clone();
    let start = Instant::now();
    stream.read_page(&mut output).await.unwrap();
    assert_eq!(start.elapsed(), Duration::from_millis(200));
    stream.follow = false;
    let start = Instant::now();
    stream.read_page(&mut output).await.unwrap();
    assert_eq!(Instant::now(), start);
    assert_eq!(output, first_output);
}
