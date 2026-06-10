# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建与测试

```bash
cargo build              # 编译
cargo run                # 运行（默认当前目录）
cargo run -- <路径>      # 指定目录或图片文件
cargo test               # 运行所有测试
cargo test <测试名>      # 运行单个测试（模糊匹配）
cargo build --release    # release 构建（启用 LTO + 单 codegen-unit + strip + panic=abort）
cargo clippy             # lint
cargo fmt                # 格式化
```

## 架构总览

termfoto 是一个终端图片浏览器——做一件事，做到极致。

**数据流**: CLI（手动解析 args）→ 扫描目录 → `App` → 事件循环（键盘 + 渲染）。

**两个状态** (`AppState`):
- `Browser` — 动态列数网格 chafa 缩略图，居中显示，文件名居中，搜索高亮
- `Fullscreen` — 3:1 分屏（左侧原图居中 + 右侧信息面板），底部 logo + 状态栏

**关键常量** (`app.rs`):
- `MIN_CELL = 24` — 网格单元最小宽度，列数由终端宽度 / MIN_CELL 动态计算
- `LOGO_HEIGHT = 6` — 终端宽度 ≥ 70 时显示，否则隐藏
- `MIN_LOGO_WIDTH = 70` — 显示 logo 的最小终端宽度
- `MAX_CACHE_SIZE = 200` — Protocol 缓存上限，超出时淘汰最早一半条目

**动态网格**（`main.rs` 事件循环）:
- `cols = term_width / MIN_CELL`，最少 2 列
- `cell_h ≈ cell_w / char_ratio` 补偿终端字符 1:2 宽高比，实现视觉正方形
- 终端 resize 时清空全部 Protocol 缓存

**模块**:

| 文件 | 职责 |
|------|------|
| `src/main.rs` | CLI（手动解析）、`TermGuard` RAII 终端管理、事件循环、动态网格计算、Picker 初始化 |
| `src/app.rs` | 导航、滚动、全屏切换、Protocol 缓存（含淘汰策略）、4 线程后台加载通道管理、按键分发 |
| `src/lang.rs` | 中/英 UI 文本（状态栏、搜索提示、信息面板标签），L 键切换，$LANG 自动检测 |
| `src/scanner.rs` | 目录扫描（不递归）、按文件名排序。识别 9 种格式（PNG/JPG/JPEG/WebP/GIF/BMP/TIFF/ICO），但 `image` crate 仅解码 PNG/JPEG/WebP |
| `src/ui/mod.rs` | 状态分派 draw、彩虹渐变色 ASCII art logo 渲染（底部右对齐） |
| `src/ui/browser.rs` | 动态列网格渲染：居中、缩略图懒加载（±1 行预取）、文件名截断+居中、搜索匹配字符高亮 |
| `src/ui/preview.rs` | 全屏 3:1 分屏：左侧原图（`Image` widget 居中+裁剪）、右侧信息面板（文件/像素/大小/格式/路径） |
| `src/ui/search.rs` | 增量搜索：`/` 或 `\` 触发，模糊匹配+评分，Tab/Shift+Tab 切换结果，`SearchBar` widget |

**关键设计**：
- **后台加载**：`spawn_image_loader()` 启动 4 个 worker 线程，每个线程从共享 channel 取 `LoadRequest`、执行 `image::open()` + `picker.new_protocol()`，主线程用 `try_recv()` 非阻塞收结果
- **Browser Protocol 缓存**：`HashMap<usize, Protocol>` 懒加载可见行 ±1 行预取。终端 resize（宽高变化）时清空。超过 200 条时淘汰最早插入的一半
- **居中渲染**：浏览器格内和全屏均计算 `offset = (area - proto_size) / 2` 居中放置
- **搜索**：模糊匹配按得分排序（连续字符 + 靠前位置加分，间隔扣分），匹配字符在文件名中以黄色高亮

**依赖**：
- ratatui 0.30（TUI）+ crossterm 0.29（终端控制）
- ratatui-image 11（chafa-dyn，图片→终端字符；default features 关闭）
- image 0.25（图片解码，仅 PNG/JPEG/WebP features）
- anyhow 1（错误处理）
- tempfile 3（仅 dev-dependencies，测试用）

**feature flags**:
- `default = []` — 零系统依赖，使用终端内置协议（sixel/kitty）或 halfblocks
- `chafa` — 动态链接 libchafa（需 `libchafa-dev`），画质更佳
- `chafa-static` — 静态链接 chafa，用于预编译二进制发布
