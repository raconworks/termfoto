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

**Do one thing, and do it well.** termfoto is not a slideshow, a batch exporter, or a photo editor. It does one thing — lets you see your images in the terminal, as fast as possible.

- **Main thread never blocks** — all I/O and encoding run on background threads
- **Terminal-native experience** — like `ls` or `vim`: launches instantly, responds immediately
- **Ruthlessly minimal** — before adding a feature, ask: "will it make browsing slower?"

## 📦 Installation

**Zero dependencies by default.** termfoto uses your terminal's built-in protocols (sixel/kitty) or halfblocks rendering with no system packages required.

> 💡 **Want even better quality?** Install chafa for superior unicode rendering: `cargo install --git https://github.com/PineWhisperStudio/termfoto --features chafa` (requires `libchafa-dev`). Prebuilt binaries already include chafa statically — download and run, no deps needed.

### Option 1: Prebuilt binary (recommended)

Download from [Releases](https://github.com/PineWhisperStudio/termfoto/releases), drop into `PATH`:

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### Option 2: .deb package (Debian/Ubuntu)

```bash
curl -LO https://github.com/PineWhisperStudio/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### Option 3: Cargo (from git)

```bash
cargo install --git https://github.com/PineWhisperStudio/termfoto
```

### Option 4: Build from source

```bash
git clone https://github.com/PineWhisperStudio/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/dr
```

### Optional alias

```bash
# Add to ~/.bashrc or ~/.config/fish/config.fish
alias dr='termfoto'
```

## 🚀 Usage

```bash
termfoto                 # browse current directory
termfoto ~/Pictures      # browse a directory
termfoto photo.jpg       # open a single image (fullscreen mode)
```

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

## 🔧 Tech Stack

| Dependency | Purpose |
|------------|---------|
| [ratatui](https://ratatui.rs) | TUI framework |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | Image → Unicode character rendering |
| [image](https://github.com/image-rs/image) | Image decoding (PNG/JPEG/WebP/GIF/BMP/TIFF/ICO) |
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |

## 📜 License

MIT
