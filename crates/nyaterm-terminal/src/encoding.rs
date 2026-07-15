//! Session character encoding for the terminal I/O path.
//!
//! Graphics protocol segments are handled on raw session bytes. Only the
//! remaining terminal stream is decoded to UTF-8 for Alacritty. Outgoing
//! text (paste / typed input) is re-encoded to the session charset.

use encoding_rs::{Decoder, Encoding, GBK, UTF_8};

/// Stateful charset converter owned by [`crate::TerminalCore`].
pub struct SessionEncoding {
    label: String,
    encoding: &'static Encoding,
    decoder: Decoder,
}

impl std::fmt::Debug for SessionEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionEncoding")
            .field("label", &self.label)
            .field("encoding", &self.encoding.name())
            .finish()
    }
}

impl Default for SessionEncoding {
    fn default() -> Self {
        Self::from_label("UTF-8")
    }
}

impl Clone for SessionEncoding {
    fn clone(&self) -> Self {
        Self::from_label(&self.label)
    }
}

impl SessionEncoding {
    /// Resolve a user/settings label (`UTF-8`, `GBK`, …) to a streaming converter.
    pub fn from_label(label: &str) -> Self {
        let (label, encoding) = resolve_encoding(label);
        Self {
            label: label.to_string(),
            encoding,
            decoder: encoding.new_decoder_without_bom_handling(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_utf8(&self) -> bool {
        self.encoding == UTF_8
    }

    /// Decode one output chunk to text.
    ///
    /// Incomplete multi-byte sequences are retained across calls, including
    /// UTF-8 sessions where output consumers may receive arbitrary byte chunks.
    pub fn decode_output_text(&mut self, input: &[u8]) -> String {
        if input.is_empty() {
            return String::new();
        }
        let mut dst = String::with_capacity(input.len());
        let mut src = input;
        loop {
            let (result, read, _replacements) = self.decoder.decode_to_string(src, &mut dst, false);
            src = &src[read..];
            match result {
                encoding_rs::CoderResult::InputEmpty => break,
                encoding_rs::CoderResult::OutputFull => {
                    dst.reserve(dst.capacity().max(16));
                }
            }
        }
        dst
    }

    /// Decode one output chunk to UTF-8 bytes for the ANSI parser.
    ///
    /// Incomplete multi-byte sequences are retained across calls.
    pub fn decode_output_chunk(&mut self, input: &[u8]) -> Vec<u8> {
        self.decode_output_text(input).into_bytes()
    }

    /// Encode UTF-8 text for the session wire format (paste / typed text).
    pub fn encode_str(&self, text: &str) -> Vec<u8> {
        if self.is_utf8() {
            return text.as_bytes().to_vec();
        }
        let (cow, _, _) = self.encoding.encode(text);
        cow.into_owned()
    }

    /// Encode a UTF-8 byte buffer. Non-UTF-8 / pure-ASCII control payloads pass through.
    pub fn encode_outgoing(&self, utf8_or_ascii: &[u8]) -> Vec<u8> {
        if self.is_utf8() || utf8_or_ascii.is_empty() {
            return utf8_or_ascii.to_vec();
        }
        if utf8_or_ascii.iter().all(|b| b.is_ascii()) {
            return utf8_or_ascii.to_vec();
        }
        match std::str::from_utf8(utf8_or_ascii) {
            Ok(text) => self.encode_str(text),
            Err(_) => utf8_or_ascii.to_vec(),
        }
    }

    /// Reset decoder state (e.g. after a hard screen clear/reconnect).
    pub fn reset_decoder(&mut self) {
        self.decoder = self.encoding.new_decoder_without_bom_handling();
    }
}

fn resolve_encoding(label: &str) -> (&'static str, &'static Encoding) {
    let trimmed = label.trim();
    if trimmed.eq_ignore_ascii_case("gbk")
        || trimmed.eq_ignore_ascii_case("gb2312")
        || trimmed.eq_ignore_ascii_case("cp936")
    {
        ("GBK", GBK)
    } else if trimmed.eq_ignore_ascii_case("gb18030") {
        ("GB18030", encoding_rs::GB18030)
    } else if trimmed.eq_ignore_ascii_case("big5") {
        ("Big5", encoding_rs::BIG5)
    } else if trimmed.eq_ignore_ascii_case("shift_jis")
        || trimmed.eq_ignore_ascii_case("shift-jis")
        || trimmed.eq_ignore_ascii_case("sjis")
    {
        ("Shift_JIS", encoding_rs::SHIFT_JIS)
    } else if trimmed.eq_ignore_ascii_case("euc-kr") || trimmed.eq_ignore_ascii_case("euckr") {
        ("EUC-KR", encoding_rs::EUC_KR)
    } else {
        ("UTF-8", UTF_8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_is_passthrough() {
        let mut enc = SessionEncoding::from_label("UTF-8");
        assert!(enc.is_utf8());
        assert_eq!(enc.decode_output_chunk(b"hi"), b"hi");
        assert_eq!(enc.encode_str("hi"), b"hi");
    }

    #[test]
    fn utf8_split_multibyte_across_chunks() {
        let mut enc = SessionEncoding::from_label("UTF-8");
        let bytes = "测".as_bytes();

        let part1 = enc.decode_output_text(&bytes[..1]);
        assert!(
            part1.is_empty(),
            "incomplete UTF-8 byte should be held, got {part1:?}"
        );
        let part2 = enc.decode_output_text(&bytes[1..]);
        assert_eq!(part2, "测");
    }

    #[test]
    fn gbk_roundtrip_chinese() {
        let mut enc = SessionEncoding::from_label("GBK");
        assert!(!enc.is_utf8());
        // "测试" in GBK
        let gbk = [0xb2, 0xe2, 0xca, 0xd4];
        let utf8 = enc.decode_output_chunk(&gbk);
        assert_eq!(String::from_utf8(utf8.clone()).unwrap(), "测试");
        assert_eq!(enc.encode_str("测试"), gbk);
        // Outgoing UTF-8 bytes re-encode.
        assert_eq!(enc.encode_outgoing("测试".as_bytes()), gbk);
        // ASCII CSI stays intact.
        assert_eq!(enc.encode_outgoing(b"\x1b[A"), b"\x1b[A");
    }

    #[test]
    fn gbk_split_multibyte_across_chunks() {
        let mut enc = SessionEncoding::from_label("GBK");
        // First byte of "测" (0xb2 0xe2)
        let part1 = enc.decode_output_chunk(&[0xb2]);
        assert!(
            part1.is_empty(),
            "incomplete GBK byte should be held, got {part1:?}"
        );
        let part2 = enc.decode_output_chunk(&[0xe2, 0xca, 0xd4]);
        let combined = [part1, part2].concat();
        assert_eq!(String::from_utf8(combined).unwrap(), "测试");
    }
}
