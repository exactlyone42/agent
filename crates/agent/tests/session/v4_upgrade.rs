use agent_harness::Session;
use agent_platform_agent::store::TaskStore;
use rusqlite::Connection;
use serde_json::json;

#[test]
fn v4_upgrade_preserves_history_and_advertises_the_new_format()
-> Result<(), Box<dyn std::error::Error>> {
    let path =
        std::env::temp_dir().join(format!("agent-v4-history-{}.sqlite3", uuid::Uuid::new_v4()));
    let connection = Connection::open(&path)?;
    connection.execute_batch(include_str!("../../src/store_v1.sql"))?;
    connection.execute_batch(include_str!("../../src/store_v2.sql"))?;
    connection.execute_batch(include_str!("../../src/store_v3.sql"))?;
    connection.execute_batch(include_str!("../../src/store_v4.sql"))?;
    connection.execute_batch(
        "PRAGMA application_id=1095780180;
         PRAGMA user_version=4;
         INSERT INTO tasks(task_id,recorded_state,last_seq) VALUES ('legacy','running',1)",
    )?;
    let legacy = json!({
        "revision": 1,
        "messages": [{"User": "work"}],
        "next_turn": 0,
        "completed": false,
        "unresolved_effect": false
    });
    let original = legacy.to_string();
    connection.execute(
        "INSERT INTO sessions(task_id,generation,revision,session_json) VALUES ('legacy',1,1,?1)",
        [&original],
    )?;
    drop(connection);
    let store = TaskStore::open(&path)?;
    let connection = Connection::open(&path)?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    assert_eq!(version, 8);
    let saved: String = connection.query_row(
        "SELECT session_json FROM sessions WHERE task_id='legacy'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(saved, original);
    let restored: Session = serde_json::from_str(&saved)?;
    assert_eq!(serde_json::to_value(restored)?, legacy);
    let tasks: i64 = connection.query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))?;
    assert_eq!(tasks, 1);
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}
