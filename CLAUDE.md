# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 构建与测试

```bash
cargo build              # debug 编译
cargo run                # 运行（默认当前目录）
cargo run -- <路径>      # 指定目录或图片文件
cargo test               # 运行所有测试
cargo test <测试名>      # 运行单个测试（模糊匹配，如 cargo test animation）
cargo build --release    # release 构建（lto="fat", codegen-units=1, opt-level="z", strip, panic="abort"）
cargo clippy -- -D warnings # lint（与 CI 一致）
cargo fmt                # 格式化
```

release profile 配置（`Cargo.toml`）：
- `lto = "fat"` — 跨 crate 的激进链接时优化
- `codegen-units = 1` — 单代码生成单元，最大化内联
- `opt-level = "z"` — 优化体积
- `strip = true` — 剥离符号表
- `panic = "abort"` — panic 直接中止，减小体积

## 提交规范

提交信息使用英文，并保持 Conventional Commit 格式，例如 `fix: handle empty image directory`。动词使用祈使语气，提交范围保持聚焦。

## 架构总览

termfoto 是一个终端图片浏览器——做一件事，做到极致。

**数据流**: CLI（手动解析 args）→ 扫描目录 → `App` → 事件循环（键盘 + 渲染）。

**两个状态** (`AppState`):
- `Browser` — 动态列数网格 chafa 缩略图，居中显示，文件名居中，搜索高亮
- `Fullscreen` — 3:1 分屏（左侧图片视口 + 右侧信息面板），底部 logo + 状态栏。自动检测动图（GIF/APNG/WebP）并循环播放。静态图支持 `+`/`=`/`-` 缩放（100%~1000%，步长 10%）、`h` `j` `k` `l` 平移、`0` 重置；动图忽略缩放

**关键常量** (`app.rs`):
- `MIN_CELL = 24` — 网格单元最小宽度，列数由终端宽度 / MIN_CELL 动态计算
- `LOGO_HEIGHT = 3` — 紧凑 3 行 logo（右下角，每行不同彩虹色）
- `MIN_LOGO_WIDTH = 70` — 终端宽度 ≥ 70 时显示紧凑 3 行 logo，否则隐藏
- `MAX_CACHE_SIZE = 200` — Protocol 缓存上限，超出时淘汰最早一半条目
- `ZOOM_STEP = 0.10`, `ZOOM_MIN = 1.0`, `ZOOM_MAX = 10.0` — 缩放参数
- `FULLSCREEN_ORIGINAL_CACHE_BYTES = 128MB` — 全屏静态原图 RGBA LRU 缓存预算
- `FULLSCREEN_RENDER_CACHE_SIZE = 8` — 全屏渲染 Protocol LRU 缓存条目数
- `INTERACTIVE_SETTLE_DELAY = 120ms` — 缩放/平移交互停止后补高质量渲染的延迟
- `DIRECT_FINAL_RENDER_PIXELS = 1_000_000` — 内容/视口 dirty 可直接高质量渲染的目标像素阈值

**动态网格**（`main.rs` 事件循环）:
- `cols = term_width / MIN_CELL`，最少 2 列
- `cell_h ≈ cell_w / char_ratio` 补偿终端字符 1:2 宽高比，实现视觉正方形
- 终端 resize 时清空全部 Protocol 缓存

**模块**:

| 文件 | 职责 |
|------|------|
| `src/main.rs` | CLI（手动解析）、`TermGuard` RAII 终端管理、事件循环、动态网格计算、Picker 初始化 |
| `src/app.rs` | 导航、滚动、全屏切换、Protocol/LRU 缓存、后台加载、全屏 render worker、按键分发 |
| `src/lang.rs` | 中/英 UI 文本（状态栏、搜索提示、信息面板标签），L 键切换，$LANG 自动检测 |
| `src/scanner.rs` | 目录扫描（不递归）、按文件名排序。识别 9 种格式（PNG/JPG/JPEG/WebP/GIF/BMP/TIFF/ICO），但非图片格式的文件在加载时会被跳过 |
| `src/ui/mod.rs` | 状态分派 draw、彩虹渐变色 ASCII art logo 渲染（底部右对齐） |
| `src/ui/browser.rs` | 动态列网格渲染：居中、缩略图懒加载（可见格优先，前后各 1 行预取）、文件名截断+居中、搜索匹配字符高亮 |
| `src/ui/preview.rs` | 全屏 3:1 分屏：左侧图片视口（已渲染 Protocol 居中）、右侧信息面板（文件/像素/大小/格式/路径） |
| `src/ui/search.rs` | 增量搜索：`/` 或 `\` 触发，模糊匹配+评分，Tab/Shift+Tab 切换结果，`SearchBar` widget |

**关键设计**：
- **后台加载**：`spawn_image_loader()` 将 `LoadRequest` 按 `Thumbnail` / `Original` 分队列，缩略图 3 个 worker、原图 1 个 worker，主线程用 `try_recv()` 非阻塞收结果。缩略图只返回 Protocol；原图先用 `image_dimensions()` 取尺寸，仅 GIF/PNG/WebP 走动画探测，静态图解码后立即转为 `RgbaImage`
- **Browser Protocol 缓存**：`HashMap<usize, Protocol>` 懒加载当前可见格，随后预取上一行/下一行。终端 resize（宽高变化）时清空。超过 200 条时淘汰一批旧条目
- **全屏动图播放**：`FullscreenContent` 枚举区分静态 (`Static`) 与动画 (`Animation(Vec<AnimationFrame>)`)。支持 GIF、APNG、Animated WebP。帧上限 120，默认帧间隔 100ms，最小 20ms。`try_decode_animation()` 解码帧序列。事件循环通过 `next_animation_deadline()` / `advance_animation()` 驱动帧切换，`event::poll(timeout)` 在无按键时等待下一帧到期
- **全屏缩放**：`StaticContent` 持有缓存好的原始 `RgbaImage`。render worker 使用 `fast_image_resize` 对 RGBA buffer 做 crop/resize，再交给 `picker.new_protocol()`。缩放/平移先提交 `Interactive` 质量（nearest），120ms 后补 `Final` 质量（Lanczos3）；内容加载和视口变化在小目标像素时可直接 `Final`
- **全屏缓存**：静态原图按 RGBA 字节数进入 128MB LRU；全屏渲染结果按 `RenderKey` 进入 8 条 Protocol LRU。render worker 会 drain 队列，只渲染最新请求，并用 generation/key 丢弃过期结果
- **居中渲染**：浏览器格内和全屏均计算 `offset = (area - proto_size) / 2` 居中放置
- **搜索**：模糊匹配按得分排序（连续字符 + 靠前位置加分，间隔扣分），匹配字符在文件名中以黄色高亮

**依赖**：
- ratatui 0.30（TUI）+ crossterm 0.29（终端控制）
- ratatui-image 11（图片→终端字符；default features 关闭，`chafa` feature 可启用 chafa-dyn）
- image 0.25（图片解码，PNG/JPEG/WebP/GIF features；GIF 用于动图支持）
- fast_image_resize 6（全屏缩放/裁剪重采样）
- lru 0.18（全屏原图和渲染结果缓存）
- anyhow 1（错误处理）
- tempfile 3（仅 dev-dependencies，测试用）

**feature flags**:
- `default = []` — 零系统依赖，使用终端内置协议（sixel/kitty）或 halfblocks
- `chafa` — 动态链接 libchafa（需 `libchafa-dev`），画质更佳
- `chafa-static` — 静态链接 chafa，用于预编译二进制发布

**项目结构（非 src）**:

| 路径 | 用途 |
|------|------|
| `npm/` | npm 薄包装（package.json + install.js），CI 发布时自动更新版本 |
| `.github/workflows/ci.yml` | push/PR 触发 build + test + clippy |
| `.github/workflows/release.yml` | tag 触发：构建二进制 + .deb + crates.io 发布 + npm publish |
| `assets/` | README demo.gif 等静态资源 |
