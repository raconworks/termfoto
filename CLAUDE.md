# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建与测试

```bash
cargo build              # 编译
cargo run                # 运行（默认当前目录）
cargo run -- <路径>      # 指定目录或图片文件
cargo test               # 运行所有测试
cargo test <测试名>      # 运行单个测试（模糊匹配）
cargo build --release    # release 构建
cargo clippy             # lint
cargo fmt                # 格式化
```

## 架构总览

darkroom 是一个终端图片浏览器，效率优先，功能其次。

**数据流**: CLI → 扫描目录 → `App` → 事件循环（键盘 + 渲染）。

**两个状态** (`AppState`):
- `Browser` — 8 列 chafa 缩略图，居中显示，文件名居中
- `Fullscreen` — 单张原图尺寸居中显示

**常量** (`app.rs`):
- `IMAGES_PER_ROW = 8`
- `CELL_HEIGHT = 10`（浏览器每格行数）

**模块**:

| 文件 | 职责 |
|------|------|
| `src/main.rs` | CLI（手动解析）、终端初始化、事件循环、按键分发 |
| `src/app.rs` | 导航、滚动、Protocol 缓存、后台加载通道管理 |
| `src/lang.rs` | 中英文翻译，L 键切换，$LANG 自动检测 |
| `src/scanner.rs` | 目录扫描、格式过滤（不递归）、按文件名排序 |
| `src/ui/mod.rs` | 状态分派：Browser / Fullscreen |
| `src/ui/browser.rs` | 8 列浏览器：chafa 缩略图懒加载 + 居中渲染 + 文件名居中 |
| `src/ui/preview.rs` | 全屏展示：原图尺寸居中，`Image` widget 渲染 |
| `src/ui/search.rs` | 增量搜索：`/` 触发，模糊匹配，Tab 切换结果 |

**关键设计**：
- **后台加载**：`spawn_image_loader()` 在独立线程中执行 `image::open()` + `picker.new_protocol()`（含 chafa 编码），主线程用 `try_recv()` 非阻塞收结果
- **Browser Protocol 缓存**：`HashMap<usize, Protocol>` 懒加载可见区域缩略图，终端 resize 时清空
- **居中渲染**：浏览器格内和全屏均计算 `offset = (area - proto_size) / 2` 居中放置

**依赖**：
- ratatui 0.30（TUI）+ crossterm 0.29（终端控制）
- ratatui-image 11（chafa-dyn，图片→终端字符）
- image 0.25（图片解码，仅 PNG/JPEG/WebP）
- anyhow 1（错误处理）

**系统依赖**：`libchafa-dev`（仅 `--features chafa` 时需要，默认零依赖）
