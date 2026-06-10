[English](README.md)

# termfoto

> 快速轻量的终端图片浏览器——像专业人士一样看图。

[![build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com)
[![version](https://img.shields.io/badge/version-0.1.0-blue)](Cargo.toml)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![rust](https://img.shields.io/badge/rust-stable-orange)](https://rust-lang.org)

```
  ╭─────────────────────────────────────────────────╮
  │  🖼  img_0001.png  🖼  img_0002.png  ...        │
  │                                                 │
  │  8 列 chafa 缩略图 · vim 式导航                  │
  │  回车全屏 · 原图尺寸 · 零阻塞                    │
  │                                                 │
  │  photo_0042.png [42/156]  ←→↑↓ 导航  ...        │
  ╰─────────────────────────────────────────────────╯
```

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

## 📦 安装

**默认零依赖。** termfoto 使用终端内置协议（sixel/kitty）或 halfblocks 渲染，无需安装任何系统包。

> 💡 **想要更好的画质？** 安装 chafa 获得更优的 unicode 渲染：`cargo install --git https://github.com/PineWhisperStudio/termfoto --features chafa`（需要 `libchafa-dev`）。预编译二进制已静态链接 chafa——下载即用，无需依赖。

### 方式一：预编译二进制（推荐）

从 [Releases](https://github.com/PineWhisperStudio/termfoto/releases) 下载二进制，放到 `PATH` 中：

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### 方式二：.deb 包（Debian/Ubuntu）

```bash
curl -LO https://github.com/PineWhisperStudio/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### 方式三：Cargo（从 git 安装）

```bash
cargo install --git https://github.com/PineWhisperStudio/termfoto
```

### 方式四：从源码编译

```bash
git clone https://github.com/PineWhisperStudio/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/dr
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
```

## ⌨ 快捷键

| 模式 | 按键 | 功能 |
|------|------|------|
| 浏览器 | `←` `→` `↑` `↓` | 导航 |
| 浏览器 | `Space` `PgUp` `PgDn` | 翻页 |
| 浏览器 | `Home` `End` | 跳到首/尾 |
| 浏览器 | `Enter` | 全屏查看 |
| 浏览器 | `/` `\` | 搜索文件名 |
| 浏览器 | `L` | 切换语言 |
| 浏览器 | `q` `Ctrl+C` | 退出 |
| 全屏 | `←` `→` | 切换图片 |
| 全屏 | `L` | 切换语言 |
| 全屏 | `Enter` `Esc` `q` | 返回浏览器 |
| 全屏 | `Ctrl+C` | 退出 |

## 🔧 技术栈

| 依赖 | 用途 |
|------|------|
| [ratatui](https://ratatui.rs) | TUI 框架 |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | 图片 → Unicode 字符渲染 |
| [image](https://github.com/image-rs/image) | 图片解码（PNG/JPEG/WebP） |
| [clap](https://github.com/clap-rs/clap) | CLI 参数解析 |

## 📜 许可证

MIT
