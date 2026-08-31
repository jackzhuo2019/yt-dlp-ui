# yt-dlp UI

基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的跨平台视频下载图形界面，使用 **Tauri v2 + React + TypeScript** 构建。
<img width="958" height="678" alt="image" src="https://github.com/user-attachments/assets/7ed1aeb5-107c-4e20-accb-2d2effe994a4" />


## 功能

- **粘贴 URL 下载** — 支持单个视频、播放列表、频道链接
- **格式 / 画质选择** — 列出所有可用格式，按分辨率、编码筛选
- **批量下载** — 多 URL 同时输入，支持播放列表自动展开
- **实时进度** — 下载进度条、速度、ETA、文件大小
- **下载队列** — 后台并发控制，支持暂停 / 恢复 / 取消
- **下载历史** — 本地 SQLite 记录，支持去重与搜索

## 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | [Tauri v2](https://v2.tauri.app/) |
| 前端 | React 19 + TypeScript |
| 样式 | Tailwind CSS |
| 后端 | Rust（进程管理、队列调度、历史存储） |
| 数据 | SQLite（通过 `tauri-plugin-sql`） |
| 下载引擎 | [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases) |

## 架构

```
┌──────────────────────────────────────────────┐
│                  React 前端                    │
│  ┌─────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ URL输入  │ │ 格式选择  │ │  下载队列列表  │  │
│  │ 批量粘贴 │ │ 画质筛选  │ │  进度条+速度   │  │
│  └─────────┘ └──────────┘ └──────────────┘  │
│  ┌──────────────────────────────────────┐    │
│  │          下载历史记录                  │    │
│  └──────────────────────────────────────┘    │
├──────────────────────────────────────────────┤
│              Tauri Rust 后端                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ 进程管理  │ │ 队列调度  │ │  历史存储     │ │
│  │ spawn     │ │ 并发控制  │ │  SQLite      │ │
│  │ yt-dlp   │ │ 暂停/恢复 │ │              │ │
│  └──────────┘ └──────────┘ └──────────────┘ │
├──────────────────────────────────────────────┤
│              yt-dlp.exe (子进程)              │
└──────────────────────────────────────────────┘
```

### 数据流

1. **获取格式**：前端发送 URL → Rust 调用 `yt-dlp -J <url>` → 解析 JSON → 返回格式列表
2. **开始下载**：前端选择格式 → Rust 将任务加入队列 → 调度器按并发上限执行
3. **进度上报**：Rust 解析 yt-dlp 的 `--progress-template` 输出 → 通过 Tauri Event 推送到前端
4. **历史记录**：下载完成后写入 SQLite，前端可查询 / 搜索

## 环境要求

- **Node.js** >= 20
- **Rust** >= 1.77（通过 [rustup](https://rustup.rs/) 安装）
- **pnpm** 或 npm
- **Windows**：需要 Visual Studio Build Tools 或 Windows SDK
- **yt-dlp.exe**：放在项目根目录或加入 PATH

## 快速开始

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装 Tauri CLI
cargo install tauri-cli --version "^2"

# 3. 安装前端依赖
pnpm install

# 4. 下载 yt-dlp.exe 放到项目根目录
# 从 https://github.com/yt-dlp/yt-dlp/releases 下载最新版

# 5. 开发模式启动
pnpm tauri dev

# 6. 构建发布
pnpm tauri build
```

## 项目结构

```
yt-dlp-ui/
├── src/                    # React 前端
│   ├── App.tsx             # 主应用入口
│   ├── components/
│   │   ├── UrlInput.tsx    # URL 输入组件
│   │   ├── FormatPicker.tsx# 格式选择组件
│   │   ├── DownloadQueue.tsx# 下载队列列表
│   │   ├── DownloadItem.tsx# 单个下载项（进度条）
│   │   └── History.tsx     # 下载历史
│   ├── hooks/
│   │   ├── useDownload.ts  # 下载状态管理
│   │   └── useHistory.ts   # 历史记录查询
│   └── styles/
│       └── index.css       # Tailwind 入口
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 入口
│   │   ├── lib.rs          # 命令注册
│   │   ├── ytdlp.rs        # yt-dlp 进程调用
│   │   ├── queue.rs        # 下载队列调度
│   │   └── history.rs      # 历史记录 CRUD
│   ├── Cargo.toml
│   └── tauri.conf.json
├── yt-dlp.exe              # 下载引擎（不纳入版本控制）
├── package.json
├── tsconfig.json
├── tailwind.config.ts
└── README.md
```

## 配置

通过 `src-tauri/tauri.conf.json` 可调整：

- 最大并发下载数（默认 3）
- 默认下载目录
- yt-dlp 可执行文件路径
- 代理设置

## 许可证

MIT
