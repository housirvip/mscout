use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::commands;
use crate::output::Output;
use crate::state::CliState;

pub fn run_repl(json_mode: bool) -> Result<()> {
    let out = Output::new(json_mode);

    // Print ready marker
    if json_mode {
        println!(
            r#"{{"ready":true,"version":"{}"}}"#,
            env!("CARGO_PKG_VERSION")
        );
    } else {
        println!(
            "MScout v{} — type 'help' for commands, 'exit' to quit",
            env!("CARGO_PKG_VERSION")
        );
    }

    let mut state = CliState::new();
    let mut rl = DefaultEditor::new()?;

    loop {
        let prompt = if json_mode { "" } else { "mscout> " };
        let line = match rl.readline(prompt) {
            Ok(l) => l,
            Err(ReadlineError::Eof | ReadlineError::Interrupted) => break,
            Err(e) => return Err(e.into()),
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "exit" || line == "quit" {
            break;
        }

        let _ = rl.add_history_entry(line);

        let tokens = match shell_words::split(line) {
            Ok(t) => t,
            Err(e) => {
                out.error("parse", &format!("Invalid input: {e}"));
                continue;
            }
        };

        if let Err(e) = execute_repl_command(&mut state, &tokens, &out) {
            let cmd = tokens.first().map(|s| s.as_str()).unwrap_or("unknown");
            out.error(cmd, &e.to_string());
        }
    }

    // Cleanup
    if let Some(mut fm) = state.freeze_manager.take() {
        fm.stop();
    }

    Ok(())
}

fn execute_repl_command(state: &mut CliState, tokens: &[String], out: &Output) -> Result<()> {
    if tokens.is_empty() {
        return Ok(());
    }

    let cmd = tokens[0].as_str();
    let args = &tokens[1..];

    match cmd {
        "help" => {
            if !out.json {
                print_help();
            } else {
                out.success("help", serde_json::json!({"commands": [
                    "ps", "attach", "detach", "regions", "scan", "next", "undo", "reset",
                    "results", "read", "write", "hex", "freeze", "unfreeze", "frozen",
                    "pointer-scan", "pointer-resolve", "vm", "help", "exit"
                ]}));
            }
            Ok(())
        }
        "ps" => {
            let filter = find_flag_value(args, "--filter")
                .or_else(|| find_flag_value(args, "-f"));
            commands::ps::run(filter.as_deref(), out)
        }
        "attach" => {
            let pid: u32 = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: attach <pid>"))?
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid PID"))?;
            commands::attach::attach_repl(state, pid, out)
        }
        "detach" => commands::attach::detach_repl(state, out),
        "regions" => {
            let perm = find_flag_value(args, "--perm");
            let module = find_flag_value(args, "--module");
            commands::regions::list_regions_repl(state, perm.as_deref(), module.as_deref(), out)
        }
        "scan" => {
            let value = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: scan <value> [--type i32] [--cond eq]"))?
                .clone();
            let type_str = find_flag_value(args, "--type")
                .or_else(|| find_flag_value(args, "-t"))
                .unwrap_or_else(|| "i32".to_string());
            let cond = find_flag_value(args, "--cond")
                .or_else(|| find_flag_value(args, "-c"))
                .unwrap_or_else(|| "eq".to_string());
            let value2 = find_flag_value(args, "--value2");
            let filter_perm = find_flag_value(args, "--filter-perm");
            commands::scan::first_scan_repl(
                state, &value, &type_str, &cond,
                value2.as_deref(), filter_perm.as_deref(), out,
            )
        }
        "next" => {
            let value = args.first()
                .filter(|s| !s.starts_with("--") && !(s.starts_with('-') && s.as_bytes().get(1).map_or(false, |b| b.is_ascii_alphabetic())))
                .cloned();
            let cond = find_flag_value(args, "--cond")
                .or_else(|| find_flag_value(args, "-c"))
                .unwrap_or_else(|| "eq".to_string());
            let value2 = find_flag_value(args, "--value2");
            commands::scan::next_scan_repl(
                state, value.as_deref(), &cond, value2.as_deref(), out,
            )
        }
        "undo" => commands::scan::undo_scan_repl(state, out),
        "reset" => commands::scan::reset_scan_repl(state, out),
        "results" => {
            let offset: usize = find_flag_value(args, "--offset")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let limit: usize = find_flag_value(args, "--limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50);
            commands::scan::results_repl(state, offset, limit, out)
        }
        "read" => {
            let addr = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: read <address> [--type i32] [--count 1]"))?
                .clone();
            let type_str = find_flag_value(args, "--type")
                .or_else(|| find_flag_value(args, "-t"))
                .unwrap_or_else(|| "i32".to_string());
            let count: usize = find_flag_value(args, "--count")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            commands::memory::read_memory_repl(state, &addr, &type_str, count, out)
        }
        "write" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: write <address> <value> [--type i32]");
            }
            let addr = &args[0];
            let value = &args[1];
            let type_str = find_flag_value(args, "--type")
                .or_else(|| find_flag_value(args, "-t"))
                .unwrap_or_else(|| "i32".to_string());
            commands::memory::write_memory_repl(state, addr, value, &type_str, out)
        }
        "hex" => {
            let addr = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: hex <address> [--len 256]"))?
                .clone();
            let len: usize = find_flag_value(args, "--len")
                .and_then(|v| v.parse().ok())
                .unwrap_or(256);
            commands::memory::hex_dump_repl(state, &addr, len, out)
        }
        "freeze" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: freeze <address> <value> [--type i32] [--label text]");
            }
            let addr = &args[0];
            let value = &args[1];
            let type_str = find_flag_value(args, "--type")
                .or_else(|| find_flag_value(args, "-t"))
                .unwrap_or_else(|| "i32".to_string());
            let label = find_flag_value(args, "--label");
            commands::freeze::freeze_repl(state, addr, value, &type_str, label.as_deref(), out)
        }
        "unfreeze" => {
            let all = args.iter().any(|s| s == "--all");
            let addr = args.first()
                .filter(|s| !s.starts_with("--"))
                .map(|s| s.as_str());
            commands::freeze::unfreeze_repl(state, addr, all, out)
        }
        "frozen" => commands::freeze::frozen_repl(state, out),
        "pointer-scan" => {
            let addr = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: pointer-scan <address> [--depth 5] [--max-offset 4096]"))?
                .clone();
            let depth: usize = find_flag_value(args, "--depth")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            let max_offset: usize = find_flag_value(args, "--max-offset")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4096);
            commands::pointer::pointer_scan_repl(state, &addr, depth, max_offset, out)
        }
        "pointer-resolve" => {
            let base = args.first()
                .ok_or_else(|| anyhow::anyhow!("Usage: pointer-resolve <base> --offsets 0x10,0x20"))?
                .clone();
            let offsets_str = find_flag_value(args, "--offsets")
                .ok_or_else(|| anyhow::anyhow!("Usage: pointer-resolve <base> --offsets 0x10,0x20"))?;
            let offsets: Vec<String> = offsets_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            commands::pointer::pointer_resolve_repl(state, &base, &offsets, out)
        }
        "vm" => {
            let sub = args.first().map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" => commands::vm::list_vms(out),
                "attach" => {
                    let pid: u32 = args.get(1)
                        .ok_or_else(|| anyhow::anyhow!("Usage: vm attach <vm_pid>"))?
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid VM PID"))?;
                    commands::vm::attach_vm(pid, out)
                }
                "attach-process" => {
                    if args.len() < 3 {
                        anyhow::bail!("Usage: vm attach-process <vm_pid> <guest_pid>");
                    }
                    let vm_pid: u32 = args[1].parse().map_err(|_| anyhow::anyhow!("Invalid VM PID"))?;
                    let guest_pid: u32 = args[2].parse().map_err(|_| anyhow::anyhow!("Invalid guest PID"))?;
                    commands::vm::attach_vm_process_repl(state, vm_pid, guest_pid, out)
                }
                _ => anyhow::bail!("Unknown vm subcommand: {sub}. Use: list, attach, attach-process"),
            }
        }
        _ => anyhow::bail!("Unknown command: {cmd}. Type 'help' for available commands."),
    }
}

/// Find a flag value like --type i32 in args slice.
fn find_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|s| s == flag)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.clone())
}

fn print_help() {
    println!(
        r#"Commands:
  ps [--filter <name>]                     List processes
  attach <pid>                             Attach to process
  detach                                   Detach from process
  regions [--perm rw] [--module <name>]    List memory regions
  scan <value> [--type i32] [--cond eq]    First scan
  next [<value>] [--cond eq]               Refine scan
  undo                                     Undo last scan
  reset                                    Clear scan state
  results [--offset 0] [--limit 50]        Show results
  read <addr> [--type i32] [--count 1]     Read memory
  write <addr> <value> [--type i32]        Write memory
  hex <addr> [--len 256]                   Hex dump
  freeze <addr> <value> [--type i32]       Freeze value
  unfreeze <addr> | --all                  Unfreeze
  frozen                                   List frozen
  pointer-scan <addr> [--depth 5]          Pointer scan
  pointer-resolve <base> --offsets 0x10    Resolve chain
  vm list                                  List VMs
  vm attach <pid>                          Attach VM
  vm attach-process <vm_pid> <guest_pid>   Attach guest
  help                                     This message
  exit                                     Quit"#
    );
}
