//! Utility to process keymap data.

use std::fmt::Write as _;

pub const LAYER_DATA_LEN: usize = 0xf0;
pub const PROFILE_DATA_LEN: usize = LAYER_DATA_LEN * 4;

pub fn serialize_to_toml_string(profile_data: &[u8]) -> String {
    assert_eq!(profile_data.len(), PROFILE_DATA_LEN);
    let mut buffer = String::new();
    for layer_data in profile_data.chunks_exact(LAYER_DATA_LEN) {
        buffer.push_str("[[layers]]\nscancodes = ");
        serialize_layer_scancodes_to_toml_string(&mut buffer, layer_data);
        buffer.push('\n');
    }
    buffer.truncate(buffer.trim_end_matches('\n').len() + 1);
    debug_assert!(buffer.parse::<toml::Table>().is_ok());
    buffer
}

fn serialize_layer_scancodes_to_toml_string(buffer: &mut String, layer_data: &[u8]) {
    let scancodes = layer_data
        .chunks_exact(2)
        .map(|d| u16::from_be_bytes(d.try_into().unwrap()));
    // Build formatted array split per keyboard raw.
    buffer.push_str("[\n");
    for (i, code) in scancodes.enumerate() {
        if i % 15 == 0 {
            buffer.push_str("  ");
        }
        write!(buffer, "0x{code:04x},").unwrap();
        buffer.push(if i % 15 == 14 { '\n' } else { ' ' });
    }
    buffer.push_str("]\n");
}

pub fn parse_toml_string(serialized: &str) -> anyhow::Result<Vec<u8>> {
    let doc: toml::Table = serialized.parse()?;
    let layers = doc
        .get("layers")
        .ok_or_else(|| anyhow::anyhow!("layers not found"))?
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected type of layers"))?;
    anyhow::ensure!(layers.len() == 4, "unexpected number of layers");

    let mut profile_data = Vec::with_capacity(PROFILE_DATA_LEN);
    for layer in layers {
        let scancodes: Vec<u16> = layer
            .get("scancodes")
            .ok_or_else(|| anyhow::anyhow!("scancodes not found"))?
            .clone()
            .try_into()?;
        anyhow::ensure!(
            scancodes.len() == LAYER_DATA_LEN / 2,
            "unexpected number of scancodes"
        );
        for code in &scancodes {
            profile_data.extend(code.to_be_bytes());
        }
    }

    assert_eq!(profile_data.len(), PROFILE_DATA_LEN);
    Ok(profile_data)
}
