use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{cmp, fs, io};

use anyhow::Context as _;
use bstr::BStr;
use clap::Parser as _;
use tracing_subscriber::prelude::*;

use crate::keymap::{LAYER_DATA_LEN, PROFILE_DATA_LEN};
use crate::{keymap, layout, scancode};

const GET_PRODUCT_NAME: u16 = 0x1001;
const GET_KEYBOARD_LAYOUT: u16 = 0x1002;
const GET_BOOT_LOADER_VERSION: u16 = 0x1003; // ?
const GET_MODEL_NAME: u16 = 0x1005;
const GET_SERIAL_NUMBER: u16 = 0x1007;
const GET_FIRMWARE_VERSION: u16 = 0x100b;

const GET_DIPSW: u16 = 0x1103;

const GET_CURRENT_PROFILE: u16 = 0x1101;
const SET_CURRENT_PROFILE: u16 = 0x1101;

#[derive(Clone, Debug, clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, clap::Subcommand)]
enum Command {
    Info(InfoArgs),
    ReadProfile(ReadProfileArgs),
    WriteProfile(WriteProfileArgs),
    ShowProfile(ShowProfileArgs),
}

#[derive(Clone, Debug, clap::Args)]
struct ConnectionArgs {
    /// Path to device file to communicate over
    #[arg(long, default_value = "/dev/hidraw1")]
    device: PathBuf,
}

pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(io::stderr))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match &cli.command {
        Command::Info(args) => run_info(args),
        Command::ReadProfile(args) => run_read_profile(args),
        Command::WriteProfile(args) => run_write_profile(args),
        Command::ShowProfile(args) => run_show_profile(args),
    }
}

/// Print information about the connected keyboard
#[derive(Clone, Debug, clap::Args)]
struct InfoArgs {
    #[command(flatten)]
    connection: ConnectionArgs,
    /// Show fetched data without interpreting
    #[arg(long)]
    raw: bool,
}

fn run_info(args: &InfoArgs) -> anyhow::Result<()> {
    let mut dev = open_device(&args.connection)?;
    if args.raw {
        for code in 0x1000..0x1010 {
            let message = get_simple(&mut dev, code)?;
            println!("{code:04x}: {:?}", &BStr::new(&message[3..]));
        }
    } else {
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

        let message = get_simple(&mut dev, GET_DIPSW)?;
        println!("DIP Sw: {:06b}", pack_dipsw(&message[3..9]));
        let index = get_current_profile(&mut dev)?;
        println!("Current profile: {index}");
    }
    Ok(())
}

/// Fetch keymap profile and save to file
#[derive(Clone, Debug, clap::Args)]
struct ReadProfileArgs {
    #[command(flatten)]
    connection: ConnectionArgs,
    /// Output file [default: stdout]
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Output raw binary data
    #[arg(long)]
    raw: bool,
    /// Profile index to fetch [default: current profile]
    #[arg(long, value_parser = clap::value_parser!(u16).range(0..4))]
    index: Option<u16>,
}

fn run_read_profile(args: &ReadProfileArgs) -> anyhow::Result<()> {
    let mut dev = open_device(&args.connection)?;
    let data = maybe_switch_profile(&mut dev, args.index, |dev| {
        read_data(dev, 0, PROFILE_DATA_LEN.try_into().unwrap())
    })?;
    let serialized = if args.raw {
        data
    } else {
        keymap::serialize_to_toml_string(&data).into_bytes()
    };
    if let Some(path) = &args.output {
        fs::write(path, serialized)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        io::stdout().write_all(&serialized)?;
    }
    Ok(())
}

/// Load keymap profile from file
#[derive(Clone, Debug, clap::Args)]
struct WriteProfileArgs {
    #[command(flatten)]
    connection: ConnectionArgs,
    /// Input file [default: stdin]
    #[arg(short, long)]
    input: Option<PathBuf>,
    /// Profile index to write [default: current profile]
    #[arg(long, value_parser = clap::value_parser!(u16).range(0..4))]
    index: Option<u16>,
}

fn run_write_profile(args: &WriteProfileArgs) -> anyhow::Result<()> {
    let data = read_profile_data(args.input.as_deref())?;
    let mut dev = open_device(&args.connection)?;
    maybe_switch_profile(&mut dev, args.index, |dev| write_data(dev, 0, &data))?;
    Ok(())
}

/// Show keymap profile data
#[derive(Clone, Debug, clap::Args)]
struct ShowProfileArgs {
    /// Input file [default: stdin]
    #[arg(short, long)]
    input: Option<PathBuf>,
    /// Print each row ignoring physical layout
    #[arg(long)]
    no_layout: bool,
}

fn run_show_profile(args: &ShowProfileArgs) -> anyhow::Result<()> {
    let profile_data = read_profile_data(args.input.as_deref())?;
    for (i, data) in profile_data.chunks_exact(LAYER_DATA_LEN).enumerate() {
        println!("Layer #{i}");
        let scancodes: Vec<_> = data
            .chunks_exact(2)
            .map(|d| u16::from_be_bytes(d.try_into().unwrap()))
            .collect();
        if args.no_layout {
            for codes in scancodes.chunks(15) {
                println!("  {codes:04x?}");
            }
        } else {
            let widths_map = &layout::US_LAYOUT_WIDTHS_MAP;
            for (codes, widths) in scancodes.chunks(15).zip(widths_map) {
                let formatted_codes =
                    layout::format_row(widths, codes.iter().map(|n| format!("{n:04x}")));
                let formatted_labels = layout::format_row(
                    widths,
                    codes
                        .iter()
                        .map(|n| scancode::scancode_to_label(*n).unwrap_or("")),
                );
                println!("  {formatted_codes}");
                println!("  {formatted_labels}");
            }
        }
    }
    Ok(())
}

fn read_profile_data(maybe_path: Option<&Path>) -> anyhow::Result<Vec<u8>> {
    let data = if let Some(path) = &maybe_path {
        fs::read(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        let mut buf = Vec::with_capacity(PROFILE_DATA_LEN);
        io::stdin().read_to_end(&mut buf)?;
        buf
    };
    anyhow::ensure!(
        data.len() == PROFILE_DATA_LEN,
        "unexpected profile data length"
    );
    Ok(data)
}

fn maybe_switch_profile<D: Read + Write, O>(
    dev: &mut D,
    profile_index: Option<u16>,
    f: impl FnOnce(&mut D) -> io::Result<O>,
) -> io::Result<O> {
    let old_profile_index = if let Some(index) = profile_index {
        let old_index = get_current_profile(dev)?;
        set_current_profile(dev, index)?;
        Some(old_index)
    } else {
        None
    };
    let res = f(dev);
    if let Some(index) = old_profile_index {
        set_current_profile(dev, index)?;
    }
    res
}

fn open_device(args: &ConnectionArgs) -> anyhow::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(&args.device)
        .with_context(|| format!("failed to open device {}", args.device.display()))
}

#[tracing::instrument(skip(dev))]
fn get_simple<D: Read + Write>(dev: &mut D, command: u16) -> io::Result<[u8; 32]> {
    let mut message = [0; 32];
    message[0] = 0x02;
    message[1..3].copy_from_slice(&command.to_be_bytes());
    tracing::trace!(?message, "write");
    dev.write_all(&message)?;
    dev.read_exact(&mut message)?;
    tracing::trace!(?message, "read");
    Ok(message)
}

#[tracing::instrument(skip(dev))]
fn get_current_profile<D: Read + Write>(dev: &mut D) -> io::Result<u16> {
    let message = get_simple(dev, GET_CURRENT_PROFILE)?;
    Ok(u16::from_be_bytes(message[3..5].try_into().unwrap()))
}

#[tracing::instrument(skip(dev))]
fn set_current_profile<D: Read + Write>(dev: &mut D, id: u16) -> io::Result<()> {
    let mut message = [0; 32];
    message[0] = 0x03;
    message[1..3].copy_from_slice(&SET_CURRENT_PROFILE.to_be_bytes());
    message[3..5].copy_from_slice(&id.to_be_bytes());
    tracing::trace!(?message, "write");
    dev.write_all(&message)?;
    // TODO: process response
    dev.read_exact(&mut message)?;
    tracing::trace!(?message, "read");
    dev.read_exact(&mut message)?;
    tracing::trace!(?message, "read");
    Ok(())
}

const MAX_DATA_CHUNK_LEN: u16 = 26; // or 28?

// TODO: Is this a generic function or specific to the profile data?
#[tracing::instrument(skip(dev))]
fn read_data<D: Read + Write>(dev: &mut D, start: u16, len: u16) -> io::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(len.into());
    for offset in (0..len).step_by(MAX_DATA_CHUNK_LEN.into()) {
        let n: u8 = cmp::min(MAX_DATA_CHUNK_LEN, len - offset)
            .try_into()
            .unwrap();
        let mut message = [0; 32];
        message[0] = 0x12;
        message[1..3].copy_from_slice(&(start + offset).to_be_bytes());
        message[3] = n;
        tracing::trace!(?message, "write");
        dev.write_all(&message)?;
        dev.read_exact(&mut message)?;
        tracing::trace!(?message, "read");
        data.extend_from_slice(&message[4..][..n.into()]);
    }
    Ok(data)
}

// TODO: Is this a generic function or specific to the profile data?
#[tracing::instrument(skip(dev, data))]
fn write_data<D: Read + Write>(dev: &mut D, start: u16, data: &[u8]) -> io::Result<()> {
    for (i, chunk) in data.chunks(MAX_DATA_CHUNK_LEN.into()).enumerate() {
        let offset: u16 = MAX_DATA_CHUNK_LEN * u16::try_from(i).unwrap();
        let mut message = [0; 32];
        message[0] = 0x13;
        message[1..3].copy_from_slice(&(start + offset).to_be_bytes());
        message[3] = chunk.len().try_into().unwrap();
        message[4..][..chunk.len()].copy_from_slice(chunk);
        tracing::trace!(?message, "write");
        dev.write_all(&message)?;
        dev.read_exact(&mut message)?;
        tracing::trace!(?message, "read");
        // TODO: process response
    }
    Ok(())
}

fn pack_dipsw(data: &[u8]) -> u8 {
    // dip-sw bit per byte (not packed), from MSB for pretty printing
    data.iter().fold(0, |acc, v| (acc << 1) | (v & 1))
}

fn truncate_nul_str(data: &[u8]) -> &BStr {
    if let Some(p) = data.iter().position(|&c| c == b'\0') {
        BStr::new(&data[..p])
    } else {
        BStr::new(data)
    }
}
