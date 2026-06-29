[English](README.md)

# termfoto

> 像终端一样快地浏览图片。

[![CI](https://github.com/raconworks/termfoto/actions/workflows/ci.yml/badge.svg)](https://github.com/raconworks/termfoto/actions/workflows/ci.yml)
[![crates.io](https://badgen.net/crates/v/termfoto)](https://crates.io/crates/termfoto)
[![downloads](https://badgen.net/crates/d/termfoto?label=downloads)](https://crates.io/crates/termfoto)
[![npm](https://img.shields.io/npm/dt/termfoto?label=npm)](https://www.npmjs.com/package/termfoto)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)

![termfoto demo](assets/demo.gif)

## ✨ 特性

| | |
|---|---|
| 🎨 **chafa 高质量渲染** | Unicode 字符 + true color 半块，比 sixel 清晰一个量级 |
| ⚡ **零阻塞异步加载** | 缩略图加载、原图解码和全屏渲染都在后台线程执行 |
| 🖼 **全屏缩放与平移** | 自适应窗口显示，交互缩放/平移响应快，停止后自动补高质量渲染 |
| 🧭 **三栏工作区** | 上级目录、图片浏览、文件信息三栏常驻，底部保留三行提示栏 |
| ⌨ **纯键盘操作** | Vim 式导航，手不离键盘，操作即响应 |
| 🪶 **极致轻量** | 无 GUI 框架，依赖保持小而专注 |
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

### npm

```bash
npm install -g termfoto
```

### Cargo

```bash
cargo install termfoto
```

### 预编译二进制

从 [Releases](https://github.com/raconworks/termfoto/releases) 下载二进制，放到 `PATH` 中：

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### .deb 包（Debian/Ubuntu）

```bash
curl -LO https://github.com/raconworks/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### 从源码编译

```bash
git clone https://github.com/raconworks/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/termfoto
```

> 💡 **创建别名：** 在 `~/.bashrc` 或 `~/.config/fish/config.fish` 中添加 `alias dr='termfoto'`。

## 🚀 使用

```bash
termfoto                 # 浏览当前目录
termfoto ~/图片           # 浏览指定目录
termfoto photo.jpg       # 直接打开单张图片（全屏模式）
termfoto --help          # 显示所有选项
termfoto --version       # 显示版本号
```

## 🧩 界面布局

termfoto 在浏览和全屏模式中使用同一套布局：左侧只读 **上级目录**，中间 **图片浏览**，右侧 **文件信息**。终端最底部固定保留三行，用于显示模式提示或搜索输入，logo 固定在右侧。

浏览模式下，上级目录显示图片集合所在文件夹的父目录，并高亮该文件夹。全屏模式下，上级目录显示当前图片所在文件夹，并高亮当前文件。

文件信息栏会显示文件名、可用时的图片尺寸、大小、格式、修改/创建时间和路径。

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
| 全屏 | `+` · `=` | 放大 |
| 全屏 | `-` | 缩小 |
| 全屏 | `0` | 重置缩放和平移 |
| 全屏 | `h` `j` `k` `l` | 左/下/上/右平移 |
| 全屏 | `L` | 切换中/英文 |
| 全屏 | `Enter` · `Esc` · `q` | 返回浏览器 |
| 全屏 | `Ctrl+C` | 退出 |

## 🔧 技术栈

| 依赖 | 用途 |
|------|------|
| [ratatui](https://ratatui.rs) | TUI 框架 |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | 图片 → Unicode 字符渲染 |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 终端输入和 raw mode 控制 |
| [image](https://github.com/image-rs/image) | 图片解码（PNG/JPEG/WebP/GIF） |
| [fast_image_resize](https://github.com/Cykooz/fast_image_resize) | 全屏交互裁剪/缩放快速重采样 |
| [lru](https://github.com/jeromefroe/lru-rs) | 有界全屏原图和渲染缓存 |

## 📜 许可证

MIT

## 🌟 喜欢 termfoto？

- ⭐ **给个 Star** — 让更多人发现它
- 🐛 **报告 Bug** — [GitHub Issues](https://github.com/raconworks/termfoto/issues)
- 💡 **建议新功能** — 先问自己：*"它会让浏览变慢吗？"*

---

📦 **也可在** [crates.io](https://crates.io/crates/termfoto) · [GitHub Releases](https://github.com/raconworks/termfoto/releases) 获取

---

用 ❤️ 由 [raconworks](https://github.com/raconworks) 打造
