//! Test-only HTTP request reader; TCP reads do not preserve request boundaries.

use std::io::{self, Read};

pub(crate) fn read_request(reader: &mut impl Read) -> io::Result<String> {
    const LIMIT: usize = 2 * 1024 * 1024;
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let count = reader.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete fixture request",
            ));
        }
        raw.extend_from_slice(&chunk[..count]);
        if raw.len() > LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixture request exceeds limit",
            ));
        }
        let Some(end) = raw.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&raw[..end]);
        let length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then_some(value.trim())
            })
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid fixture content length")
            })?;
        let total = end
            .checked_add(4)
            .and_then(|n| n.checked_add(length))
            .filter(|n| *n <= LIMIT)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "fixture request exceeds limit")
            })?;
        if raw.len() >= total {
            raw.truncate(total);
            return String::from_utf8(raw).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid fixture request UTF-8")
            });
        }
    }
}

#[test]
fn fragmented_headers_and_body_are_read_completely() {
    struct Fragments(std::io::Cursor<Vec<u8>>);
    impl Read for Fragments {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let count = buf.len().min(3);
            self.0.read(&mut buf[..count])
        }
    }
    let body = r#"{"model":"fixture"}"#;
    let request = format!(
        "POST /chat HTTP/1.1\r\nAuthorization: Bearer fixture\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let mut input = Fragments(std::io::Cursor::new(request.as_bytes().to_vec()));
    assert_eq!(read_request(&mut input).unwrap(), request);
}
