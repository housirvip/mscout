use std::path::Path;

use anyhow::Result;
use mscout_core::platform::{NativeProcess, ProcessMemory};
use serde_json::json;

use crate::commands::scan::{parse_address, parse_scan_value, parse_value_type};
use crate::output::Output;
use crate::session;
use crate::state::CliState;

/// Read memory values at an address.
pub fn read_memory(
    addr_str: &str,
    type_str: &str,
    count: usize,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let type_size = value_type.size().max(1);

    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut results = Vec::new();
    for i in 0..count {
        let addr = address + i * type_size;
        let mut buf = vec![0u8; type_size];
        process.read(addr, &mut buf).map_err(|e| anyhow::anyhow!("{e}"))?;
        let value = format_typed_value(&buf, value_type);
        results.push(json!({"address": format!("0x{:X}", addr), "value": value}));
    }

    if out.json {
        out.success("read", results);
    } else {
        for r in &results {
            println!("{}: {}", r["address"].as_str().unwrap(), r["value"].as_str().unwrap());
        }
    }
    Ok(())
}

/// Write a value to a memory address.
pub fn write_memory(
    addr_str: &str,
    value_str: &str,
    type_str: &str,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let scan_value = parse_scan_value(value_str, value_type)?;
    let bytes = scan_value.to_bytes();

    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    process.write(address, &bytes).map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        out.success("write", json!({"address": format!("0x{:X}", address), "value": value_str, "bytes": bytes.len()}));
    } else {
        println!("Wrote {} bytes to 0x{:X}", bytes.len(), address);
    }
    Ok(())
}

/// Hex dump of memory at address.
pub fn hex_dump(
    addr_str: &str,
    len: usize,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let address = parse_address(addr_str)?;

    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut buf = vec![0u8; len];
    let bytes_read = process.read(address, &mut buf).map_err(|e| anyhow::anyhow!("{e}"))?;
    buf.truncate(bytes_read);

    if out.json {
        let hex_str = buf.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        out.success("hex", json!({
            "address": format!("0x{:X}", address),
            "length": bytes_read,
            "hex": hex_str,
        }));
    } else {
        print_hex_dump(address, &buf);
    }
    Ok(())
}

// ─── REPL variants ─────────────────────────────────────────────────────────

pub fn read_memory_repl(
    state: &CliState,
    addr_str: &str,
    type_str: &str,
    count: usize,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let type_size = value_type.size().max(1);

    let mut results = Vec::new();
    for i in 0..count {
        let addr = address + i * type_size;
        let mut buf = vec![0u8; type_size];
        process.read(addr, &mut buf).map_err(|e| anyhow::anyhow!("{e}"))?;
        let value = format_typed_value(&buf, value_type);
        results.push(json!({"address": format!("0x{:X}", addr), "value": value}));
    }

    if out.json {
        out.success("read", results);
    } else {
        for r in &results {
            println!("{}: {}", r["address"].as_str().unwrap(), r["value"].as_str().unwrap());
        }
    }
    Ok(())
}

pub fn write_memory_repl(
    state: &CliState,
    addr_str: &str,
    value_str: &str,
    type_str: &str,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let scan_value = parse_scan_value(value_str, value_type)?;
    let bytes = scan_value.to_bytes();

    process.write(address, &bytes).map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        out.success("write", json!({"address": format!("0x{:X}", address), "value": value_str, "bytes": bytes.len()}));
    } else {
        println!("Wrote {} bytes to 0x{:X}", bytes.len(), address);
    }
    Ok(())
}

pub fn hex_dump_repl(
    state: &CliState,
    addr_str: &str,
    len: usize,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let address = parse_address(addr_str)?;

    let mut buf = vec![0u8; len];
    let bytes_read = process.read(address, &mut buf).map_err(|e| anyhow::anyhow!("{e}"))?;
    buf.truncate(bytes_read);

    if out.json {
        let hex_str = buf.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        out.success("hex", json!({
            "address": format!("0x{:X}", address),
            "length": bytes_read,
            "hex": hex_str,
        }));
    } else {
        print_hex_dump(address, &buf);
    }
    Ok(())
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn format_typed_value(data: &[u8], vt: mscout_core::scanner::ValueType) -> String {
    use mscout_core::scanner::ValueType;
    macro_rules! fmt {
        ($ty:ty) => {{
            let size = std::mem::size_of::<$ty>();
            if data.len() >= size {
                let val = <$ty>::from_le_bytes(data[..size].try_into().unwrap());
                format!("{}", val)
            } else {
                "???".to_string()
            }
        }};
    }
    match vt {
        ValueType::I8 => fmt!(i8),
        ValueType::I16 => fmt!(i16),
        ValueType::I32 => fmt!(i32),
        ValueType::I64 => fmt!(i64),
        ValueType::U8 => fmt!(u8),
        ValueType::U16 => fmt!(u16),
        ValueType::U32 => fmt!(u32),
        ValueType::U64 => fmt!(u64),
        ValueType::F32 => fmt!(f32),
        ValueType::F64 => fmt!(f64),
        ValueType::ByteArray => {
            data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
        }
    }
}

fn print_hex_dump(base: usize, data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = base + i * 16;
        // Offset
        print!("{:012X}  ", offset);
        // Hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02X} ", byte);
            if j == 7 {
                print!(" ");
            }
        }
        // Padding for short last line
        if chunk.len() < 16 {
            for j in chunk.len()..16 {
                print!("   ");
                if j == 7 {
                    print!(" ");
                }
            }
        }
        // ASCII
        print!(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}
