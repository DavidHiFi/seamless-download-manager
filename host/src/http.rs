use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

/// One HTTP/1.1 request to the app on loopback, with the connection closed afterwards.
pub fn request(port: u16, method: &str, path: &str, api_key: Option<&str>, body: Option<&Value>) -> Result<(u16, Value)> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let payload = match body {
        Some(v) => serde_json::to_vec(v)?,
        None => Vec::new(),
    };
    let key_line = match api_key {
        Some(key) if !key.contains(['\r', '\n']) => format!("X-Api-Key: {key}\r\n"),
        Some(_) => bail!("the api key is malformed"),
        None => String::new(),
    };
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{key_line}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&payload)?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    parse_response(&raw)
}

pub fn parse_response(raw: &[u8]) -> Result<(u16, Value)> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| anyhow!("the app sent an incomplete reply"))?;
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| anyhow!("the app sent an unreadable reply"))?;
    let status: u16 = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| anyhow!("the app sent an unreadable status"))?;
    let chunked = head.lines().any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:") && l.to_ascii_lowercase().contains("chunked"));
    let body = &raw[split + 4..];
    let body = if chunked { dechunk(body)? } else { body.to_vec() };
    let value = if body.iter().all(u8::is_ascii_whitespace) { Value::Null } else { serde_json::from_slice(&body)? };
    Ok((status, value))
}

fn dechunk(mut data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let eol = data.windows(2).position(|w| w == b"\r\n").ok_or_else(|| anyhow!("bad chunked reply"))?;
        let size = std::str::from_utf8(&data[..eol]).ok().and_then(|s| usize::from_str_radix(s.split(';').next()?.trim(), 16).ok()).ok_or_else(|| anyhow!("bad chunk size"))?;
        data = &data[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if data.len() < size + 2 {
            bail!("bad chunked reply");
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_chunked() {
        let plain = b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";
        assert_eq!(parse_response(plain).unwrap(), (200, serde_json::json!({"ok": true})));
        let chunked = b"HTTP/1.1 401 Unauthorized\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n{\"ok\"\r\n6\r\n:false\r\n1\r\n}\r\n0\r\n\r\n";
        assert_eq!(parse_response(chunked).unwrap(), (401, serde_json::json!({"ok": false})));
        assert!(parse_response(b"garbage").is_err());
    }
}
