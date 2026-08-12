use serde::Serialize;
use serde_json::json;

/// Dual-mode output: JSON for machines, human-readable tables for terminals.
pub struct Output {
    pub json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    pub fn success<T: Serialize>(&self, cmd: &str, data: T) {
        if self.json {
            let out = json!({
                "ok": true,
                "cmd": cmd,
                "data": data,
            });
            println!("{}", serde_json::to_string(&out).unwrap());
        }
        // In human mode, callers print their own formatted output
    }

    pub fn error(&self, cmd: &str, message: &str) {
        if self.json {
            let out = json!({
                "ok": false,
                "cmd": cmd,
                "error": message,
            });
            println!("{}", serde_json::to_string(&out).unwrap());
        } else {
            eprintln!("error: {}", message);
        }
    }

    pub fn print_table(&self, headers: &[&str], rows: Vec<Vec<String>>) {
        use comfy_table::{Table, ContentArrangement};
        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(headers.iter().map(|h| h.to_string()));
        for row in rows {
            table.add_row(row);
        }
        println!("{table}");
    }
}
