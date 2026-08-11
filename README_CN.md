# MScout

跨平台内存扫描器，用于游戏修改与逆向工程。

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
- **跨平台** — macOS、Windows、Linux
- **双语界面** — 中文 & 英文

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 前端 | React 19 + TypeScript + Vite |
| 后端 | Rust |
| 平台支持 | macOS (mach2)、Windows (Win32 API)、Linux (/proc) |

## 前置条件

- **Node.js** ≥ 18
- **Rust** 工具链（stable）
- 平台特定要求：
  - **macOS** — 关闭 SIP，或为二进制签名 `com.apple.security.cs.debugger` 权限
  - **Linux** — 使用 `sudo` 运行，或授予 `CAP_SYS_PTRACE` 能力
  - **Windows** — 以管理员身份运行

## 快速开始

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

产物位于 `src-tauri/target/release/bundle/`。

## 使用方法

1. 启动 MScout → 从进程列表中选择目标进程
2. 设置数据类型和扫描条件 → **首次扫描**
3. 在游戏中改变数值 → **再次扫描** 缩小结果范围
4. 右键点击结果 → **添加到地址表**
5. 冻结或修改数值
6. 保存为 `.mst` 金手指表以便下次使用

## 快捷键

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
├── src/                  # React 前端
├── src-tauri/            # Tauri + Rust 命令层
│   └── src/commands/     # IPC 处理器（进程、扫描、内存、冻结、指针、表、虚拟机）
├── crates/mscout-core/   # 核心库（平台抽象、扫描引擎、指针扫描、冻结、虚拟机）
└── package.json
```

## 许可证

MIT 许可证 — 详见 [LICENSE](LICENSE)。
