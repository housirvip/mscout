# mscout — Cross-platform Memory Scanner CLI

Memory scanner for game modding and reverse engineering. Attach to processes,
scan/modify memory values, freeze addresses, trace pointer chains, and
introspect VM guest memory — all from the command line.

## When to use

- Finding and modifying game values (health, gold, ammo)
- Pointer chain discovery for stable addresses across restarts
- VM memory introspection (VMware/Hyper-V guests) without guest tools
- Automated memory analysis workflows
- Any task involving process memory reading/writing

## Permissions

| Platform | Requirement |
|----------|-------------|
| macOS    | `sudo` or binary signed with `com.apple.security.cs.debugger` entitlement; SIP must be disabled for system processes |
| Linux    | `sudo` or `CAP_SYS_PTRACE` capability |
| Windows  | Run as Administrator |

## Command Reference

### Process management
```bash
mscout ps [--filter <name>] [--json]          # List processes
mscout attach <pid> [--json]                   # Attach (creates session)
mscout detach [--json]                         # Detach (removes session, stops freeze)
```

### Memory scanning (stateful — requires prior attach)
```bash
mscout scan <value> --type <type> [--cond <cond>] [--value2 <v>] [--json]
mscout next [<value>] [--cond <cond>] [--value2 <v>] [--json]
mscout undo [--json]                           # Revert last next_scan
mscout reset [--json]                          # Clear all scan state
mscout results [--offset 0] [--limit 50] [--json]
```

### Memory read/write
```bash
mscout read <addr> --type <type> [--count 1] [--json]
mscout write <addr> <value> --type <type> [--json]
mscout hex <addr> [--len 256] [--json]
```

### Value freezing
```bash
mscout freeze <addr> <value> --type <type> [--interval 100] [--label <text>] [--json]
mscout unfreeze <addr> [--json]
mscout unfreeze --all [--json]
mscout frozen [--json]                         # List frozen entries
```

### Pointer scanning
```bash
mscout pointer-scan <addr> [--depth 5] [--max-offset 4096] [--json]
mscout pointer-resolve <base> --offsets 0x10,0x20 [--json]
```

### Cheat table
```bash
mscout table save <file.mst> [--json]
mscout table load <file.mst> [--json]
mscout table list [--json]
mscout table add <addr> --type <type> [--label <text>] [--pointer <spec>] [--json]
mscout table rm [--address <addr>] [--index <n>] [--json]
```

### Memory regions
```bash
mscout regions [--perm rw] [--module <name>] [--json]
```

### Virtual machine introspection
```bash
mscout vm list [--json]                        # Detect VMware/Hyper-V VMs
mscout vm attach <vm_pid> [--json]             # List guest processes
mscout vm attach-process <vm_pid> <guest_pid> [--json]  # Attach to guest process
```

### Interactive REPL (for AI agents with PTY or stdin pipe)
```bash
mscout repl [--json]                           # Enter interactive mode
```

## Data types

`i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | aob`

- Default: `i32`
- `aob` (Array of Bytes): hex string with `??` wildcards, e.g. `"48 8B ?? ?? 89"`

## Scan conditions

| Condition | Alias | Requires value | Description |
|-----------|-------|---------------|-------------|
| `eq`      | exact | yes | Equal to value |
| `gt`      |       | yes | Greater than value |
| `lt`      |       | yes | Less than value |
| `range`   |       | yes (+ --value2) | Between value and value2 |
| `changed` |       | no  | Changed since last scan |
| `unchanged` |     | no  | Same as last scan |
| `increased` |     | no  | Increased since last scan |
| `decreased` |     | no  | Decreased since last scan |
| `increased_by` |  | yes | Increased by exactly value |
| `decreased_by` |  | yes | Decreased by exactly value |
| `unknown`  |      | no  | Record all (first scan only) |

## Address format

Addresses accept hex with or without `0x` prefix:
- `0x7FFF12340000`
- `7FFF12340000`

## JSON output schema

All commands with `--json` output a single JSON line to stdout:

```json
// Success
{"ok": true, "cmd": "<command>", "data": <varies by command>}

// Error
{"ok": false, "cmd": "<command>", "error": "<message>"}
```

### Per-command data shapes

```json
// ps
{"ok":true,"cmd":"ps","data":[{"pid":1234,"name":"game.exe"},...]}

// attach
{"ok":true,"cmd":"attach","data":{"pid":1234,"name":"game.exe"}}

// scan / next
{"ok":true,"cmd":"scan","data":{"found":48231,"type":"i32","cond":"eq"}}

// undo
{"ok":true,"cmd":"undo","data":{"found":150000}}

// results
{"ok":true,"cmd":"results","data":{"total":3,"offset":0,"items":[
  {"address":"0x7FFF12340000","value":"1000","previous":"1050"},
  {"address":"0x7FFF12340100","value":"1000","previous":"1050"}
]}}

// read
{"ok":true,"cmd":"read","data":[{"address":"0x7FFF12340000","value":"1000"},...]}

// hex
{"ok":true,"cmd":"hex","data":{"address":"0x7FFF12340000","length":256,"hex":"48 8B 05 ..."}}

// regions
{"ok":true,"cmd":"regions","data":[
  {"start":"0x100000000","end":"0x100004000","size":16384,"perm":"r-x","module":"/usr/lib/game"}
]}

// pointer-scan
{"ok":true,"cmd":"pointer-scan","data":{"target":"0x7FFF20040","chains":[
  {"base":"0x10000","module":"/game.exe","offsets":["0x40"],"resolved":"0x7FFF20040"}
]}}

// frozen
{"ok":true,"cmd":"frozen","data":[
  {"address":"0x7FFF12340000","value":"999","type":"i32","enabled":true,"label":"gold"}
]}

// vm list
{"ok":true,"cmd":"vm-list","data":[{"id":"vm1","name":"Win11","type":"vmware","pid":5678}]}
```

## Typical workflows

### 1. Find and modify a game value

Scenario: game shows 1000 gold, you want to change it to 99999.

```bash
# Find the game process
mscout ps --filter "game" --json
# -> get PID, e.g. 12345

mscout attach 12345

# First scan: look for value 1000 as 32-bit integer
mscout scan 1000 --type i32 --cond eq --json
# -> {"ok":true,"cmd":"scan","data":{"found":5832}}

# Too many results. Go spend some gold in-game (now 950).
mscout next 950 --cond eq --json
# -> {"ok":true,"cmd":"next","data":{"found":3}}

# Check results
mscout results --json
# -> see 3 addresses

# Write new value to the most likely address
mscout write 0x7FFF12340000 99999 --type i32

# Optionally freeze it so it doesn't decrease
mscout freeze 0x7FFF12340000 99999 --type i32 --label "gold"

# Save for later
mscout table add 0x7FFF12340000 --type i32 --label "gold"
mscout table save game.mst

mscout detach
```

### 2. Unknown initial value (health bar with no number)

```bash
mscout attach 12345

# Record everything — no value needed
mscout scan 0 --type f32 --cond unknown --json
# -> {"ok":true,"cmd":"scan","data":{"found":2841600}}

# Take damage in game
mscout next --cond decreased --json
# -> found: 89420

# Take more damage
mscout next --cond decreased --json
# -> found: 312

# Heal back to full
mscout next --cond increased --json
# -> found: 8

# Now check values
mscout results --json
```

### 3. Pointer chain for stable address

```bash
# After finding the dynamic address 0x7FFF20040:
mscout pointer-scan 0x7FFF20040 --depth 4 --max-offset 4096 --json
# -> chains with static base addresses

# Save as pointer-based table entry
mscout table add 0x7FFF20040 --type i32 --label "health" \
  --pointer "/game.exe+0x10000:0x0,0x40"
mscout table save game.mst
```

Pointer spec format: `<module>+<base_offset>:<offset1>,<offset2>,...`

### 4. VM introspection (scan VM guest without agent)

```bash
# Find running VMs
mscout vm list --json
# -> [{"id":"...","name":"Win11-Gaming","type":"vmware","pid":5678}]

# List guest processes
mscout vm attach 5678 --json
# -> [{"pid":1234,"name":"game.exe"}, ...]

# Attach to guest process (replaces normal attach)
mscout vm attach-process 5678 1234

# Now use scan/read/write/freeze normally — they go through VM page tables
mscout scan 1000 --type i32 --cond eq --json
```

### 5. REPL mode for AI agents

For AI frameworks with stdin/stdout process management (e.g., OMP hub):

```bash
# Start REPL
sudo mscout repl --json
# First output: {"ready":true,"version":"0.1.0"}

# Send commands line by line via stdin:
attach 12345
scan 1000 --type i32 --cond eq
next 950 --cond eq
results --limit 10
write 0x7FFF12340000 99999 --type i32
exit
```

Each command produces exactly one JSON line response. REPL state is in-memory (no session file overhead).

For frameworks without PTY management, use subcommand mode with `--json` directly:
```bash
bash("sudo mscout scan 1000 --type i32 --cond eq --json")
bash("sudo mscout next 950 --cond eq --json")
```

## Error handling

| Error | Cause | Fix |
|-------|-------|-----|
| "No active session" | Command requires attach | Run `mscout attach <pid>` first |
| "No process attached" | Session exists but process died | Re-attach with same PID |
| "Permission denied" | Insufficient privileges | Use sudo / run as admin |
| "Process not found" | PID doesn't exist | Check with `mscout ps` |
| "No scan session active" | next/undo without first scan | Run `mscout scan` first |
| "Invalid address" | Bad hex format | Use `0x` prefix or pure hex digits |

## Notes

- Session state persists in `$TMPDIR/mscout/session-<pid>.bin` — delete manually if corrupt
- Freeze in subcommand mode writes once; use REPL mode for continuous freeze (background write loop)
- `mscout undo` can be called multiple times (up to 10 history levels)
- Pointer scan is CPU-intensive; limit `--depth` to 3-4 for faster results
- AOB patterns: `"48 8B ?? 05 90"` — `??` matches any byte
- Results are capped at 10,000 displayed; use `--offset` to paginate
