# MScout

跨平台内存扫描器，用于游戏修改与逆向工程。  
类似 Cheat Engine，但原生运行于 macOS、Windows 和 Linux —— 同时提供 GUI 和无界面 CLI。

[English](./README.md)

![MScout 截图 1](screenshots/pic1.png)
![MScout 截图 2](screenshots/pic2.png)
![MScout 截图 3](screenshots/pic3.png)
![MScout 截图 4](screenshots/pic4.png)

## 功能特性

- **进程附加** — 通过 PID 附加到任意运行中的进程
- **内存扫描** — 支持 11 种扫描条件的迭代式扫描（精确值、大于、小于、范围、变化、未变化、增大、减小、增加了、减少了、未知）
- **数据类型** — I8 / I16 / I32 / I64 / U8 / U16 / U32 / U64 / F32 / F64 / 字节数组（支持通配符 AOB）
- **数值冻结** — 后台写入循环锁定地址值
- **指针扫描** — 基于 BFS 的指针链查找器，可配置深度和偏移量
- **金手指表** — 保存/加载 `.mst` 文件（JSON 格式，支持静态地址和指针链地址）
- **内存查看器** — 十六进制编辑器，实时刷新
- **区域浏览器** — 可视化内存布局，按权限颜色编码
- **虚拟机内省** — 无需客户机组件即可扫描 VMware / Hyper-V 虚拟机内存
- **命令行工具** — 完整功能的 CLI，支持脚本化和 AI agent 自动化
- **跨平台** — macOS、Windows、Linux
- **双语界面** — 中文 & 英文

## 技术栈

| 层级 | 技术 |
|------|------|
| 核心 | Rust (mscout-core) — 平台抽象、扫描引擎、指针扫描、冻结、VM |
| GUI | Tauri 2 + React 19 + TypeScript + Vite |
| CLI | Rust (clap) — 子命令模式 + 交互式 REPL |
| 平台支持 | macOS (mach2)、Windows (Win32 API)、Linux (/proc) |

## 前置条件

- **Rust** 工具链（stable）
- **Node.js** ≥ 18（仅 GUI 需要）
- 平台特定要求：
  - **macOS** — 关闭 SIP，或为二进制签名 `com.apple.security.cs.debugger` 权限
  - **Linux** — 使用 `sudo` 运行，或授予 `CAP_SYS_PTRACE` 能力
  - **Windows** — 以管理员身份运行

## 快速开始

### GUI

```bash
git clone https://github.com/housirvip/mscout.git
cd mscout
npm install
npm run tauri dev
```

生产构建：

```bash
npm run tauri build
```

### CLI

```bash
cargo build -p mscout-cli --release
# 产物位于: target/release/mscout
```

或直接运行：

```bash
cargo run -p mscout-cli -- ps --filter "game"
```

## CLI 使用

CLI 暴露所有核心能力，适用于终端操作、脚本自动化和 AI agent 驱动。

```bash
# 列出进程
mscout ps --filter "game" --json

# 附加到进程
mscout attach 12345

# 扫描数值
mscout scan 1000 --type i32 --cond eq --json

# 数值变化后缩小范围
mscout next 950 --cond eq --json

# 查看结果
mscout results --limit 20

# 读写内存
mscout read 0x7FFF1234 --type i32
mscout write 0x7FFF1234 99999 --type i32

# 冻结数值
mscout freeze 0x7FFF1234 99999 --type i32 --label "gold"

# 指针扫描
mscout pointer-scan 0x7FFF1234 --depth 4 --max-offset 4096

# 十六进制转储
mscout hex 0x7FFF1234 --len 256

# 内存区域
mscout regions --perm rw

# 虚拟机内省
mscout vm list
mscout vm attach-process 5678 1234

# 金手指表
mscout table save game.mst
mscout table load game.mst

# 交互式 REPL（AI agent 推荐）
mscout repl --json

# 脱离
mscout detach
```

所有命令支持 `--json` 输出结构化数据。完整参考见 [`docs/skill-mscout-cli.md`](docs/skill-mscout-cli.md)。

## GUI 使用

1. 启动 MScout → 从进程列表中选择目标进程
2. 设置数据类型和扫描条件 → **首次扫描**
3. 在游戏中改变数值 → **再次扫描** 缩小结果范围
4. 右键点击结果 → **添加到地址表**
5. 冻结或修改数值
6. 保存为 `.mst` 金手指表以便下次使用

### 快捷键

| 快捷键 | 操作 |
|--------|------|
| F5 | 首次扫描 |
| F6 | 再次扫描 |
| ⌘/Ctrl+Z | 撤销扫描 |
| ⌘/Ctrl+N | 新建扫描（重置） |
| ⌘/Ctrl+S | 保存表 |
| ⌘/Ctrl+O | 加载表 |

## 项目结构

```
mscout/
├── crates/
│   ├── mscout-core/      # 核心库（平台抽象、扫描引擎、指针扫描、冻结、虚拟机）
│   └── mscout-cli/       # CLI 二进制（子命令 + REPL）
├── src-tauri/            # Tauri 应用壳 + Rust IPC 命令
│   └── src/commands/     # GUI 命令处理器
├── src/                  # React 前端
├── docs/
│   └── skill-mscout-cli.md  # AI agent 技能参考文档
├── Cargo.toml            # 工作空间根
└── package.json
```

## 架构

```
┌─────────────────────────────────────────────────┐
│                 mscout-core                      │
│  （平台抽象、扫描引擎、指针扫描、                    │
│    冻结、金手指表、虚拟机内省）                      │
└──────────────┬───────────────────┬──────────────┘
               │                   │
    ┌──────────▼──────────┐  ┌────▼─────────────┐
    │   src-tauri (GUI)    │  │  mscout-cli (CLI) │
    │  Tauri 2 + React 19  │  │  clap + REPL      │
    └──────────────────────┘  └──────────────────┘
```

GUI 和 CLI 均直接消费 `mscout-core` —— CLI 零 IPC 开销，GUI 通过 Tauri IPC 桥接。

## 许可证

MIT 许可证 — 详见 [LICENSE](LICENSE)。
