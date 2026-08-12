use std::path::Path;

use anyhow::Result;
use mscout_core::platform::{NativeProcess, ProcessMemory};
use serde_json::json;

use crate::output::Output;
use crate::session;
use crate::state::CliState;

pub fn list_regions(
    perm: Option<&str>,
    module: Option<&str>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(perm) = perm {
        regions.retain(|r| {
            (!perm.contains('r') || r.readable)
                && (!perm.contains('w') || r.writable)
                && (!perm.contains('x') || r.executable)
        });
    }
    if let Some(module) = module {
        let m_lower = module.to_lowercase();
        regions.retain(|r| r.info.to_lowercase().contains(&m_lower));
    }

    if out.json {
        let data: Vec<_> = regions.iter().map(|r| {
            let perm_str = format!("{}{}{}",
                if r.readable { "r" } else { "-" },
                if r.writable { "w" } else { "-" },
                if r.executable { "x" } else { "-" },
            );
            json!({
                "start": format!("0x{:X}", r.base),
                "end": format!("0x{:X}", r.base.saturating_add(r.size)),
                "size": r.size,
                "perm": perm_str,
                "module": r.info,
            })
        }).collect();
        out.success("regions", data);
    } else {
        let rows: Vec<Vec<String>> = regions.iter().map(|r| {
            let perm_str = format!("{}{}{}",
                if r.readable { "r" } else { "-" },
                if r.writable { "w" } else { "-" },
                if r.executable { "x" } else { "-" },
            );
            vec![
                format!("0x{:X}", r.base),
                format!("0x{:X}", r.base.saturating_add(r.size)),
                format!("{}", r.size),
                perm_str,
                r.info.clone(),
            ]
        }).collect();
        out.print_table(&["Start", "End", "Size", "Perm", "Module"], rows);
    }
    Ok(())
}

pub fn list_regions_repl(
    state: &CliState,
    perm: Option<&str>,
    module: Option<&str>,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;

    let mut regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(perm) = perm {
        regions.retain(|r| {
            (!perm.contains('r') || r.readable)
                && (!perm.contains('w') || r.writable)
                && (!perm.contains('x') || r.executable)
        });
    }
    if let Some(module) = module {
        let m_lower = module.to_lowercase();
        regions.retain(|r| r.info.to_lowercase().contains(&m_lower));
    }

    if out.json {
        let data: Vec<_> = regions.iter().map(|r| {
            let perm_str = format!("{}{}{}",
                if r.readable { "r" } else { "-" },
                if r.writable { "w" } else { "-" },
                if r.executable { "x" } else { "-" },
            );
            json!({
                "start": format!("0x{:X}", r.base),
                "end": format!("0x{:X}", r.base.saturating_add(r.size)),
                "size": r.size,
                "perm": perm_str,
                "module": r.info,
            })
        }).collect();
        out.success("regions", data);
    } else {
        let rows: Vec<Vec<String>> = regions.iter().map(|r| {
            let perm_str = format!("{}{}{}",
                if r.readable { "r" } else { "-" },
                if r.writable { "w" } else { "-" },
                if r.executable { "x" } else { "-" },
            );
            vec![
                format!("0x{:X}", r.base),
                format!("0x{:X}", r.base.saturating_add(r.size)),
                format!("{}", r.size),
                perm_str,
                r.info.clone(),
            ]
        }).collect();
        out.print_table(&["Start", "End", "Size", "Perm", "Module"], rows);
    }
    Ok(())
}
