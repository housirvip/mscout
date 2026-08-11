# MScout

A cross-platform memory scanner for game hacking and reverse engineering.

[中文](./README_CN.md)

![MScout Screenshot 1](screenshots/pic1.png)
![MScout Screenshot 2](screenshots/pic2.png)
![MScout Screenshot 3](screenshots/pic3.png)
![MScout Screenshot 4](screenshots/pic4.png)

## Features

- **Process attachment** — attach to any running process by PID
- **Memory scanning** — iterative scan with 11 conditions (Exact, GreaterThan, LessThan, Between, Changed, Unchanged, Increased, Decreased, IncreasedBy, DecreasedBy, Unknown)
- **Value types** — I8 / I16 / I32 / I64 / U8 / U16 / U32 / U64 / F32 / F64 / ByteArray (wildcard AOB)
- **Value freezing** — lock addresses to a value with a background write loop
- **Pointer scanning** — BFS-based pointer chain finder with configurable depth and offset
- **Cheat tables** — save/load `.mst` files (JSON-based, supports static & pointer addresses)
- **Memory viewer** — hex editor with real-time refresh
- **Region browser** — visualize memory layout with permission color coding
- **VM introspection** — scan VMware / Hyper-V guest memory without guest-side tools
- **Cross-platform** — macOS, Windows, Linux
- **Bilingual UI** — English & Chinese

## Tech Stack

| Layer | Stack |
|-------|-------|
| Desktop | Tauri 2 |
| Frontend | React 19 + TypeScript + Vite |
| Backend | Rust |
| Platforms | macOS (mach2), Windows (Win32 API), Linux (/proc) |

## Prerequisites

- **Node.js** ≥ 18
- **Rust** toolchain (stable)
- Platform-specific requirements:
  - **macOS** — disable SIP, or sign the binary with `com.apple.security.cs.debugger` entitlement
  - **Linux** — run with `sudo`, or grant `CAP_SYS_PTRACE` capability
  - **Windows** — run as Administrator

## Quick Start

```bash
git clone https://github.com/housirvip/mscout.git
cd mscout
npm install
npm run tauri dev
```

Production build:

```bash
npm run tauri build
```

Output binaries are located at `src-tauri/target/release/bundle/`.

## Usage

1. Launch MScout → pick a process from the process list
2. Set value type and scan condition → **First Scan**
3. Change the value in-game → **Next Scan** to narrow results
4. Right-click a result → **Add to Address Table**
5. Freeze or modify the value
6. Save as `.mst` cheat table for later

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| F5 | First Scan |
| F6 | Next Scan |
| ⌘/Ctrl+Z | Undo Scan |
| ⌘/Ctrl+N | New Scan (reset) |
| ⌘/Ctrl+S | Save Table |
| ⌘/Ctrl+O | Load Table |

## Project Structure

```
mscout/
├── src/                  # React frontend
├── src-tauri/            # Tauri + Rust commands
│   └── src/commands/     # IPC handlers (process, scan, memory, freeze, pointer, table, vm)
├── crates/mscout-core/   # Core library (platform, scanner, pointer, freeze, vm)
└── package.json
```

## License

MIT License — see [LICENSE](LICENSE) for details.
