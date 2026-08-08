use std::io::{self, BufRead};

pub(crate) const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// Read one LSP/DAP Content-Length framed message. Protocol peers may include
/// additional headers (for example Content-Type), and header names are
/// case-insensitive. Leading blank lines are ignored so a stray separator does
/// not desynchronise the rest of the stream.
pub(crate) fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut content_length = None;
    let mut saw_header = false;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            if saw_header {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "protocol stream ended inside a header block",
                ));
            }
            return Ok(None);
        }

        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            if !saw_header {
                continue;
            }
            break;
        }
        saw_header = true;

        let Some((name, value)) = header.split_once(':') else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid protocol header: {header}"),
            ));
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length value: {}", value.trim()),
                )
            })?;
            if let Some(previous) = content_length {
                if previous != parsed {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "conflicting Content-Length headers",
                    ));
                }
            }
            content_length = Some(parsed);
        }
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "protocol header block is missing Content-Length",
        )
    })?;
    if content_length > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Content-Length {content_length} exceeds {MAX_FRAME_BYTES} byte limit"),
        ));
    }

    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::{read_frame, MAX_FRAME_BYTES};
    use std::io::{Cursor, ErrorKind};

    #[test]
    fn reads_extra_headers_and_case_insensitive_content_length() {
        let body = br#"{\"jsonrpc\":\"2.0\",\"id\":1}"#;
        let framed = format!(
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: {}\r\nX-Trace: abc\r\n\r\n{}",
            body.len(),
            String::from_utf8_lossy(body)
        );
        let mut input = Cursor::new(framed.into_bytes());

        assert_eq!(read_frame(&mut input).unwrap().unwrap(), body);
        assert!(read_frame(&mut input).unwrap().is_none());
    }

    #[test]
    fn reads_consecutive_frames_without_losing_boundaries() {
        let input = b"\r\nContent-Length: 3\r\nContent-Type: x\r\n\r\noneCONTENT-LENGTH: 3\n\ntwo";
        let mut input = Cursor::new(input.as_slice());

        assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"one");
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"two");
        assert!(read_frame(&mut input).unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_and_truncated_frames() {
        let oversized = format!("Content-Length: {}\r\n\r\n", MAX_FRAME_BYTES + 1);
        let err = read_frame(&mut Cursor::new(oversized.into_bytes())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);

        let err = read_frame(&mut Cursor::new(b"Content-Length: 5\r\n\r\nabc")).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
    }
}
