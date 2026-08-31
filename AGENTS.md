# AGENTS.md

## 核心原则

### 1. 每次变更必须测试

任何代码修改，必须先通过测试验证，再交付给用户。

- **后端变更**：运行 `cargo test --lib` 确保单元测试通过
- **前端变更**：运行 `npm run build` 确保 TypeScript 编译和 Vite 构建无错误
- **集成测试**：必要时运行 `cargo run --bin test_progress` 验证 yt-dlp 调用链路

### 2. 测试成功后再提交

变更通过测试后，立即提交到 Git：

```bash
git add -A
git commit -m "<type>: <description>"
```

提交信息遵循约定式提交格式：

- `feat:` — 新功能
- `fix:` — 修复 bug
- `refactor:` — 重构
- `test:` — 测试相关
- `chore:` — 构建/工具链

### 3. 不要猜测

如果对某个行为不确定，**写测试验证**，而不是假设代码正确。

## 项目结构

```
yt-dlp-ui/
├── src/                  # React 前端
│   ├── components/       # UI 组件
│   ├── store.ts          # Zustand 状态管理
│   └── types.ts          # 共享类型
├── src-tauri/            # Rust 后端
│   ├── src/
│   │   ├── lib.rs        # Tauri 命令注册
│   │   ├── ytdlp.rs      # yt-dlp 进程调用 & 进度解析
│   │   ├── queue.rs      # 下载队列调度
│   │   ├── history.rs    # 历史记录存储
│   │   └── bin/          # 独立测试二进制
│   └── tauri.conf.json
├── dist/                 # 前端构建产物
└── target/               # Rust 构建产物
```

## 快速命令

```bash
# 启动开发
npx tauri dev

# 后端测试
cd src-tauri && cargo test --lib

# 前端检查
npm run build

# 集成测试
cd src-tauri && cargo run --bin test_progress
```

## 已知坑

- **yt-dlp 进度在 stdout**：下载进度 `[download] XX%` 输出在 stdout，不是 stderr
- **JS 运行时**：yt-dlp 需要 deno 等 JS 运行时来提取 YouTube 视频格式，已用 `--extractor-args "youtube:player_client=android,ios"` 绕过
- **Tauri 事件**：`app_handle.emit()` 需要 `use tauri::Emitter;` 导入 trait