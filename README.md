[中文版](README.zh.md)

# darkroom

> A darkroom for your terminal — develop every photo with chafa.

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

## ✨ Features

| | |
|---|---|
| 🎨 **High-quality chafa rendering** | Unicode chars + true color half-blocks — a magnitude sharper than sixel |
| ⚡ **Non-blocking async loading** | Image decoding & chafa encoding run on background thread — zero UI jank |
| 🖼 **Original-size fullscreen** | No scaling, no distortion — pixel-accurate, centered display |
| ⌨ **Keyboard-only navigation** | Vim-style bindings — keep your hands where they matter |
| 🪶 **Extremely lightweight** | No GUI framework — only 4 core dependencies |
| 📂 **Instant startup** | No indexing, no recursive scanning, no metadata cache — open and browse |

## 🎯 Design Philosophy

**Efficiency first, features second.** darkroom is not a slideshow, a batch exporter, or a photo editor. It does one thing — lets you see your images in the terminal, as fast as possible.

- **Main thread never blocks** — all I/O and encoding run on background threads
- **Terminal-native experience** — like `ls` or `vim`: launches instantly, responds immediately
- **Ruthlessly minimal** — before adding a feature, ask: "will it make browsing slower?"

## 📦 Installation

### System dependency

All methods require `libchafa`:

```bash
# Debian/Ubuntu
sudo apt install libchafa-dev

# Arch
sudo pacman -S chafa

# macOS
brew install chafa
```

### Option 1: Cargo (recommended, requires Rust)

```bash
cargo install darkroom
```

### Option 2: Prebuilt binary

Download from [Releases](https://github.com/boyangso/darkroom/releases), drop into `PATH`:

```bash
chmod +x darkroom
sudo cp darkroom /usr/local/bin/
```

### Option 3: .deb package (Debian/Ubuntu)

```bash
curl -LO https://github.com/boyangso/darkroom/releases/latest/download/darkroom_latest_amd64.deb
sudo apt install ./darkroom_latest_amd64.deb  # auto-resolves libchafa dependency
```

### Option 4: Build from source

```bash
git clone https://github.com/boyangso/darkroom.git
cd darkroom
cargo build --release
ln -s $(pwd)/target/release/darkroom ~/.local/bin/dr
```

### Optional alias

```bash
# Add to ~/.bashrc or ~/.config/fish/config.fish
alias dr='darkroom'
```

## 🚀 Usage

```bash
darkroom                 # browse current directory
darkroom ~/Pictures      # browse a directory
darkroom photo.jpg       # open a single image (fullscreen mode)
```

## ⌨ Keybindings

| Mode | Key | Action |
|------|-----|--------|
| Browser | `←` `→` `↑` `↓` | Navigate |
| Browser | `Space` `PgUp` `PgDn` | Page up/down |
| Browser | `Home` `End` | Jump to first/last |
| Browser | `Enter` | View fullscreen |
| Browser | `q` `Ctrl+C` | Quit |
| Fullscreen | `←` `→` | Previous/next image |
| Fullscreen | `Enter` `Esc` `q` | Back to browser |
| Fullscreen | `Ctrl+C` | Quit |

## 🔧 Tech Stack

| Dependency | Purpose |
|------------|---------|
| [ratatui](https://ratatui.rs) | TUI framework |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | Image → Unicode character rendering |
| [image](https://github.com/image-rs/image) | Image decoding (PNG/JPEG/WebP/GIF/BMP/TIFF/ICO) |
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |

## 📜 License

MIT
