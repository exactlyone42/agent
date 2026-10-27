use std::process::Command;

#[test]
fn cli_contract_uses_real_binary_and_local_http() -> Result<(), Box<dyn std::error::Error>> {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/test_cli.py");
    let result = Command::new(python)
        .arg(script)
        .arg("-v")
        .env("AGENT_CLI_BINARY", env!("CARGO_BIN_EXE_agent-platform"))
        .output()?;
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}
