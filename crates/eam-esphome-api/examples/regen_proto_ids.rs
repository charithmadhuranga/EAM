//! Regenerate `src/message.rs` (the message ID table) from the official
//! ESPHome API definitions.
//!
//! ```sh
//! cargo run -p eam-esphome-api --example regen_proto_ids
//! ```
//!
//! Keeps the ID table in sync without hand-editing. No protoc required: this
//! only parses `option (id) = N` entries from `proto/api.proto`.

use std::fmt::Write as _;

fn pascal_case(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/proto/api.proto");
    let src = std::fs::read_to_string(path)?;

    // Walk the file tracking the current `message NAME {` block, collecting
    // `option (id) = N;` where it appears.
    let mut ids: Vec<(u32, String)> = Vec::new();
    let mut current: Option<String> = None;
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("message ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            current = Some(name);
        } else if t.starts_with('}') {
            current = None;
        } else if let (Some(name), Some(rest)) = (&current, t.strip_prefix("option (id) = ")) {
            if let Some(num) = rest.strip_suffix(';') {
                if let Ok(id) = num.trim().parse::<u32>() {
                    ids.push((id, name.clone()));
                }
            }
        }
    }

    ids.sort_by_key(|(id, _)| *id);
    ids.dedup_by_key(|(id, _)| *id);

    let mut out = String::new();
    out.push_str("//! ESPHome Native API message type identifiers.\n");
    out.push_str("//!\n");
    out.push_str("//! Message IDs are protocol constants (see `option (id) = N` in the official\n");
    out.push_str("//! ESPHome `api.proto`). Regenerate with:\n");
    out.push_str("//!   `cargo run -p eam-esphome-api --example regen_proto_ids`\n");
    out.push_str("//!\n");
    out.push_str("//! Wire format (per ESPHome Native API docs):\n");
    out.push_str("//!   [0x00][varint payload_length][varint message_type][protobuf payload]\n\n#![allow(missing_docs)]\n\n");
    out.push_str("/// A message type from the ESPHome Native API.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum MessageType {\n");
    for (id, name) in &ids {
        write!(out, "    {} = {},", pascal_case(name), id)?;
        out.push('\n');
    }
    out.push_str("}\n\nimpl MessageType {\n");
    writeln!(
        out,
        "    /// All message IDs defined by the ESPHome API protocol (v2026.9.1)."
    )?;
    writeln!(out, "    pub const ALL: [Self; {}] = [", ids.len())?;
    for (_, name) in &ids {
        writeln!(out, "        Self::{},", pascal_case(name))?;
    }
    out.push_str("    ];\n\n");
    out.push_str("    /// Protocol ID as used on the wire.\n");
    out.push_str("    #[must_use]\n");
    out.push_str("    pub const fn id(self) -> u16 {\n");
    out.push_str("        self as u16\n");
    out.push_str("    }\n\n");
    out.push_str("    /// Look up a message type by wire ID. Returns `None` for IDs this\n");
    out.push_str("    /// protocol version does not define.\n");
    out.push_str("    #[must_use]\n");
    out.push_str("    pub const fn from_id(id: u16) -> Option<Self> {\n");
    out.push_str("        Some(match id {\n");
    for (id, name) in &ids {
        writeln!(out, "            {id} => Self::{},", pascal_case(name))?;
    }
    out.push_str("            _ => return None,\n");
    out.push_str("        })\n");
    out.push_str("    }\n}\n");

    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/src/message.rs"), out)?;
    println!("wrote src/message.rs ({} message types)", ids.len());
    Ok(())
}
