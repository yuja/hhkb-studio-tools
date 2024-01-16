use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::{env, io};

use bstr::BStr;

const GET_PRODUCT_NAME: u16 = 0x1001;
const GET_KEYBOARD_LAYOUT: u16 = 0x1002;
const GET_BOOT_LOADER_VERSION: u16 = 0x1003; // ?
const GET_MODEL_NAME: u16 = 0x1005;
const GET_SERIAL_NUMBER: u16 = 0x1007;
const GET_FIRMWARE_VERSION: u16 = 0x100b;

const GET_DIPSW: u16 = 0x1103;

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
