use agent_platform_agent::verification::{VerificationError, verify_file};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn file_verification_checks_exact_bytes_and_bounds_reads() -> Result<(), Box<dyn std::error::Error>>
{
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("agent-verifier-{}-{stamp}", std::process::id()));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    for (actual, expected, succeeds) in [
        (&b""[..], &b""[..], true),
        (&b"unexpected"[..], &b""[..], false),
        (&b"result"[..], &b"result"[..], true),
        (&b"resul"[..], &b"result"[..], false),
        (&b"result trailing"[..], &b"result"[..], false),
        (&b"wrong!"[..], &b"result"[..], false),
        (&b"\xff\x00"[..], &b"\xff\x00"[..], true),
    ] {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(actual)?;
        let result = verify_file(&mut file, expected);
        if succeeds {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(matches!(result, Err(VerificationError::ContentMismatch)));
        }
        assert!(file.stream_position()? <= expected.len() as u64 + 1);
    }
    let mut write_only = OpenOptions::new().write(true).open(&path)?;
    assert!(matches!(
        verify_file(&mut write_only, b"required"),
        Err(VerificationError::Read(_))
    ));
    drop(write_only);
    drop(file);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn special_file_is_not_an_artifact() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    let path = "/dev/null";
    #[cfg(windows)]
    let path = "NUL";
    let mut file = std::fs::File::open(path)?;
    assert!(matches!(
        verify_file(&mut file, b""),
        Err(VerificationError::UnsupportedFile)
    ));
    Ok(())
}
