# darkroom

> 终端里的暗房——用 chafa 冲洗每一张照片。

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

**效率优先，功能其次。** darkroom 不做幻灯片、不做批量导出、不做滤镜调整。它只做一件事——用最快的方式让你在终端里看清图片。

- **主线程永不阻塞** — 所有 I/O 和编码都在后台线程执行
- **终端原生体验** — 就像 `ls` 或 `vim`，启动瞬间，操作即时
- **功能克制** — 每考虑一个新功能，先问"它会让浏览变慢吗？"

## 📦 安装

```bash
# 系统依赖
sudo apt install libchafa-dev

# 从源码编译
git clone https://github.com/user/darkroom.git
cd darkroom
cargo build --release

# 可选：创建别名
ln -s $(pwd)/target/release/darkroom ~/.local/bin/dr
```

## 🚀 使用

```bash
dr                   # 浏览当前目录
dr ~/图片             # 浏览指定目录
dr photo.jpg         # 直接打开单张图片（全屏模式）
```

## ⌨ 快捷键

| 模式 | 按键 | 功能 |
|------|------|------|
| 浏览器 | `←` `→` `↑` `↓` | 导航 |
| 浏览器 | `Space` `PgUp` `PgDn` | 翻页 |
| 浏览器 | `Home` `End` | 跳到首/尾 |
| 浏览器 | `Enter` | 全屏查看 |
| 浏览器 | `q` `Ctrl+C` | 退出 |
| 全屏 | `←` `→` | 切换图片 |
| 全屏 | `Enter` `Esc` `q` | 返回浏览器 |
| 全屏 | `Ctrl+C` | 退出 |

## 🔧 技术栈

| 依赖 | 用途 |
|------|------|
| [ratatui](https://ratatui.rs) | TUI 框架 |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | 图片 → Unicode 字符渲染 |
| [image](https://github.com/image-rs/image) | 图片解码（PNG/JPEG/WebP/GIF/BMP/TIFF/ICO） |
| [clap](https://github.com/clap-rs/clap) | CLI 参数解析 |

## 📜 许可证

MIT
