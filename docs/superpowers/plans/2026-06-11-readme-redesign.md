# README 推广优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 README 从模板占位提升为具备发现-试用-传播能力的项目门面

**Architecture:** 修改 4 个文件：新建 `ci.yml` 提供真实 badge，修改 `Cargo.toml` 统一描述和仓库 URL，重写 `README.md` 和 `README.zh.md`

**Tech Stack:** Markdown, GitHub Actions, shields.io, asciinema

**前置手动任务:** 录制 asciinema 录屏（30 秒：启动→浏览→搜索→全屏→切换→退出），上传到 asciinema.org 获取链接

---

### Task 1: 创建最小 CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: 创建 CI workflow 文件**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: 安装 Rust
        uses: dtolnay/rust-toolchain@stable

      - name: 安装系统依赖
        run: |
          sudo apt-get update -qq
          sudo apt-get install -y libchafa-dev libglib2.0-dev

      - name: 编译
        run: cargo build --no-default-features

      - name: 测试
        run: cargo test --no-default-features

      - name: Clippy
        run: cargo clippy --no-default-features -- -D warnings
```

- [ ] **Step 2: 提交**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: 添加最小 CI workflow（build + test + clippy）"
```

---

### Task 2: 修改 Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 改 description 为英文、统一 repository URL**

找到并替换：

```toml
description = "终端图片浏览器——快速、轻量、Vim 式操作"
repository = "https://github.com/boyangso/termfoto"
```

改为：

```toml
description = "Fast terminal photo viewer — keyboard-driven, chafa-rendered"
repository = "https://github.com/raconworks/termfoto"
```

- [ ] **Step 2: 验证**

```bash
cargo metadata --format-version=1 --no-deps | grep -E '"description"|"repository"'
```

应输出：

```
"description": "Fast terminal photo viewer — keyboard-driven, chafa-rendered"
"repository": "https://github.com/raconworks/termfoto"
```

- [ ] **Step 3: 提交**

```bash
git add Cargo.toml
git commit -m "docs: Cargo.toml description 改为英文，统一 repo URL"
```

---

### Task 3: 重写 README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: 替换 Hero 区**

**旧代码** (第 1-21 行):

```markdown
[中文版](README.zh.md)

# termfoto

> Fast, lightweight terminal photo viewer — browse images like a pro.

[![build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com)
[![version](https://img.shields.io/badge/version-0.1.0-blue)](Cargo.toml)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![rust](https://img.shields.io/badge/rust-stable-orange)](https://rust-lang.org)

```
  ╭─────────────────────────────────────────────────╮
  │  🖼  img_0001.png  🖼  img_0002.png  ...        │
  │                                                 │
  │  8-column chafa thumbnails · vim-style nav      │
  │  Enter fullscreen · original size · zero-block  │
  │                                                 │
  │  photo_0042.png [42/156]  ←→↑↓ navigate  ...   │
  ╰─────────────────────────────────────────────────╯
```
```

**新代码**:

```markdown
[中文版](README.zh.md)

# termfoto

> Browse images at the speed of your terminal.

[![CI](https://github.com/raconworks/termfoto/actions/workflows/ci.yml/badge.svg)](https://github.com/raconworks/termfoto/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/termfoto?label=crates.io)](https://crates.io/crates/termfoto)
[![downloads](https://img.shields.io/crates/d/termfoto?label=downloads)](https://crates.io/crates/termfoto)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)

<!-- TODO: 替换为实际 asciinema 链接 -->
[![asciicast](https://asciinema.org/a/PLACEHOLDER.svg)](https://asciinema.org/a/PLACEHOLDER)
```

- [ ] **Step 2: 在特性表格后插入「为什么不用其他工具」**

在 `## 🎯 Design Philosophy` 段落后、`## 📦 Installation` 之前插入：

```markdown
## 🤔 Why not other tools?

| Tool | For | termfoto instead |
|------|-----|-----------------|
| [`viu`](https://github.com/atanunq/viu) | Single image preview | Directory browsing + keyboard nav + fullscreen |
| [`timg`](https://github.com/hzeller/timg) | Image/video playback | Focused on images, faster startup, lighter |
| [`ranger`](https://github.com/ranger/ranger) / [`lf`](https://github.com/gokcehan/lf) | File manager | Image-first, interactive browsing |

```

- [ ] **Step 3: 修复安装区 bug**

找到：

```bash
ln -s $(pwd)/target/release/termfoto ~/.local/bin/dr
```

替换为：

```bash
ln -s $(pwd)/target/release/termfoto ~/.local/bin/termfoto
```

- [ ] **Step 4: 使用区加 --help / --version**

在 `## 🚀 Usage` 代码块末尾追加两行：

```markdown
```bash
termfoto                 # browse current directory
termfoto ~/Pictures      # browse a directory
termfoto photo.jpg       # open a single image (fullscreen mode)
termfoto --help          # show all options
termfoto --version       # print version
```
```

- [ ] **Step 5: 替换快捷键表**

**旧代码**:

```markdown
## ⌨ Keybindings

| Mode | Key | Action |
|------|-----|--------|
| Browser | `←` `→` `↑` `↓` | Navigate |
| Browser | `Space` `PgUp` `PgDn` | Page up/down |
| Browser | `Home` `End` | Jump to first/last |
| Browser | `Enter` | View fullscreen |
| Browser | `/` `\` | Search filenames |
| Browser | `L` | Toggle language |
| Browser | `q` `Ctrl+C` | Quit |
| Fullscreen | `←` `→` | Previous/next image |
| Fullscreen | `L` | Toggle language |
| Fullscreen | `Enter` `Esc` `q` | Back to browser |
| Fullscreen | `Ctrl+C` | Quit |
```

**新代码**:

```markdown
## ⌨ Keybindings

| Mode | Key | Action |
|------|-----|--------|
| Browser | `←` `→` `↑` `↓` | Navigate |
| Browser | `Space` · `PgDn` | Page down |
| Browser | `PgUp` | Page up |
| Browser | `Home` · `End` | Jump first/last |
| Browser | `Enter` | Fullscreen |
| Browser | `/` · `\` | Search filenames |
| Browser | `L` | Toggle EN/中文 |
| Browser | `q` · `Ctrl+C` | Quit |
| Search | `Esc` | Cancel search |
| Search | `Tab` · `Shift+Tab` | Next/prev match |
| Search | `Enter` | Fullscreen match |
| Fullscreen | `←` `→` | Prev/next image |
| Fullscreen | `L` | Toggle EN/中文 |
| Fullscreen | `Enter` · `Esc` · `q` | Back to browser |
| Fullscreen | `Ctrl+C` | Quit |
```

- [ ] **Step 6: 修正技术栈**

找到并删除 clap 行：

```markdown
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |
```

- [ ] **Step 7: 替换许可证区（追加 CTA）**

**旧代码**:

```markdown
## 📜 License

MIT
```

**新代码**:

```markdown
## 🌟 Like termfoto?

- ⭐ **Star this repo** — helps others discover it
- 🐛 **Report bugs** — [GitHub Issues](https://github.com/raconworks/termfoto/issues)
- 💡 **Suggest features** — before requesting, ask: *"will it make browsing slower?"*

---

📦 **Also available on** [crates.io](https://crates.io/crates/termfoto) · [GitHub Releases](https://github.com/raconworks/termfoto/releases)

---

Made with ❤️ by [raconworks](https://github.com/raconworks)

## 📜 License

MIT
```

- [ ] **Step 8: 提交**

```bash
git add README.md
git commit -m "docs: 重写 README——真实 badge + asciinema + 竞品对比 + CTA"
```

---

### Task 4: 同步 README.zh.md

**Files:**
- Modify: `README.zh.md`

- [ ] **Step 1: 对照 README.md 逐区同步中文翻译**

改动点与英文版一致：

1. slogan → `"像终端一样快地浏览图片"`
2. badge 替换为真实 badge（与英文版相同的 shields.io URL）
3. asciinema 占位（与英文版相同）
4. 插入「为什么不用其他工具」表格（中文翻译）
5. 修复 `dr` → `termfoto`
6. 使用区加 `--help` / `--version`
7. 快捷键表替换（搜索模式 3 行中文）
8. 删除 clap 行
9. 追加 CTA 区（中文翻译）

完整内容：

```markdown
[English](README.md)

# termfoto

> 像终端一样快地浏览图片。

[![CI](https://github.com/raconworks/termfoto/actions/workflows/ci.yml/badge.svg)](https://github.com/raconworks/termfoto/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/termfoto?label=crates.io)](https://crates.io/crates/termfoto)
[![downloads](https://img.shields.io/crates/d/termfoto?label=downloads)](https://crates.io/crates/termfoto)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)

<!-- TODO: 替换为实际 asciinema 链接 -->
[![asciicast](https://asciinema.org/a/PLACEHOLDER.svg)](https://asciinema.org/a/PLACEHOLDER)

## ✨ 特性

| | |
|---|---|
| 🎨 **chafa 高质量渲染** | Unicode 字符 + true color 半块，比 sixel 清晰一个量级 |
| ⚡ **零阻塞异步加载** | 图片解码和 chafa 编码全在后台线程，滚动丝滑不卡 UI |
| 🖼 **原图尺寸全屏** | 不缩放、不失真，像素级精确居中显示 |
| ⌨ **纯键盘操作** | Vim 式导航，手不离键盘，操作即响应 |
| 🪶 **极致轻量** | 无 GUI 框架，核心依赖仅 4 个 crate |
| 📂 **即时启动** | 不建索引、不扫子目录、不缓存元数据，打开即浏览 |

## 🎯 设计哲学

**做一件事，做到极致。** termfoto 不做幻灯片、不做批量导出、不做滤镜调整。它只做一件事——用最快的方式让你在终端里看清图片。

- **主线程永不阻塞** — 所有 I/O 和编码都在后台线程执行
- **终端原生体验** — 就像 `ls` 或 `vim`，启动瞬间，操作即时
- **功能克制** — 每考虑一个新功能，先问"它会让浏览变慢吗？"

## 🤔 为什么不用其他工具？

| 工具 | 定位 | termfoto 差异 |
|------|------|-------------|
| [`viu`](https://github.com/atanunq/viu) | 单图预览 | 目录浏览 + 键盘导航 + 全屏 |
| [`timg`](https://github.com/hzeller/timg) | 图片/视频播放 | 专注图片，启动更快更轻 |
| [`ranger`](https://github.com/ranger/ranger) / [`lf`](https://github.com/gokcehan/lf) | 文件管理器 | 图片优先，交互浏览 |

## 📦 安装

**默认零依赖。** termfoto 使用终端内置协议（sixel/kitty）或 halfblocks 渲染，无需安装任何系统包。

> 💡 **想要更好的画质？** 安装 chafa 支持：`cargo install termfoto --features chafa`（需要 `libchafa-dev`）。预编译二进制已静态链接 chafa——下载即用，无需依赖。

### 方式一：Cargo（推荐）

```bash
cargo install termfoto
```

### 方式二：预编译二进制

从 [Releases](https://github.com/raconworks/termfoto/releases) 下载二进制，放到 `PATH` 中：

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### 方式三：.deb 包（Debian/Ubuntu）

```bash
curl -LO https://github.com/raconworks/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### 方式四：从源码编译

```bash
git clone https://github.com/raconworks/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/termfoto
```

### 创建别名

```bash
# 可选：在 ~/.bashrc 或 ~/.config/fish/config.fish 中添加
alias dr='termfoto'
```

## 🚀 使用

```bash
termfoto                 # 浏览当前目录
termfoto ~/图片           # 浏览指定目录
termfoto photo.jpg       # 直接打开单张图片（全屏模式）
termfoto --help          # 显示所有选项
termfoto --version       # 显示版本号
```

## ⌨ 快捷键

| 模式 | 按键 | 功能 |
|------|------|------|
| 浏览器 | `←` `→` `↑` `↓` | 导航 |
| 浏览器 | `Space` · `PgDn` | 下翻页 |
| 浏览器 | `PgUp` | 上翻页 |
| 浏览器 | `Home` · `End` | 跳到首/尾 |
| 浏览器 | `Enter` | 全屏查看 |
| 浏览器 | `/` · `\` | 搜索文件名 |
| 浏览器 | `L` | 切换中/英文 |
| 浏览器 | `q` · `Ctrl+C` | 退出 |
| 搜索 | `Esc` | 取消搜索 |
| 搜索 | `Tab` · `Shift+Tab` | 上/下一个结果 |
| 搜索 | `Enter` | 全屏当前结果 |
| 全屏 | `←` `→` | 上/下一张 |
| 全屏 | `L` | 切换中/英文 |
| 全屏 | `Enter` · `Esc` · `q` | 返回浏览器 |
| 全屏 | `Ctrl+C` | 退出 |

## 🔧 技术栈

| 依赖 | 用途 |
|------|------|
| [ratatui](https://ratatui.rs) | TUI 框架 |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | 图片 → Unicode 字符渲染 |
| [image](https://github.com/image-rs/image) | 图片解码（PNG/JPEG/WebP） |

## 🌟 喜欢 termfoto？

- ⭐ **给个 Star** — 让更多人发现它
- 🐛 **报告 Bug** — [GitHub Issues](https://github.com/raconworks/termfoto/issues)
- 💡 **建议新功能** — 先问自己：*"它会让浏览变慢吗？"*

---

📦 **也可在** [crates.io](https://crates.io/crates/termfoto) · [GitHub Releases](https://github.com/raconworks/termfoto/releases)

---

用 ❤️ 由 [raconworks](https://github.com/raconworks) 打造

## 📜 许可证

MIT
```

- [ ] **Step 2: 提交**

```bash
git add README.zh.md
git commit -m "docs: 同步 README.zh.md——中文翻译与英文版一致"
```

---

### Task 5: 最终提交

- [ ] **Step 1: 确认所有改动**

```bash
git status
git log --oneline -5
```

应看到 4 个 commit：
1. `ci: 添加最小 CI workflow`
2. `docs: Cargo.toml description 改为英文，统一 repo URL`
3. `docs: 重写 README——真实 badge + asciinema + 竞品对比 + CTA`
4. `docs: 同步 README.zh.md`

- [ ] **Step 2: 推送**

```bash
git push
```

---

## 实施后

- [ ] 录制 asciinema 录屏，上传后替换两处 `PLACEHOLDER`
- [ ] 首次推送 CI workflow 后，确认 badge 显示正常（GitHub Actions 页面会有绿色 badge）
- [ ] 首次发布到 crates.io 后，确认 version/downloads badge 显示正常
