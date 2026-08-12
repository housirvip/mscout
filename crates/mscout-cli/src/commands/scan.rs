use std::path::Path;

use anyhow::{Result, bail};
use mscout_core::{
    platform::{NativeProcess, ProcessMemory},
    scanner::{ScanCondition, ScanSession, ScanValue, ValueType, BytePattern},
};
use serde_json::json;

use crate::output::Output;
use crate::session::{self, ScanState};
use crate::state::CliState;

// ─── Parsing helpers ───────────────────────────────────────────────────────

pub fn parse_value_type(s: &str) -> Result<ValueType> {
    match s.to_lowercase().as_str() {
        "i8" => Ok(ValueType::I8),
        "i16" => Ok(ValueType::I16),
        "i32" => Ok(ValueType::I32),
        "i64" => Ok(ValueType::I64),
        "u8" => Ok(ValueType::U8),
        "u16" => Ok(ValueType::U16),
        "u32" => Ok(ValueType::U32),
        "u64" => Ok(ValueType::U64),
        "f32" => Ok(ValueType::F32),
        "f64" => Ok(ValueType::F64),
        "aob" | "bytes" | "bytearray" => Ok(ValueType::ByteArray),
        _ => bail!("Unknown type: {s}. Use: i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|aob"),
    }
}

pub fn parse_condition(s: &str) -> Result<ScanCondition> {
    match s.to_lowercase().as_str() {
        "eq" | "exact" => Ok(ScanCondition::Exact),
        "gt" | "greaterthan" => Ok(ScanCondition::GreaterThan),
        "lt" | "lessthan" => Ok(ScanCondition::LessThan),
        "range" | "between" => Ok(ScanCondition::Between),
        "changed" => Ok(ScanCondition::Changed),
        "unchanged" => Ok(ScanCondition::Unchanged),
        "increased" => Ok(ScanCondition::Increased),
        "decreased" => Ok(ScanCondition::Decreased),
        "increased_by" | "increasedby" => Ok(ScanCondition::IncreasedBy),
        "decreased_by" | "decreasedby" => Ok(ScanCondition::DecreasedBy),
        "unknown" => Ok(ScanCondition::Unknown),
        _ => bail!("Unknown condition: {s}. Use: eq|gt|lt|range|changed|unchanged|increased|decreased|increased_by|decreased_by|unknown"),
    }
}

pub fn parse_scan_value(s: &str, value_type: ValueType) -> Result<ScanValue> {
    match value_type {
        ValueType::I8 => Ok(ScanValue::I8(s.parse().map_err(|e| anyhow::anyhow!("Invalid i8: {e}"))?)),
        ValueType::I16 => Ok(ScanValue::I16(s.parse().map_err(|e| anyhow::anyhow!("Invalid i16: {e}"))?)),
        ValueType::I32 => Ok(ScanValue::I32(s.parse().map_err(|e| anyhow::anyhow!("Invalid i32: {e}"))?)),
        ValueType::I64 => Ok(ScanValue::I64(s.parse().map_err(|e| anyhow::anyhow!("Invalid i64: {e}"))?)),
        ValueType::U8 => Ok(ScanValue::U8(s.parse().map_err(|e| anyhow::anyhow!("Invalid u8: {e}"))?)),
        ValueType::U16 => Ok(ScanValue::U16(s.parse().map_err(|e| anyhow::anyhow!("Invalid u16: {e}"))?)),
        ValueType::U32 => Ok(ScanValue::U32(s.parse().map_err(|e| anyhow::anyhow!("Invalid u32: {e}"))?)),
        ValueType::U64 => Ok(ScanValue::U64(s.parse().map_err(|e| anyhow::anyhow!("Invalid u64: {e}"))?)),
        ValueType::F32 => Ok(ScanValue::F32(s.parse().map_err(|e| anyhow::anyhow!("Invalid f32: {e}"))?)),
        ValueType::F64 => Ok(ScanValue::F64(s.parse().map_err(|e| anyhow::anyhow!("Invalid f64: {e}"))?)),
        ValueType::ByteArray => {
            let pattern = BytePattern::parse(s).map_err(|e| anyhow::anyhow!("Invalid byte pattern: {e}"))?;
            Ok(ScanValue::Pattern(pattern))
        }
    }
}

pub fn parse_address(s: &str) -> Result<usize> {
    let s = s.trim();
    let s = s.strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    usize::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!("Invalid address '{s}': {e}"))
}

// ─── Subcommand mode ───────────────────────────────────────────────────────

pub fn first_scan(
    value_str: &str,
    type_str: &str,
    cond_str: &str,
    value2_str: Option<&str>,
    filter_perm: Option<&str>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let value_type = parse_value_type(type_str)?;
    let condition = parse_condition(cond_str)?;

    let (mut session_file, spath) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let value = if condition == ScanCondition::Unknown {
        None
    } else {
        Some(parse_scan_value(value_str, value_type)?)
    };
    let value2 = value2_str.map(|v| parse_scan_value(v, value_type)).transpose()?;

    let mut regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Filter by permissions
    if let Some(perm) = filter_perm {
        regions.retain(|r| {
            (!perm.contains('r') || r.readable)
                && (!perm.contains('w') || r.writable)
                && (!perm.contains('x') || r.executable)
        });
    }

    let alignment = value_type.size().max(1);
    let scan_session = ScanSession::first_scan(
        &process, value_type, condition, value, value2, &regions, alignment,
    ).map_err(|e| anyhow::anyhow!("{e}"))?;

    let found = scan_session.result_count();

    // Persist scan state
    let (vt, align, addrs, prev, hist, regs) = scan_session.into_parts();
    session_file.scan_state = Some(ScanState {
        value_type: vt,
        alignment: align,
        addresses: addrs,
        previous_values: prev,
        history: hist,
        regions: regs,
    });
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("scan", json!({"found": found, "type": type_str, "cond": cond_str}));
    } else {
        println!("Found {} matches (type={}, cond={})", found, type_str, cond_str);
    }
    Ok(())
}

pub fn next_scan(
    value_str: Option<&str>,
    cond_str: &str,
    value2_str: Option<&str>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let (mut session_file, spath) = session::load_session(session_path)?;
    let scan_state = session_file.scan_state.take()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress. Run `mscout scan` first."))?;

    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let condition = parse_condition(cond_str)?;
    let value_type = scan_state.value_type;

    let value = value_str
        .map(|v| parse_scan_value(v, value_type))
        .transpose()?;
    let value2 = value2_str
        .map(|v| parse_scan_value(v, value_type))
        .transpose()?;

    let mut scan_session = ScanSession::from_parts(
        scan_state.value_type,
        scan_state.alignment,
        scan_state.addresses,
        scan_state.previous_values,
        scan_state.history,
        scan_state.regions,
    );

    scan_session.next_scan(&process, condition, value, value2)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let found = scan_session.result_count();

    let (vt, align, addrs, prev, hist, regs) = scan_session.into_parts();
    session_file.scan_state = Some(ScanState {
        value_type: vt,
        alignment: align,
        addresses: addrs,
        previous_values: prev,
        history: hist,
        regions: regs,
    });
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("next", json!({"found": found}));
    } else {
        println!("Narrowed to {} matches", found);
    }
    Ok(())
}

pub fn undo_scan(session_path: Option<&Path>, out: &Output) -> Result<()> {
    let (mut session_file, spath) = session::load_session(session_path)?;
    let scan_state = session_file.scan_state.take()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress."))?;

    let mut scan_session = ScanSession::from_parts(
        scan_state.value_type,
        scan_state.alignment,
        scan_state.addresses,
        scan_state.previous_values,
        scan_state.history,
        scan_state.regions,
    );

    if !scan_session.undo() {
        bail!("Nothing to undo (no history).");
    }

    let found = scan_session.result_count();
    let (vt, align, addrs, prev, hist, regs) = scan_session.into_parts();
    session_file.scan_state = Some(ScanState {
        value_type: vt,
        alignment: align,
        addresses: addrs,
        previous_values: prev,
        history: hist,
        regions: regs,
    });
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("undo", json!({"found": found}));
    } else {
        println!("Undo: {} matches", found);
    }
    Ok(())
}

pub fn reset_scan(session_path: Option<&Path>, out: &Output) -> Result<()> {
    let (mut session_file, spath) = session::load_session(session_path)?;
    session_file.scan_state = None;
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("reset", json!({"reset": true}));
    } else {
        println!("Scan state cleared.");
    }
    Ok(())
}

pub fn results(
    offset: usize,
    limit: usize,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let (session_file, _spath) = session::load_session(session_path)?;
    let scan_state = session_file.scan_state.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress."))?;

    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let scan_session = ScanSession::from_parts(
        scan_state.value_type,
        scan_state.alignment,
        scan_state.addresses.clone(),
        scan_state.previous_values.clone(),
        scan_state.history.clone(),
        scan_state.regions.clone(),
    );

    let total = scan_session.result_count();
    let items = scan_session.get_results(&process, offset, limit)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        let json_items: Vec<_> = items.iter().map(|r| {
            json!({"address": r.address, "value": r.value, "previous": r.previous_value})
        }).collect();
        out.success("results", json!({"total": total, "offset": offset, "items": json_items}));
    } else {
        println!("Results ({} total, showing {}-{}):", total, offset, offset + items.len());
        let rows: Vec<Vec<String>> = items.iter().map(|r| {
            vec![r.address.clone(), r.value.clone(), r.previous_value.clone()]
        }).collect();
        out.print_table(&["Address", "Value", "Previous"], rows);
    }
    Ok(())
}

// ─── REPL mode ─────────────────────────────────────────────────────────────

pub fn first_scan_repl(
    state: &mut CliState,
    value_str: &str,
    type_str: &str,
    cond_str: &str,
    value2_str: Option<&str>,
    filter_perm: Option<&str>,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached. Run `attach <pid>` first."))?;

    let value_type = parse_value_type(type_str)?;
    let condition = parse_condition(cond_str)?;

    let value = if condition == ScanCondition::Unknown {
        None
    } else {
        Some(parse_scan_value(value_str, value_type)?)
    };
    let value2 = value2_str.map(|v| parse_scan_value(v, value_type)).transpose()?;

    let mut regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(perm) = filter_perm {
        regions.retain(|r| {
            (!perm.contains('r') || r.readable)
                && (!perm.contains('w') || r.writable)
                && (!perm.contains('x') || r.executable)
        });
    }

    let alignment = value_type.size().max(1);
    let scan_session = ScanSession::first_scan(
        process.as_ref(), value_type, condition, value, value2, &regions, alignment,
    ).map_err(|e| anyhow::anyhow!("{e}"))?;

    let found = scan_session.result_count();
    state.scan_session = Some(scan_session);

    if out.json {
        out.success("scan", json!({"found": found, "type": type_str, "cond": cond_str}));
    } else {
        println!("Found {} matches (type={}, cond={})", found, type_str, cond_str);
    }
    Ok(())
}

pub fn next_scan_repl(
    state: &mut CliState,
    value_str: Option<&str>,
    cond_str: &str,
    value2_str: Option<&str>,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let session = state.scan_session.as_mut()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress. Run `scan` first."))?;

    let condition = parse_condition(cond_str)?;
    let value_type = session.value_type;
    let value = value_str.map(|v| parse_scan_value(v, value_type)).transpose()?;
    let value2 = value2_str.map(|v| parse_scan_value(v, value_type)).transpose()?;

    session.next_scan(process.as_ref(), condition, value, value2)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let found = session.result_count();
    if out.json {
        out.success("next", json!({"found": found}));
    } else {
        println!("Narrowed to {} matches", found);
    }
    Ok(())
}

pub fn undo_scan_repl(state: &mut CliState, out: &Output) -> Result<()> {
    let session = state.scan_session.as_mut()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress."))?;
    if !session.undo() {
        bail!("Nothing to undo.");
    }
    let found = session.result_count();
    if out.json {
        out.success("undo", json!({"found": found}));
    } else {
        println!("Undo: {} matches", found);
    }
    Ok(())
}

pub fn reset_scan_repl(state: &mut CliState, out: &Output) -> Result<()> {
    state.scan_session = None;
    if out.json {
        out.success("reset", json!({"reset": true}));
    } else {
        println!("Scan state cleared.");
    }
    Ok(())
}

pub fn results_repl(state: &CliState, offset: usize, limit: usize, out: &Output) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let session = state.scan_session.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No scan in progress."))?;

    let total = session.result_count();
    let items = session.get_results(process.as_ref(), offset, limit)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        let json_items: Vec<_> = items.iter().map(|r| {
            json!({"address": r.address, "value": r.value, "previous": r.previous_value})
        }).collect();
        out.success("results", json!({"total": total, "offset": offset, "items": json_items}));
    } else {
        println!("Results ({} total, showing {}-{}):", total, offset, offset + items.len());
        let rows: Vec<Vec<String>> = items.iter().map(|r| {
            vec![r.address.clone(), r.value.clone(), r.previous_value.clone()]
        }).collect();
        out.print_table(&["Address", "Value", "Previous"], rows);
    }
    Ok(())
}
