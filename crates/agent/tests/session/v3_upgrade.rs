use agent_platform_agent::store::TaskStore;

#[test]
fn v3_upgrade_installs_history_and_rolls_back_when_table_conflicts()
-> Result<(), Box<dyn std::error::Error>> {
    for fail in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "agent-history-migrate-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let connection = rusqlite::Connection::open(&path)?;
        connection.execute_batch(include_str!("../../src/store_v1.sql"))?;
        connection.execute_batch(include_str!("../../src/store_v2.sql"))?;
        connection.execute_batch(include_str!("../../src/store_v3.sql"))?;
        connection.execute_batch("PRAGMA application_id=1095780180; PRAGMA user_version=3; INSERT INTO tasks(task_id,recorded_state,last_seq) VALUES ('legacy','running',1)")?;
        if fail {
            connection.execute_batch("CREATE TABLE sessions(value TEXT)")?;
        }
        assert_eq!(TaskStore::open(&path).is_err(), fail);
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(version, if fail { 3 } else { 8 });
        let task: String =
            connection.query_row("SELECT task_id FROM tasks", [], |row| row.get(0))?;
        assert_eq!(task, "legacy");
        drop(connection);
        std::fs::remove_file(path)?;
    }
    Ok(())
}
