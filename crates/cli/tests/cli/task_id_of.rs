pub fn task_id_of(stderr: &str) -> String {
    stderr
        .split("任務編號：")
        .nth(1)
        .and_then(|rest| rest.lines().next())
        .expect("task identifier line")
        .to_string()
}
