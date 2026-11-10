use agent_platform_agent::store::{StoreError, TaskStore};
use rusqlite::{Connection, types::Value};
use std::collections::BTreeMap;

#[test]
fn v6_cancel_migration_preserves_rows_and_rolls_back_broken_references()
-> Result<(), Box<dyn std::error::Error>> {
    for broken in [false, true] {
        let directory =
            std::env::temp_dir().join(format!("agent-cancel-migration-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory)?;
        let path = directory.join("tasks.sqlite3");
        let connection = Connection::open(&path)?;
        connection.pragma_update(None, "foreign_keys", false)?;
        for sql in [
            include_str!("../src/store_v1.sql"),
            include_str!("../src/store_v2.sql"),
            include_str!("../src/store_v3.sql"),
            include_str!("../src/store_v4.sql"),
            include_str!("../src/store_v6.sql"),
        ] {
            connection.execute_batch(sql)?;
        }
        connection.pragma_update(None, "application_id", 0x41504754_i64)?;
        connection.execute_batch(
            "PRAGMA user_version=6;
             INSERT INTO tasks(task_id,generation,recorded_state,last_seq) VALUES ('legacy',2,'waiting_approval',3);
             INSERT INTO events(event_id,task_id,generation,seq,event_type) VALUES ('start','legacy',1,1,'started'),('resume','legacy',2,2,'started'),('approval-event','legacy',2,3,'approval_requested');
             INSERT INTO task_specs(task_id,spec_json,spec_hash) VALUES ('legacy','original spec bytes','original hash bytes');
             INSERT INTO tool_intents(intent_id,task_id,generation,call_id,tool_name,tool_version,args_json,args_hash,scope_path,status,step_id)
             VALUES ('intent','legacy',2,'call','write_output',1,'original args','hash','/workspace/output','prepared',0);
             INSERT INTO approvals(approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,tool_version,status,decision_source,expires_at,uses_remaining)
             VALUES ('approval','intent',1,1,'original hash bytes','hash',1,'pending','pending',9999999999,1);
             INSERT INTO sessions VALUES ('legacy',2,1,'original session bytes');
             INSERT INTO task_runs VALUES ('legacy',1,'cancelled',1,0,1,'original previous session bytes');"
        )?;
        if broken {
            connection.execute(
                "UPDATE tool_intents SET task_id='missing' WHERE intent_id='intent'",
                [],
            )?;
        }
        let tables = [
            "tasks",
            "events",
            "task_specs",
            "tool_intents",
            "approvals",
            "sessions",
            "task_runs",
        ];
        let mut before = BTreeMap::new();
        for table in tables {
            let mut query = connection.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
            let rows = query
                .query_map([], |row| {
                    (0..row.as_ref().column_count())
                        .map(|index| row.get::<_, Value>(index))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            before.insert(table, rows);
        }
        assert!(matches!(
            TaskStore::write(&path),
            Err(StoreError::Incompatible)
        ));
        let migrated = TaskStore::open(&path);
        if broken {
            assert!(matches!(migrated, Err(StoreError::Incompatible)));
        } else {
            let mut store = migrated?;
            assert_eq!(store.status("legacy")?.recorded_state, "waiting_approval");
            let (task, generation) = store.start()?;
            assert_eq!(store.status(&task)?.schema_version, 3);
            assert_eq!(store.events(&task, 0)?[0].schema_version, 4);
            store.request_cancel(&task, generation)?;
            assert_eq!(store.status(&task)?.recorded_state, "cancel_requested");
            connection.execute("DELETE FROM events WHERE task_id=?1", [&task])?;
            connection.execute("DELETE FROM tasks WHERE task_id=?1", [&task])?;
            drop(store);
            drop(TaskStore::open(&path)?);
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(version, if broken { 6 } else { 8 });
        for table in tables {
            let mut query = connection.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
            let rows = query
                .query_map([], |row| {
                    (0..row.as_ref().column_count())
                        .map(|index| row.get::<_, Value>(index))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            assert_eq!(rows, before[table], "{table}, broken={broken}");
        }
        if !broken {
            let mut store = TaskStore::write(&path)?;
            assert_eq!(store.status("legacy")?.schema_version, 2);
            store.request_cancel("legacy", 2)?;
            assert_eq!(store.status("legacy")?.schema_version, 3);
            assert_eq!(store.status("legacy")?.recorded_state, "cancel_requested");
        }
        drop(connection);
        std::fs::remove_dir_all(directory)?;
    }
    Ok(())
}
