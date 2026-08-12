use anyhow::Result;
use mscout_core::platform::{NativeProcess, ProcessMemory};
use serde_json::json;

use crate::output::Output;

pub fn run(filter: Option<&str>, out: &Output) -> Result<()> {
    let processes = NativeProcess::list_processes()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let filtered: Vec<_> = match filter {
        Some(f) => {
            let f_lower = f.to_lowercase();
            processes
                .into_iter()
                .filter(|p| p.name.to_lowercase().contains(&f_lower))
                .collect()
        }
        None => processes,
    };

    if out.json {
        let data: Vec<_> = filtered
            .iter()
            .map(|p| json!({"pid": p.pid, "name": p.name}))
            .collect();
        out.success("ps", data);
    } else {
        let rows: Vec<Vec<String>> = filtered
            .iter()
            .map(|p| vec![p.pid.to_string(), p.name.clone()])
            .collect();
        out.print_table(&["PID", "Name"], rows);
    }
    Ok(())
}
