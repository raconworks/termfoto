[中文版](README.zh.md)

# termfoto

> Browse images at the speed of your terminal.

[![CI](https://github.com/PineWhisperStudio/termfoto/actions/workflows/ci.yml/badge.svg)](https://github.com/PineWhisperStudio/termfoto/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/termfoto?label=crates.io)](https://crates.io/crates/termfoto)
[![downloads](https://img.shields.io/crates/d/termfoto?label=downloads)](https://crates.io/crates/termfoto)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)

![termfoto demo](assets/demo.gif)

## ✨ Features

| | |
|---|---|
| 🎨 **High-quality chafa rendering** | Unicode chars + true color half-blocks — a magnitude sharper than sixel |
| ⚡ **Non-blocking async loading** | Image decoding & chafa encoding run on background threads — zero UI jank |
| 🖼 **Original-size fullscreen** | No scaling, no distortion — pixel-accurate, centered display |
| ⌨ **Keyboard-only navigation** | Vim-style bindings — keep your hands where they matter |
| 🪶 **Extremely lightweight** | No GUI framework — only 4 core dependencies |
| 📂 **Instant startup** | No indexing, no recursive scanning, no metadata cache — open and browse |

## 🎯 Design Philosophy

**Do one thing, and do it well.** termfoto is not a slideshow, a batch exporter, or a photo editor. It does one thing — lets you see your images in the terminal, as fast as possible.

- **Main thread never blocks** — all I/O and encoding run on background threads
- **Terminal-native experience** — like `ls` or `vim`: launches instantly, responds immediately
- **Ruthlessly minimal** — before adding a feature, ask: "will it make browsing slower?"

## 🤔 Why not other tools?

| Tool | For | termfoto instead |
|------|-----|-----------------|
| [`viu`](https://github.com/atanunq/viu) | Single image preview | Directory browsing + keyboard nav + fullscreen |
| [`timg`](https://github.com/hzeller/timg) | Image/video playback | Focused on images, faster startup, lighter |
| [`ranger`](https://github.com/ranger/ranger) / [`lf`](https://github.com/gokcehan/lf) | File manager | Image-first, interactive browsing |

## 📦 Installation

**Zero dependencies by default.** termfoto uses your terminal's built-in protocols (sixel/kitty) or halfblocks rendering with no system packages required.

> 💡 **Want even better quality?** Install with chafa support: `cargo install termfoto --features chafa` (requires `libchafa-dev`). Prebuilt binaries include chafa statically — download and run, no deps needed.

### npm

```bash
npm install -g termfoto
```

### Cargo

```bash
cargo install termfoto
```

### Prebuilt binary

Download from [Releases](https://github.com/PineWhisperStudio/termfoto/releases), drop into `PATH`:

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### .deb package (Debian/Ubuntu)

```bash
curl -LO https://github.com/PineWhisperStudio/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### Build from source

```bash
git clone https://github.com/PineWhisperStudio/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/termfoto
```

> 💡 **Optional alias:** add `alias dr='termfoto'` to `~/.bashrc` or `~/.config/fish/config.fish`.

## 🚀 Usage

```bash
termfoto                 # browse current directory
termfoto ~/Pictures      # browse a directory
termfoto photo.jpg       # open a single image (fullscreen mode)
termfoto --help          # show all options
termfoto --version       # print version
```

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

## 🔧 Tech Stack

| Dependency | Purpose |
|------------|---------|
| [ratatui](https://ratatui.rs) | TUI framework |
| [ratatui-image](https://github.com/ratatui/ratatui-image) + [chafa](https://hpjansson.org/chafa/) | Image → Unicode character rendering |
| [image](https://github.com/image-rs/image) | Image decoding (PNG/JPEG/WebP) |

## 📜 License

MIT

## 🌟 Like termfoto?

- ⭐ **Star this repo** — helps others discover it
- 🐛 **Report bugs** — [GitHub Issues](https://github.com/PineWhisperStudio/termfoto/issues)
- 💡 **Suggest features** — before requesting, ask: *"will it make browsing slower?"*

---

📦 **Also available on** [crates.io](https://crates.io/crates/termfoto) · [GitHub Releases](https://github.com/PineWhisperStudio/termfoto/releases)

---

Made with ❤️ by [PineWhisperStudio](https://github.com/PineWhisperStudio)
