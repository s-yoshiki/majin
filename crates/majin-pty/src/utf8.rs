//! Incremental UTF-8 decoding for PTY output.
//!
//! Reads off a PTY land on arbitrary byte boundaries, so a multi-byte
//! character can straddle two chunks. Decoding each chunk independently would
//! corrupt any non-ASCII output — CJK text and box-drawing characters in TUI
//! programs break first and most visibly.

/// Buffers the trailing bytes of a truncated character until the rest arrives.
#[derive(Debug, Default)]
pub struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decodes everything currently decodable, holding back a partial tail.
    ///
    /// Genuinely invalid bytes become U+FFFD rather than stalling the stream:
    /// a PTY can legitimately carry binary noise (a stray `cat` of a binary),
    /// and dropping the session over it would be worse than showing garbage.
    pub fn push(&mut self, chunk: &[u8]) -> String {
        self.pending.extend_from_slice(chunk);
        let mut out = String::with_capacity(self.pending.len());

        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(text) => {
                    out.push_str(text);
                    self.pending.clear();
                    break;
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if valid_up_to > 0 {
                        match std::str::from_utf8(&self.pending[..valid_up_to]) {
                            Ok(text) => out.push_str(text),
                            Err(_) => unreachable!("valid_up_to marks a valid prefix"),
                        }
                    }
                    match err.error_len() {
                        // An invalid sequence: skip it and carry on.
                        Some(len) => {
                            self.pending.drain(..valid_up_to + len);
                            out.push(char::REPLACEMENT_CHARACTER);
                        }
                        // A truncated sequence: keep it for the next chunk.
                        None => {
                            self.pending.drain(..valid_up_to);
                            break;
                        }
                    }
                }
            }
        }

        out
    }

    /// Flushes any bytes still held back, at end of stream.
    pub fn finish(&mut self) -> String {
        if self.pending.is_empty() {
            return String::new();
        }
        let out = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_ascii_straight_through() {
        let mut stream = Utf8Stream::new();
        assert_eq!(stream.push(b"hello"), "hello");
    }

    #[test]
    fn reassembles_a_character_split_across_chunks() {
        let mut stream = Utf8Stream::new();
        let bytes = "日本語".as_bytes();
        // Cut in the middle of the first character.
        assert_eq!(stream.push(&bytes[..2]), "");
        assert_eq!(stream.push(&bytes[2..]), "日本語");
    }

    #[test]
    fn reassembles_across_many_single_byte_chunks() {
        let mut stream = Utf8Stream::new();
        let mut decoded = String::new();
        for byte in "こんにちは".as_bytes() {
            decoded.push_str(&stream.push(&[*byte]));
        }
        assert_eq!(decoded, "こんにちは");
    }

    #[test]
    fn replaces_invalid_bytes_instead_of_stalling() {
        let mut stream = Utf8Stream::new();
        assert_eq!(stream.push(&[b'a', 0xff, b'b']), "a\u{fffd}b");
    }

    #[test]
    fn finish_flushes_a_dangling_partial_character() {
        let mut stream = Utf8Stream::new();
        assert_eq!(stream.push(&"日".as_bytes()[..1]), "");
        assert_eq!(stream.finish(), "\u{fffd}");
    }
}
