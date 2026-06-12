# README + CLAUDE.md 整理实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 整理 README 安装区编号和尾部 CTA 顺序，补充 CLAUDE.md 缺失的项目结构

**Architecture:** 修改 3 个文件：`README.md`（安装区去编号 + 尾部 License/CTA 换位）、`README.zh.md`（同步）、`CLAUDE.md`（新增非 src 项目结构表）

**Tech Stack:** Markdown

---

### Task 1: 整理 README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: 替换安装区——去编号、npm 优先、alias 移入 Source**

**旧代码** (第 41-89 行):

```markdown
## 📦 Installation

**Zero dependencies by default.** termfoto uses your terminal's built-in protocols (sixel/kitty) or halfblocks rendering with no system packages required.

> 💡 **Want even better quality?** Install with chafa support: `cargo install termfoto --features chafa` (requires `libchafa-dev`). Prebuilt binaries include chafa statically — download and run, no deps needed.

### Option 0: npm

```bash
npm install -g termfoto
```

### Option 1: Cargo (recommended)

```bash
cargo install termfoto
```

### Option 2: Prebuilt binary

Download from [Releases](https://github.com/PineWhisperStudio/termfoto/releases), drop into `PATH`:

```bash
chmod +x termfoto
sudo cp termfoto /usr/local/bin/
```

### Option 3: .deb package (Debian/Ubuntu)

```bash
curl -LO https://github.com/PineWhisperStudio/termfoto/releases/latest/download/termfoto_latest_amd64.deb
sudo apt install ./termfoto_latest_amd64.deb
```

### Option 4: Build from source

```bash
git clone https://github.com/PineWhisperStudio/termfoto.git
cd termfoto
cargo build --release
ln -s $(pwd)/target/release/termfoto ~/.local/bin/termfoto
```

### Optional alias

```bash
# Add to ~/.bashrc or ~/.config/fish/config.fish
alias dr='termfoto'
```
```

**新代码**:

```markdown
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
```

- [ ] **Step 2: 替换尾部——License 提到 CTA 之前**

**旧代码** (第 129-145 行):

```markdown
## 🌟 Like termfoto?

- ⭐ **Star this repo** — helps others discover it
- 🐛 **Report bugs** — [GitHub Issues](https://github.com/PineWhisperStudio/termfoto/issues)
- 💡 **Suggest features** — before requesting, ask: *"will it make browsing slower?"*

---

📦 **Also available on** [crates.io](https://crates.io/crates/termfoto) · [GitHub Releases](https://github.com/PineWhisperStudio/termfoto/releases)

---

Made with ❤️ by [PineWhisperStudio](https://github.com/PineWhisperStudio)

## 📜 License

MIT
```

**新代码**:

```markdown
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
```

- [ ] **Step 3: 提交**

```bash
git add README.md
git commit -m "docs: 整理 README——安装区去编号、License/CTA 换位"
```

---

### Task 2: 同步 README.zh.md

**Files:**
- Modify: `README.zh.md`

- [ ] **Step 1: 安装区同样去编号、npm 优先、alias 内联**

与英文版相同改动：
- `### 方式零：npm` → `### npm`
- `### 方式一：Cargo（推荐）` → `### Cargo`
- `### 方式二：预编译二进制` → `### 预编译二进制`
- `### 方式三：.deb 包` → `### .deb 包（Debian/Ubuntu）`
- `### 方式四：从源码编译` → `### 从源码编译`
- "创建别名" 独立段 → 移入 Source 段内 blockquote

- [ ] **Step 2: 尾部 License 和 CTA 换位**

与英文版相同改动。

- [ ] **Step 3: 提交**

```bash
git add README.zh.md
git commit -m "docs: 同步 README.zh.md——安装区整理与英文版一致"
```

---

### Task 3: 补充 CLAUDE.md 项目结构

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: 在 feature flags 段之后插入项目结构表**

在文件末尾（第 68 行之后）追加：

```markdown

**项目结构（非 src）**:

| 路径 | 用途 |
|------|------|
| `npm/` | npm 薄包装（package.json + install.js），CI 发布时自动更新版本 |
| `.github/workflows/ci.yml` | push/PR 触发 build + test + clippy |
| `.github/workflows/release.yml` | tag 触发：构建二进制 + .deb + crates.io 发布 + npm publish |
| `assets/` | README demo.gif 等静态资源 |
```

- [ ] **Step 2: 提交**

```bash
git add CLAUDE.md
git commit -m "docs: CLAUDE.md 补充项目结构说明（npm/、CI、assets）"
```
