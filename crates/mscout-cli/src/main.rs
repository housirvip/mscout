use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

mod commands;
mod output;
mod repl;
mod session;
mod state;

use output::Output;

#[derive(Parser)]
#[command(name = "mscout", version, about = "Cross-platform memory scanner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Session file path (default: auto-detect)
    #[arg(long, global = true)]
    session: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List running processes
    Ps {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Attach to a process
    Attach { pid: u32 },
    /// Detach from current process
    Detach,
    /// List memory regions
    Regions {
        #[arg(long)]
        perm: Option<String>,
        #[arg(long)]
        module: Option<String>,
    },
    /// First scan for a value
    Scan {
        value: String,
        #[arg(long, short = 't', default_value = "i32")]
        r#type: String,
        #[arg(long, short = 'c', default_value = "eq")]
        cond: String,
        #[arg(long)]
        value2: Option<String>,
        #[arg(long)]
        filter_perm: Option<String>,
    },
    /// Refine scan results
    Next {
        value: Option<String>,
        #[arg(long, short = 'c', default_value = "eq")]
        cond: String,
        #[arg(long)]
        value2: Option<String>,
    },
    /// Undo last scan refinement
    Undo,
    /// Reset scan (clear all results)
    Reset,
    /// Show current scan results
    Results {
        #[arg(long, default_value = "0")]
        offset: usize,
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Read memory at address
    Read {
        address: String,
        #[arg(long, short = 't', default_value = "i32")]
        r#type: String,
        #[arg(long, default_value = "1")]
        count: usize,
    },
    /// Write value to memory address
    Write {
        address: String,
        value: String,
        #[arg(long, short = 't', default_value = "i32")]
        r#type: String,
    },
    /// Hex dump memory at address
    Hex {
        address: String,
        #[arg(long, default_value = "256")]
        len: usize,
    },
    /// Freeze a memory address to a value
    Freeze {
        address: String,
        value: String,
        #[arg(long, short = 't', default_value = "i32")]
        r#type: String,
        #[arg(long, default_value = "100")]
        interval: u64,
        #[arg(long)]
        label: Option<String>,
    },
    /// Unfreeze a memory address
    Unfreeze {
        /// Address to unfreeze
        address: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// List frozen addresses
    Frozen,
    /// Pointer scan to find stable chains
    PointerScan {
        address: String,
        #[arg(long, default_value = "5")]
        depth: usize,
        #[arg(long, default_value = "4096")]
        max_offset: usize,
    },
    /// Resolve a pointer chain to current address
    PointerResolve {
        base: String,
        #[arg(long, value_delimiter = ',')]
        offsets: Vec<String>,
    },
    /// Cheat table operations
    Table {
        #[command(subcommand)]
        action: TableAction,
    },
    /// Virtual machine operations
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },
    /// Interactive REPL mode
    Repl {
        /// JSON output mode (for AI agents)
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TableAction {
    /// Save table to file
    Save { path: PathBuf },
    /// Load table from file
    Load { path: PathBuf },
    /// List table entries
    List,
    /// Add entry to table
    Add {
        address: String,
        #[arg(long, short = 't')]
        r#type: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        pointer: Option<String>,
    },
    /// Remove entry from table
    Rm {
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        index: Option<usize>,
    },
}

#[derive(Subcommand)]
enum VmAction {
    /// List detected virtual machines
    List,
    /// Attach to a VM (list guest processes)
    Attach { pid: u32 },
    /// Attach to a guest process inside a VM
    AttachProcess { vm_pid: u32, guest_pid: u32 },
}

fn main() {
    let cli = Cli::parse();
    let out = Output::new(cli.json);
    let session_path = cli.session.as_deref();

    let result = match cli.command {
        Commands::Ps { filter } => {
            commands::ps::run(filter.as_deref(), &out)
        }
        Commands::Attach { pid } => {
            commands::attach::attach(pid, session_path, &out)
        }
        Commands::Detach => {
            commands::attach::detach(session_path, &out)
        }
        Commands::Regions { perm, module } => {
            commands::regions::list_regions(perm.as_deref(), module.as_deref(), session_path, &out)
        }
        Commands::Scan { value, r#type, cond, value2, filter_perm } => {
            commands::scan::first_scan(
                &value, &r#type, &cond, value2.as_deref(),
                filter_perm.as_deref(), session_path, &out,
            )
        }
        Commands::Next { value, cond, value2 } => {
            commands::scan::next_scan(
                value.as_deref(), &cond, value2.as_deref(), session_path, &out,
            )
        }
        Commands::Undo => {
            commands::scan::undo_scan(session_path, &out)
        }
        Commands::Reset => {
            commands::scan::reset_scan(session_path, &out)
        }
        Commands::Results { offset, limit } => {
            commands::scan::results(offset, limit, session_path, &out)
        }
        Commands::Read { address, r#type, count } => {
            commands::memory::read_memory(&address, &r#type, count, session_path, &out)
        }
        Commands::Write { address, value, r#type } => {
            commands::memory::write_memory(&address, &value, &r#type, session_path, &out)
        }
        Commands::Hex { address, len } => {
            commands::memory::hex_dump(&address, len, session_path, &out)
        }
        Commands::Freeze { address, value, r#type, interval, label } => {
            commands::freeze::freeze_cmd(
                &address, &value, &r#type, interval, label.as_deref(), session_path, &out,
            )
        }
        Commands::Unfreeze { address, all } => {
            commands::freeze::unfreeze_cmd(address.as_deref(), all, session_path, &out)
        }
        Commands::Frozen => {
            commands::freeze::frozen_cmd(session_path, &out)
        }
        Commands::PointerScan { address, depth, max_offset } => {
            commands::pointer::pointer_scan(&address, depth, max_offset, session_path, &out)
        }
        Commands::PointerResolve { base, offsets } => {
            commands::pointer::pointer_resolve(&base, &offsets, session_path, &out)
        }
        Commands::Table { action } => match action {
            TableAction::Save { path } => commands::table::save_table(&path, session_path, &out),
            TableAction::Load { path } => commands::table::load_table(&path, session_path, &out),
            TableAction::List => commands::table::list_table(session_path, &out),
            TableAction::Add { address, r#type, label, pointer } => {
                commands::table::add_entry(&address, &r#type, label.as_deref(), pointer.as_deref(), session_path, &out)
            }
            TableAction::Rm { address, index } => {
                commands::table::remove_entry(address.as_deref(), index, session_path, &out)
            }
        },
        Commands::Vm { action } => match action {
            VmAction::List => commands::vm::list_vms(&out),
            VmAction::Attach { pid } => commands::vm::attach_vm(pid, &out),
            VmAction::AttachProcess { vm_pid, guest_pid } => {
                commands::vm::attach_vm_process(vm_pid, guest_pid, session_path, &out)
            }
        },
        Commands::Repl { json } => {
            repl::run_repl(json || cli.json)
        }
    };

    if let Err(e) = result {
        let cmd = std::env::args().nth(1).unwrap_or_else(|| "unknown".into());
        out.error(&cmd, &e.to_string());
        process::exit(1);
    }
}
