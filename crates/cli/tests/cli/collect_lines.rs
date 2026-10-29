use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::Sender;

pub fn collect_lines(pipe: impl Read, sink: Option<Sender<String>>) -> String {
    let mut reader = BufReader::new(pipe);
    let mut all = String::new();
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        if !matches!(reader.read_until(b'\n', &mut buffer), Ok(count) if count > 0) {
            return all;
        }
        let line = String::from_utf8_lossy(&buffer).into_owned();
        all.push_str(&line);
        if let Some(sink) = &sink {
            let _ = sink.send(line.trim_end().to_string());
        }
    }
}
