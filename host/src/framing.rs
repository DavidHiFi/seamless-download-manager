use std::io::{self, Read, Write};

/// Largest message the host accepts from the browser.
pub const MAX_IN: usize = 4 * 1024 * 1024;
/// Chrome refuses replies over 1 MiB.
pub const MAX_OUT: usize = 1024 * 1024;

/// Reads one length-prefixed message. `Ok(None)` means the browser closed the pipe between messages.
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    let mut got = 0;
    while got < len.len() {
        match reader.read(&mut len[got..])? {
            0 if got == 0 => return Ok(None),
            0 => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "pipe closed inside a length prefix")),
            n => got += n,
        }
    }
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_IN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message is larger than 4 MiB"));
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

pub fn write_message<W: Write>(writer: &mut W, body: &[u8]) -> io::Result<()> {
    if body.len() > MAX_OUT {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "reply is larger than 1 MiB"));
    }
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(body)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut wire = Vec::new();
        write_message(&mut wire, "h\u{e9}llo".as_bytes()).unwrap();
        assert_eq!(&wire[..4], &(6u32).to_le_bytes());
        let mut cursor = io::Cursor::new(wire);
        assert_eq!(read_message(&mut cursor).unwrap().unwrap(), "h\u{e9}llo".as_bytes());
        assert!(read_message(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn two_messages_in_a_row() {
        let mut wire = Vec::new();
        write_message(&mut wire, b"{}").unwrap();
        write_message(&mut wire, b"[]").unwrap();
        let mut cursor = io::Cursor::new(wire);
        assert_eq!(read_message(&mut cursor).unwrap().unwrap(), b"{}");
        assert_eq!(read_message(&mut cursor).unwrap().unwrap(), b"[]");
    }

    #[test]
    fn rejects_oversize_and_truncated() {
        let mut huge = io::Cursor::new(((MAX_IN + 1) as u32).to_le_bytes().to_vec());
        assert!(read_message(&mut huge).is_err());
        let mut cut = io::Cursor::new(vec![9, 0, 0, 0, b'a']);
        assert!(read_message(&mut cut).is_err());
        let mut half_prefix = io::Cursor::new(vec![1, 0]);
        assert!(read_message(&mut half_prefix).is_err());
        let mut sink = Vec::new();
        assert!(write_message(&mut sink, &vec![0u8; MAX_OUT + 1]).is_err());
        assert!(sink.is_empty());
    }

    #[test]
    fn empty_message_is_allowed() {
        let mut cursor = io::Cursor::new(vec![0, 0, 0, 0]);
        assert_eq!(read_message(&mut cursor).unwrap().unwrap(), Vec::<u8>::new());
    }
}
