//! Wire framing for the ESPHome Native API.
//!
//! Each message is `[0x00][varint length][varint type][payload]`.
//! The leading zero byte is the protocol indicator; older builds rejected
//! anything else with "Bad indicator byte".

use crate::error::Error;

/// Number of bytes in a protocol frame indicator.
pub const INDICATOR_BYTE: u8 = 0x00;

/// Maximum payload we accept from a device (64 MiB), to bound memory use
/// per AGENTS.md rule 9.
pub const MAX_FRAME_SIZE: usize = 64 * 1024 * 1024;

/// Encoded frame: indicator, length, message type, then payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Message type ID.
    pub message_type: u16,
    /// Protobuf-encoded payload.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a frame of the given message type with an empty payload.
    pub fn new(message_type: u16) -> Self {
        Self {
            message_type,
            payload: Vec::new(),
        }
    }

    /// Create a frame with a protobuf payload.
    pub fn with_payload(message_type: u16, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            payload,
        }
    }

    /// Encode this frame to its wire representation.
    ///
    /// The length varint covers only the payload; the message type is a
    /// separate varint after it (per the ESPHome protocol spec).
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let body_len = self.payload.len();
        let mut out = Vec::with_capacity(body_len + varint_len(body_len as u64) + 1);
        out.push(INDICATOR_BYTE);
        write_varint(&mut out, body_len as u64);
        write_varint(&mut out, self.message_type.into());
        out.extend_from_slice(&self.payload);
        out
    }
}

/// Encode a varint into `out`.
pub fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value as u8) & 0x7f;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Number of bytes `value` occupies as a varint.
#[must_use]
pub const fn varint_len(value: u64) -> usize {
    let mut bits = 64 - value.leading_zeros();
    if bits == 0 {
        return 1;
    }
    let mut len = 0;
    while bits > 0 {
        len += 1;
        bits = bits.saturating_sub(7);
    }
    len
}

/// Decoder for reading frames from a byte stream.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Create an empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Feed raw bytes from the socket into the decoder.
    pub fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Try to decode the next complete frame, if one is buffered.
    ///
    /// # Errors
    /// Returns [`Error::ProtocolViolation`] when the stream is malformed
    /// (bad indicator byte, oversize frame, truncated varint).
    pub fn next_frame(&mut self) -> Result<Option<Frame>, Error> {
        let mut pos = 0usize;

        if self.buffer.is_empty() {
            return Ok(None);
        }
        if self.buffer[0] != INDICATOR_BYTE {
            return Err(Error::protocol(format!(
                "expected indicator byte 0x00, got 0x{:02x}",
                self.buffer[0]
            )));
        }
        pos += 1;

        // Payload length (bounded so a hostile/buggy device can't exhaust RAM).
        let (len, consumed) = match read_varint(&self.buffer[pos..])? {
            Some(v) => v,
            None => return Ok(None), // partial varint: wait for more bytes
        };
        let length = usize::try_from(len).map_err(|_| Error::frame_too_large(len))?;
        if length > MAX_FRAME_SIZE {
            return Err(Error::frame_too_large(len));
        }
        pos += consumed;

        let (type_id, consumed) = match read_varint(&self.buffer[pos..])? {
            Some(v) => v,
            None => return Ok(None),
        };
        if type_id > u64::from(u16::MAX) {
            return Err(Error::protocol(format!(
                "message type {type_id} exceeds u16 range"
            )));
        }
        pos += consumed;

        let total = pos
            .checked_add(length)
            .ok_or_else(|| Error::protocol("frame length overflow"))?;
        if self.buffer.len() < total {
            // Still waiting for the rest of the payload.
            return Ok(None);
        }

        let payload = self.buffer[pos..pos + length].to_vec();
        self.buffer.drain(..total);
        Ok(Some(Frame {
            message_type: type_id as u16,
            payload,
        }))
    }
}

/// Read a varint from the start of `buf`.
/// Returns the value and how many bytes it consumed.
fn read_varint(buf: &[u8]) -> Result<Option<(u64, usize)>, Error> {
    let mut value: u64 = 0;
    let mut shift = 0u32;
    for (i, &byte) in buf.iter().enumerate() {
        if shift > 63 {
            return Err(Error::protocol("varint exceeds 64 bits"));
        }
        value |= u64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Ok(Some((value, i + 1)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_proto::HelloResponse;
    use prost::Message as _;

    #[test]
    fn roundtrip_small_frame() {
        let frame = Frame::with_payload(33, vec![0x0d, 0xde, 0xad, 0xbe, 0xef]);
        let wire = frame.encode();
        let mut decoder = FrameDecoder::new();
        decoder.extend(&wire);
        let decoded = decoder.next_frame().unwrap().unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let frame = Frame::new(7);
        let mut decoder = FrameDecoder::new();
        decoder.extend(&frame.encode());
        assert_eq!(decoder.next_frame().unwrap().unwrap(), frame);
    }

    #[test]
    fn partial_data_yields_none_then_frame() {
        let frame = Frame::with_payload(26, vec![1, 2, 3, 4, 5]);
        let wire = frame.encode();
        let mut decoder = FrameDecoder::new();
        for (i, chunk) in wire.chunks(1).enumerate() {
            decoder.extend(chunk);
            if i + 1 < wire.len() {
                assert!(
                    decoder.next_frame().unwrap().is_none(),
                    "frame decoded after only {} of {} bytes",
                    i + 1,
                    wire.len()
                );
            }
        }
        assert_eq!(decoder.next_frame().unwrap().unwrap(), frame);
    }

    #[test]
    fn multiple_frames_in_one_buffer() {
        let a = Frame::new(7);
        let b = Frame::with_payload(9, vec![0xff; 40]);
        let mut decoder = FrameDecoder::new();
        let mut wire = a.encode();
        wire.extend(b.encode());
        decoder.extend(&wire);
        assert_eq!(decoder.next_frame().unwrap().unwrap(), a);
        assert_eq!(decoder.next_frame().unwrap().unwrap(), b);
        assert!(decoder.next_frame().unwrap().is_none());
    }

    #[test]
    fn rejects_bad_indicator() {
        let mut decoder = FrameDecoder::new();
        decoder.extend(&[0x01, 0x02]);
        assert!(decoder.next_frame().is_err());
    }

    #[test]
    fn handles_real_device_greeting() {
        // Captured from the real ESP32 (eam-test-device, 10.60.109.61:6053)
        // during Phase 0 validation: HelloResponse, API 1.19, "2026.10.2",
        // node name "eam-test-device".
        let wire: &[u8] = &[
            0x00, 0x20, 0x02, 0x08, 0x01, 0x10, 0x13, 0x1a, 0x09, b'2', b'0', b'2', b'6', b'.',
            b'1', b'0', b'.', b'2', 0x22, 0x0f, b'e', b'a', b'm', b'-', b't', b'e', b's', b't',
            b'-', b'd', b'e', b'v', b'i', b'c', b'e',
        ];
        let mut decoder = FrameDecoder::new();
        decoder.extend(wire);
        let frame = decoder.next_frame().unwrap().unwrap();
        assert_eq!(frame.message_type, 2, "hello response");
        assert_eq!(frame.payload.len(), 32);
        let response = HelloResponse::decode(&frame.payload[..]).unwrap();
        assert_eq!(response.api_version_major, 1);
        assert_eq!(response.api_version_minor, 19);
        assert_eq!(response.server_info, "2026.10.2");
        assert_eq!(response.name, "eam-test-device");
    }

    #[test]
    fn roundtrip_varint() {
        for value in [0u64, 1, 127, 128, 300, 16383, 16384, u64::MAX] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            assert_eq!(buf.len(), varint_len(value));
            let (decoded, consumed) = read_varint(&buf).unwrap().unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, buf.len());
        }
    }
}
