# 项目瘦身优化 — 设计文档

## 概述

在不改变功能的前提下，通过编译参数调优、依赖裁剪、`clap` 替换三项措施，缩减二进制体积和编译时间。

## 改动清单

### ① 编译参数调优 — `Cargo.toml`

```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = "z"
strip = true
panic = "abort"
```

所有参数仅影响 release 构建，debug 构建保持不变。

### ② 裁剪 image codec — `Cargo.toml`

```diff
- features = ["png", "jpeg", "webp", "gif", "bmp", "tiff", "ico"]
+ features = ["png", "jpeg", "webp"]
```

不支持 GIF/BMP/TIFF/ICO 的图片文件将无法打开。扫描时这些文件仍会列出，但加载会失败。

### ③ 替换 clap 为手动解析 — `Cargo.toml` + `src/main.rs`

移除 `clap` 依赖，`main.rs` 中 `Args` struct 替换为简单函数：

```rust
fn parse_args() -> Option<PathBuf> {
    std::env::args().nth(1).map(PathBuf::from)
}
```

`main()` 中 `Args::parse()` → `parse_args()`。

## 不改动的

- ratatui / crossterm / ratatui-image — 核心 TUI + 图片渲染栈
- anyhow — 零依赖，保留
- 所有业务代码：线程池、搜索、Lang、导航、渲染逻辑
- `--version` 输出：可后续加回（手动判断 `--version` / `-V`），当前仅 `cargo run -- --version` 场景损失，不影响使用

## 图片格式支持变化

| 格式 | 优化前 | 优化后 |
|------|--------|--------|
| PNG | ✅ | ✅ |
| JPEG | ✅ | ✅ |
| WebP | ✅ | ✅ |
| GIF | ✅ | ❌ |
| BMP | ✅ | ❌ |
| TIFF | ✅ | ❌ |
| ICO | ✅ | ❌ |

## 验证

```bash
cargo build --release
ls -lh target/release/darkroom    # binary size before/after
cargo tree --prefix indent | grep -cE "^[│├└]"  # crate count
cargo test                        # all tests pass
cargo run -- <dir-with-png+jpg>   # normal browsing works
```
