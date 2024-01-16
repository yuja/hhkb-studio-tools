use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::{cmp, env, io};

use bstr::BStr;

const GET_PRODUCT_NAME: u16 = 0x1001;
const GET_KEYBOARD_LAYOUT: u16 = 0x1002;
const GET_BOOT_LOADER_VERSION: u16 = 0x1003; // ?
const GET_MODEL_NAME: u16 = 0x1005;
const GET_SERIAL_NUMBER: u16 = 0x1007;
const GET_FIRMWARE_VERSION: u16 = 0x100b;

const GET_DIPSW: u16 = 0x1103;

const GET_CURRENT_PROFILE: u16 = 0x1101;
const SET_CURRENT_PROFILE: u16 = 0x1101;

fn main() -> anyhow::Result<()> {
    let dev_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/hidraw1".to_owned());
    let mut dev = OpenOptions::new().read(true).write(true).open(dev_path)?;

    let message = get_simple(&mut dev, GET_PRODUCT_NAME)?;
    println!("Product name: {}", truncate_nul_str(&message[3..]));
    let message = get_simple(&mut dev, GET_MODEL_NAME)?;
    println!("Model name: {}", truncate_nul_str(&message[3..]));
    let message = get_simple(&mut dev, GET_SERIAL_NUMBER)?;
    println!("Serial number: {}", truncate_nul_str(&message[3..]));
    let message = get_simple(&mut dev, GET_KEYBOARD_LAYOUT)?;
    println!("Keyboard layout: {}", truncate_nul_str(&message[3..]));
    let message = get_simple(&mut dev, GET_BOOT_LOADER_VERSION)?;
    println!("Boot loader version?: {}", truncate_nul_str(&message[3..]));
    let message = get_simple(&mut dev, GET_FIRMWARE_VERSION)?;
    println!("Firmware version: {}", truncate_nul_str(&message[3..]));

    for code in 0x1000..0x1010 {
        let message = get_simple(&mut dev, code)?;
        println!("{code:04x}: {:?}", truncate_nul_str(&message[3..]));
    }

    let message = get_simple(&mut dev, GET_DIPSW)?;
    println!(
        "DIP Sw: {:?}",
        parse_dipsw(&message[3..9].try_into().unwrap())
    );

    let original_profile = get_current_profile(&mut dev)?;
    println!("Current profile: {original_profile}");
    for profile_id in 0..4 {
        set_current_profile(&mut dev, profile_id)?;
        let profiles = read_profile_data(&mut dev)?;
        println!("Profile #{profile_id}");
        for (layer_id, key_codes) in profiles.iter().enumerate() {
            println!("  Layer #{layer_id}");
            for chunk in key_codes.chunks(15) {
                println!("    {chunk:04x?}");
            }
        }
    }
    set_current_profile(&mut dev, original_profile)?;

    Ok(())
}

fn get_simple<D: Read + Write>(dev: &mut D, command: u16) -> io::Result<[u8; 32]> {
    let mut message = [0; 32];
    message[0] = 0x02;
    message[1..3].copy_from_slice(&command.to_be_bytes());
    dev.write_all(&message)?;
    dev.read_exact(&mut message)?;
    Ok(message)
}

fn get_current_profile<D: Read + Write>(dev: &mut D) -> io::Result<u16> {
    let message = get_simple(dev, GET_CURRENT_PROFILE)?;
    Ok(u16::from_be_bytes(message[3..5].try_into().unwrap()))
}

fn set_current_profile<D: Read + Write>(dev: &mut D, id: u16) -> io::Result<()> {
    let mut message = [0; 32];
    message[0] = 0x03;
    message[1..3].copy_from_slice(&SET_CURRENT_PROFILE.to_be_bytes());
    message[3..5].copy_from_slice(&id.to_be_bytes());
    dev.write_all(&message)?;
    // TODO: process response
    dev.read_exact(&mut message)?;
    dev.read_exact(&mut message)?;
    Ok(())
}

// TODO: parse keymap
fn read_profile_data<D: Read + Write>(dev: &mut D) -> io::Result<Vec<Vec<u16>>> {
    const PROFILE_DATA_LEN: u16 = 0xf0;
    let mut layers = Vec::with_capacity(4);
    for layer_id in 0..4 {
        let data = read_data(dev, layer_id * PROFILE_DATA_LEN, PROFILE_DATA_LEN)?;
        let key_codes = data
            .chunks_exact(2)
            .map(|d| u16::from_be_bytes(d.try_into().unwrap()))
            .collect();
        layers.push(key_codes);
    }
    Ok(layers)
}

// TODO: Is this a generic function or specific to the profile data?
fn read_data<D: Read + Write>(dev: &mut D, start: u16, len: u16) -> io::Result<Vec<u8>> {
    const MAX_CHUNK_LEN: u16 = 0x1b;
    let mut data = Vec::with_capacity(len.into());
    for offset in (0..len).step_by(MAX_CHUNK_LEN.into()) {
        let n: u8 = cmp::min(MAX_CHUNK_LEN, len - offset).try_into().unwrap();
        let mut message = [0; 32];
        message[0] = 0x12;
        message[1..3].copy_from_slice(&(start + offset).to_be_bytes());
        message[3] = n;
        dev.write_all(&message)?;
        dev.read_exact(&mut message)?;
        data.extend_from_slice(&message[4..][..n.into()]);
    }
    Ok(data)
}

fn parse_dipsw(data: &[u8; 6]) -> [bool; 6] {
    // dip-sw bit per byte (not packed)
    data.map(|v| v != 0)
}

fn truncate_nul_str(data: &[u8]) -> &BStr {
    if let Some(p) = data.iter().position(|&c| c == b'\0') {
        BStr::new(&data[..p])
    } else {
        BStr::new(data)
    }
}
