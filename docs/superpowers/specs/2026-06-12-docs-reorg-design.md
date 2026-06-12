# README + CLAUDE.md 重新整理设计

## 目标

整理 README 安装区的编号、尾部 CTA 区结构，以及 CLAUDE.md 缺失的项目结构说明。

## README 改动

### 安装区重排

npm 优先，去掉 Option 编号：

```markdown
## 📦 Installation

...

### npm

```bash
npm install -g termfoto
```

### Cargo

```bash
cargo install termfoto
```

### Prebuilt binary

...

### .deb package (Debian/Ubuntu)

...

### Build from source

...
```

"Optional alias" 移入 "Build from source" 段的末尾，不再独立成段。

### 尾部重排

License 移到 CTA 之前：

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

## CLAUDE.md 改动

在 "依赖" 段之后，新增 "项目结构（非 src）"：

```markdown
**项目结构（非 src）**:

| 路径 | 用途 |
|------|------|
| `npm/` | npm 薄包装（package.json + install.js），CI 发布时自动更新版本 |
| `.github/workflows/ci.yml` | push/PR 触发 build + test + clippy |
| `.github/workflows/release.yml` | tag 触发：构建二进制 + .deb 打包 + crates.io 发布 + npm publish |
| `assets/` | README demo.gif 等静态资源 |
```

## 同步文件

- `README.zh.md` 同步上述 README 所有改动

## 不改变

- 特性、设计哲学、竞品对比、使用方式、快捷键、技术栈——这些不动
