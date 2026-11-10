//! Minimal DNS-SD (mDNS) message encoding and decoding.
//!
//! Only what ESPHome discovery needs: PTR, SRV, TXT, and A records, with
//! name-compression pointers. Unknown record types are skipped, not rejected,
//! so a device adding records cannot break the client.

use crate::error::{Error, Result};

/// mDNS multicast address and port.
pub const MDNS_ADDR: &str = "224.0.0.251";
/// mDNS port.
pub const MDNS_PORT: u16 = 5353;
/// ESPHome's DNS-SD service type.
pub const ESPHOME_SERVICE_TYPE: &str = "_esphomelib._tcp.local";

/// DNS resource record type we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// Host address (IPv4).
    A,
    /// Reverse pointer to a service instance.
    Ptr,
    /// Service location (host + port).
    Srv,
    /// Free-form key/value strings.
    Txt,
}

impl RecordType {
    /// Numeric DNS type code.
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::A => 1,
            Self::Ptr => 12,
            Self::Txt => 16,
            Self::Srv => 33,
        }
    }

    /// Parse a type code, ignoring types we do not handle.
    #[must_use]
    pub const fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::A),
            12 => Some(Self::Ptr),
            16 => Some(Self::Txt),
            33 => Some(Self::Srv),
            _ => None,
        }
    }
}

/// A single decoded resource record's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordData {
    /// IPv4 address bytes.
    A([u8; 4]),
    /// Target domain name.
    Ptr(String),
    /// Priority, weight, port, hostname.
    Srv {
        /// Service priority.
        priority: u16,
        /// Service weight.
        weight: u16,
        /// TCP port the service listens on.
        port: u16,
        /// Hostname providing the service.
        target: String,
    },
    /// Key/value strings, already combined (`key=value`).
    Txt(Vec<String>),
}

/// A decoded response record.
#[derive(Debug, Clone)]
pub struct ResourceRecord {
    /// Owner name, lowercased and without the trailing dot.
    pub name: String,
    /// Record type.
    pub record_type: RecordType,
    /// Decoded payload.
    pub data: RecordData,
}

/// Encode a multicast query for PTR records of `service_type`.
///
/// # Errors
/// Returns [`Error::Dns`] when `service_type` cannot be encoded.
pub fn encode_ptr_query(service_type: &str) -> Result<Vec<u8>> {
    let mut msg = Vec::with_capacity(64);
    msg.extend_from_slice(&[0x00, 0x00]); // transaction id (mDNS ignores it)
    msg.extend_from_slice(&[0x00, 0x00]); // standard query
    msg.extend_from_slice(&[0x00, 0x01]); // one question
    msg.extend_from_slice(&[0x00, 0x00]); // no answers
    msg.extend_from_slice(&[0x00, 0x00]); // no authority records
    msg.extend_from_slice(&[0x00, 0x00]); // no additional records
    encode_name(&mut msg, service_type)?;
    msg.extend_from_slice(&RecordType::Ptr.code().to_be_bytes()); // qtype
    msg.extend_from_slice(&[0x00, 0x01]); // qclass IN
    Ok(msg)
}

/// Encode a domain name as DNS labels.
fn encode_name(out: &mut Vec<u8>, name: &str) -> Result<()> {
    let trimmed = name.trim_end_matches('.');
    if trimmed.is_empty() {
        out.push(0x00);
        return Ok(());
    }
    for label in trimmed.split('.') {
        let len = label.len();
        if len == 0 || len > 63 {
            return Err(Error::Dns(format!("invalid label in {name}")));
        }
        out.push(u8::try_from(len).map_err(|_| Error::Dns("label too long".into()))?);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0x00);
    Ok(())
}

/// Decode an mDNS response into its resource records.
///
/// # Errors
/// Returns [`Error::Dns`] when the packet is malformed.
pub fn decode_message(packet: &[u8]) -> Result<Vec<ResourceRecord>> {
    if packet.len() < 12 {
        return Err(Error::Dns("packet shorter than header".into()));
    }
    let answer_count = u16::from_be_bytes([packet[6], packet[7]]);
    let authority_count = u16::from_be_bytes([packet[8], packet[9]]);
    let additional_count = u16::from_be_bytes([packet[10], packet[11]]);

    let question_count = u16::from_be_bytes([packet[4], packet[5]]);
    let mut cursor = Cursor {
        data: packet,
        pos: 12,
    };

    // Skip questions: name, qtype, qclass.
    for _ in 0..question_count {
        cursor.read_name()?;
        cursor.skip(4)?;
    }

    let total = answer_count + authority_count + additional_count;
    let mut records = Vec::with_capacity(total as usize);
    for _ in 0..total {
        let name = cursor.read_name()?.to_ascii_lowercase();
        let rr_type = u16::from_be_bytes([cursor.u8()?, cursor.u8()?]);
        let _class = u16::from_be_bytes([cursor.u8()?, cursor.u8()?]);
        let _ttl = u32::from_be_bytes([cursor.u8()?, cursor.u8()?, cursor.u8()?, cursor.u8()?]);
        let rdlength = u32::from(u16::from_be_bytes([cursor.u8()?, cursor.u8()?]));
        let rdata_end = cursor
            .pos
            .checked_add(rdlength as usize)
            .filter(|end| *end <= packet.len())
            .ok_or_else(|| Error::Dns("record length runs past the packet".into()))?;
        let rdata = &packet[cursor.pos..rdata_end];

        if let Some(rr_type) = RecordType::from_code(rr_type) {
            let data = match rr_type {
                RecordType::A if rdata.len() >= 4 => {
                    let mut ip = [0u8; 4];
                    ip.copy_from_slice(&rdata[..4]);
                    RecordData::A(ip)
                }
                RecordType::Ptr => RecordData::Ptr(
                    Cursor {
                        data: packet,
                        pos: cursor.pos,
                    }
                    .read_name()?
                    .to_ascii_lowercase(),
                ),
                RecordType::Srv if rdata.len() >= 6 => {
                    let priority = u16::from_be_bytes([rdata[0], rdata[1]]);
                    let weight = u16::from_be_bytes([rdata[2], rdata[3]]);
                    let port = u16::from_be_bytes([rdata[4], rdata[5]]);
                    let target = Cursor {
                        data: packet,
                        pos: cursor.pos + 6,
                    }
                    .read_name()?
                    .to_ascii_lowercase();
                    RecordData::Srv {
                        priority,
                        weight,
                        port,
                        target,
                    }
                }
                RecordType::Txt => {
                    let mut values = Vec::new();
                    let mut pos = 0usize;
                    while pos < rdata.len() {
                        let len = rdata[pos] as usize;
                        pos += 1;
                        if pos + len > rdata.len() {
                            break; // malformed remainder: take what we have
                        }
                        values.push(String::from_utf8_lossy(&rdata[pos..pos + len]).into_owned());
                        pos += len;
                    }
                    RecordData::Txt(values)
                }
                _ => {
                    cursor.pos = rdata_end;
                    continue;
                }
            };
            records.push(ResourceRecord {
                name,
                record_type: rr_type,
                data,
            });
        }

        cursor.pos = rdata_end;
    }

    Ok(records)
}

/// Read cursor over a DNS packet, with name compression support.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn u8(&mut self) -> Result<u8> {
        let byte = *self
            .data
            .get(self.pos)
            .ok_or_else(|| Error::Dns("read past end of packet".into()))?;
        self.pos += 1;
        Ok(byte)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|e| *e <= self.data.len())
            .ok_or_else(|| Error::Dns("read past end of packet".into()))?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn skip(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    /// Read a (possibly compressed) domain name into a dotted string.
    fn read_name(&mut self) -> Result<String> {
        let mut labels: Vec<String> = Vec::new();
        let mut pos = self.pos;
        let mut jumps = 0usize;
        loop {
            let length = *self
                .data
                .get(pos)
                .ok_or_else(|| Error::Dns("name runs past packet end".into()))?;
            match length {
                0x00 => {
                    if jumps == 0 {
                        self.pos = pos + 1;
                    }
                    break;
                }
                len if len & 0xc0 == 0xc0 => {
                    // Compression pointer: jump, but only advance the original
                    // cursor past the pointer on the first one.
                    jumps += 1;
                    if jumps > 16 {
                        return Err(Error::Dns("name compression loop".into()));
                    }
                    let ptr_lo = *self
                        .data
                        .get(pos + 1)
                        .ok_or_else(|| Error::Dns("truncated compression pointer".into()))?;
                    if jumps == 1 {
                        self.pos = pos + 2;
                    }
                    pos = (((len & 0x3f) as usize) << 8) | ptr_lo as usize;
                }
                len if len & 0xc0 == 0x00 => {
                    let label = self
                        .data
                        .get(pos + 1..pos + 1 + len as usize)
                        .ok_or_else(|| Error::Dns("truncated name label".into()))?;
                    labels.push(String::from_utf8_lossy(label).into_owned());
                    pos += 1 + len as usize;
                }
                _ => return Err(Error::Dns("unsupported name label type".into())),
            }
        }

        let mut name = labels.join(".");
        while name.ends_with('.') {
            name.pop();
        }
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    #[test]
    fn encodes_ptr_query() {
        let query = encode_ptr_query(ESPHOME_SERVICE_TYPE).unwrap();
        let mut expected = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        expected.extend_from_slice(&[11]);
        expected.extend_from_slice(b"_esphomelib");
        expected.extend_from_slice(&[4]);
        expected.extend_from_slice(b"_tcp");
        expected.extend_from_slice(&[5]);
        expected.extend_from_slice(b"local");
        expected.push(0x00);
        expected.extend_from_slice(&[0x00, 0x0c, 0x00, 0x01]);
        assert_eq!(query, expected);
    }

    #[test]
    fn rejects_bad_labels() {
        assert!(encode_ptr_query("").is_ok()); // root
        let long = "a".repeat(64);
        assert!(encode_ptr_query(&long).is_err());
    }

    #[test]
    fn decodes_ptr_response() {
        // Hand-built minimal answer: PTR -> "device._esphomelib._tcp.local"
        let mut packet = Vec::new();
        packet.extend_from_slice(&[0x00, 0x00, 0x84, 0x00]); // id + response flags
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]); // 1 answer
        packet.extend_from_slice(&[11]);
        packet.extend_from_slice(b"_esphomelib");
        packet.extend_from_slice(&[4]);
        packet.extend_from_slice(b"_tcp");
        packet.extend_from_slice(&[5]);
        packet.extend_from_slice(b"local");
        packet.push(0x00);
        packet.extend_from_slice(&[0x00, 0x0c]); // PTR
        packet.extend_from_slice(&[0x00, 0x01]); // IN
        packet.extend_from_slice(&[0x00, 0x00, 0x11, 0x94]); // TTL
        let rdata = {
            let mut r = Vec::new();
            r.extend_from_slice(&[6]);
            r.extend_from_slice(b"device");
            r.extend_from_slice(&[11]);
            r.extend_from_slice(b"_esphomelib");
            r.extend_from_slice(&[4]);
            r.extend_from_slice(b"_tcp");
            r.extend_from_slice(&[5]);
            r.extend_from_slice(b"local");
            r.push(0x00);
            r
        };
        packet.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        packet.extend_from_slice(&rdata);

        let records = decode_message(&packet).unwrap();
        assert_eq!(records.len(), 1);
        match &records[0].data {
            RecordData::Ptr(target) => assert_eq!(target, "device._esphomelib._tcp.local"),
            other => panic!("expected PTR, got {other:?}"),
        }
    }

    #[test]
    fn decodes_real_device_reply() {
        // Capture from the Phase 0 test device: PTR + SRV + TXT records with
        // compressed owner names.
        let records = decode_message(fixtures::PTR_REPLY).unwrap();
        assert_eq!(records.len(), 4);

        let ptr = records
            .iter()
            .find(|r| r.record_type == RecordType::Ptr)
            .expect("PTR record");
        assert_eq!(ptr.name, "_esphomelib._tcp.local");
        match &ptr.data {
            RecordData::Ptr(target) => {
                assert_eq!(target, "eam-test-device._esphomelib._tcp.local")
            }
            other => panic!("expected PTR, got {other:?}"),
        }

        let srv = records
            .iter()
            .find(|r| r.record_type == RecordType::Srv)
            .expect("SRV record");
        assert_eq!(srv.name, "eam-test-device._esphomelib._tcp.local");
        match &srv.data {
            RecordData::Srv { port, target, .. } => {
                assert_eq!(*port, 6053);
                assert_eq!(target, "eam-test-device.local");
            }
            other => panic!("expected SRV, got {other:?}"),
        }

        let txt = records
            .iter()
            .find(|r| r.record_type == RecordType::Txt)
            .expect("TXT record");
        match &txt.data {
            RecordData::Txt(values) => {
                assert!(values.contains(&"friendly_name=EAM Test Device".to_string()));
                assert!(values.contains(&"version=2026.10.2".to_string()));
                assert!(values.contains(&"platform=ESP32".to_string()));
                assert!(values.contains(&"network=wifi".to_string()));
            }
            other => panic!("expected TXT, got {other:?}"),
        }
        assert!(records.iter().all(|r| !r.name.is_empty()));
    }

    #[test]
    fn decodes_real_device_a_record() {
        let records = decode_message(fixtures::A_REPLY).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "eam-test-device.local");
        assert_eq!(records[0].data, RecordData::A([10, 60, 109, 61]));
    }

    #[test]
    fn rejects_short_packet() {
        assert!(decode_message(&[0, 1]).is_err());
    }

    #[test]
    fn ignores_unknown_record_types() {
        let mut packet = Vec::new();
        packet.extend_from_slice(&[0x00, 0x00, 0x84, 0x00]);
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
        packet.push(0x00); // root owner name
        packet.extend_from_slice(&[0x00, 0x21]); // AAAA (unsupported)
        packet.extend_from_slice(&[0x00, 0x01]);
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        packet.extend_from_slice(&(4u16).to_be_bytes());
        packet.extend_from_slice(&[0, 0, 0, 0]);
        assert!(decode_message(&packet).unwrap().is_empty());
    }
}
