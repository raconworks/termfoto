# chafa 可选化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** chafa 从硬编译依赖变为可选 feature，Release 用 static 链接，支持 `--no-default-features` 零系统依赖构建。

**Architecture:** Cargo features 控制 chafa 编译方式——`chafa`（默认，dyn 链接）用于 `cargo install`，`chafa-static` 用于 GitHub Release。无 feature 时为 lite 版（纯 halfblocks/sixel/kitty）。Picker 构造不变，降级链自动生效。

**Tech Stack:** Cargo features, ratatui-image

---

## 文件映射

| 文件 | 操作 |
|------|------|
| `Cargo.toml` | 修改：新增 `[features]`，移除硬编码 `chafa-dyn` |
| `.github/workflows/release.yml` | 修改：`chafa-static` 构建 |
| `README.md` | 修改：英文版补充 chafa 可选说明 |
| `README.zh.md` | 修改：中文版补充 chafa 可选说明 |

`src/main.rs` 和 `src/app.rs` 无需改动——Picker 构造逻辑已包含降级，`from_query_stdio()` 内部根据编译 feature 自动调整。

---

### Task 1: Cargo.toml feature 化

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 新增 `[features]` section，移除硬编码 `chafa-dyn`**

当前行：
```toml
ratatui-image = { version = "11", features = ["crossterm", "chafa-dyn"] }
```

改为：
```toml
ratatui-image = { version = "11", features = ["crossterm"] }
```

在 `[dependencies]` 之前插入 `[features]` section：

```toml
[features]
default = ["chafa"]
chafa = ["ratatui-image/chafa-dyn"]
chafa-static = ["ratatui-image/chafa-static"]
```

完整的 `Cargo.toml`：

```toml
[package]
name = "darkroom"
version = "0.1.0"
edition = "2021"
description = "终端图片浏览器——高效、轻量、chafa 渲染"
repository = "https://github.com/boyangso/darkroom"
license = "MIT"
keywords = ["tui", "image", "viewer", "terminal", "chafa"]
categories = ["command-line-utilities", "multimedia::images"]

[features]
default = ["chafa"]
chafa = ["ratatui-image/chafa-dyn"]
chafa-static = ["ratatui-image/chafa-static"]

[[bin]]
name = "darkroom"
path = "src/main.rs"

[dependencies]
ratatui = { version = "0.30", default-features = false, features = ["crossterm"] }
crossterm = "0.29"
ratatui-image = { version = "11", features = ["crossterm"] }
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp", "gif", "bmp", "tiff", "ico"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: 验证默认构建（含 chafa）**

```bash
cargo build
```

Expected: 编译成功，和之前行为一致。

- [ ] **Step 3: 验证无 chafa 构建**

```bash
cargo build --no-default-features
```

Expected: 编译成功，不需要 libchafa-dev。

- [ ] **Step 4: 验证 chafa-static 构建**

```bash
cargo build --release --features chafa-static
```

Expected: 编译成功，静态链接 libchafa。

- [ ] **Step 5: 运行测试**

```bash
cargo test
cargo test --no-default-features
```

Expected: 全部通过。

- [ ] **Step 6: 提交**

```bash
git add Cargo.toml
git commit -m "feat: chafa 改为可选 feature，支持 --no-default-features 零依赖构建"
```

---

### Task 2: CI Release 使用 chafa-static

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: 修改编译命令为 chafa-static**

```yaml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: 安装 Rust
        uses: dtolnay/rust-toolchain@stable

      - name: 安装系统依赖
        run: sudo apt-get install -y libchafa-dev

      - name: 编译 release
        run: cargo build --release --features chafa-static

      - name: 打包 .deb
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          PKG="darkroom_${VERSION}_amd64"
          mkdir -p "${PKG}/DEBIAN"
          mkdir -p "${PKG}/usr/local/bin"
          cp target/release/darkroom "${PKG}/usr/local/bin/"
          cat > "${PKG}/DEBIAN/control" <<EOF
          Package: darkroom
          Version: ${VERSION}
          Architecture: amd64
          Maintainer: darkroom
          Description: 终端图片浏览器——高效、轻量、chafa 渲染
          EOF
          dpkg-deb --build "${PKG}"
        env:
          GITHUB_REF_NAME: ${{ github.ref_name }}

      - name: 发布到 GitHub Releases
        uses: softprops/action-gh-release@v1
        with:
          files: |
            target/release/darkroom
            darkroom_*.deb
```

变更：编译步骤从 `cargo build --release` 改为 `cargo build --release --features chafa-static`，.deb 的 `Depends: libchafa0` 移除（static 链接后不需要运行时库）。

- [ ] **Step 2: 提交**

```bash
git add .github/workflows/release.yml
git commit -m "ci: Release 使用 chafa-static 静态链接，移除运行时 libchafa 依赖"
```

---

### Task 3: README 补充 chafa 可选说明

**Files:**
- Modify: `README.md`
- Modify: `README.zh.md`

- [ ] **Step 1: 更新英文版 README.md 安装部分**

在 `### System dependency` 节的 `libchafa` 安装命令后，追加：

```markdown
> 💡 **Don't want chafa?** darkroom can use your terminal's built-in protocols (sixel/kitty) or fall back to halfblocks. Install without chafa: `cargo install darkroom --no-default-features`. Prebuilt binaries include chafa statically — no system deps needed.
```

修改后的完整 System dependency 节：

```markdown
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

> 💡 **Don't want chafa?** darkroom can use your terminal's built-in protocols (sixel/kitty) or fall back to halfblocks. Install without chafa: `cargo install darkroom --no-default-features`. Prebuilt binaries include chafa statically — no system deps needed.
```

- [ ] **Step 2: 更新中文版 README.zh.md 安装部分**

在 `### 系统依赖` 节的 `libchafa` 安装命令后，追加：

```markdown
> 💡 **不想装 chafa？** darkroom 可使用终端内置协议（sixel/kitty）或 halfblocks 渲染。无依赖安装：`cargo install darkroom --no-default-features`。预编译二进制已静态链接 chafa，无需任何系统依赖。
```

- [ ] **Step 3: 提交**

```bash
git add README.md README.zh.md
git commit -m "docs: README 补充 chafa 可选安装说明"
```

---

## 验证

```bash
# 全部三种构建方式
cargo build                         # 默认 chafa-dyn
cargo build --no-default-features   # 零依赖 lite
cargo build --release --features chafa-static  # static 链接

# 测试
cargo test
cargo test --no-default-features
```
