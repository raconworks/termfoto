# chafa 可选化设计

## 目标

让没有 `libchafa` 的用户也能编译运行 darkroom。chafa 从硬编译依赖变为可选 feature（默认开启），使用 terminal native protocols（sixel/kitty）或 halfblocks 作为备选渲染后端。

## 方案

**编译时可选 chafa + 运行时终端协议自动降级**。

- `Cargo.toml` 中 chafa 改为可选 feature，默认开启
- `Picker::from_query_stdio()` 内部自动检测终端能力，选择最优协议
- 降级链：sixel → kitty → chafa（若编译）→ halfblocks

## 降级效果

| 终端 | libchafa? | 实际协议 | 质量 |
|------|-----------|---------|------|
| Kitty/WezTerm | 不需要 | kitty | 高质量（零外部依赖）|
| 支持 sixel 的终端 | 不需要 | sixel | 高质量（零外部依赖）|
| 普通终端 | 已安装 | chafa | 高质量 |
| 普通终端 | 未安装 | halfblocks | 中低质量（保底）|

## 文件变更

### Cargo.toml

```toml
[features]
default = ["chafa"]
chafa = ["ratatui-image/chafa-dyn"]
chafa-static = ["ratatui-image/chafa-static"]

[dependencies]
ratatui-image = { version = "11", features = ["crossterm"] }
# chafa-dyn / chafa-static 由 feature 控制，不再是硬编码
```

当前 `ratatui-image` 的 features 硬编码了 `chafa-dyn`：
```toml
# 当前（改前）
ratatui-image = { version = "11", features = ["crossterm", "chafa-dyn"] }
```

### .github/workflows/release.yml

Release CI 使用 `chafa-static` 将 `libchafa.a` 编入二进制，用户下载后零运行时依赖：

```yaml
- name: 编译 release
  run: cargo build --release --features chafa-static
```

### src/main.rs

Picker 构造保持不变。`from_query_stdio()` 内部已包含协议检测降级逻辑——根据编译 feature 自动决定是否包含 chafa 检测；halfblocks 保底始终存在。

```rust
// 不变
let picker = Picker::from_query_stdio()
    .unwrap_or_else(|_| Picker::halfblocks());
```

### README 更新

安装说明中补充 chafa 可选信息：

**中文版 (README.zh.md)**：
```markdown
### 系统依赖

darkroom 默认使用 chafa 渲染图片，推荐安装 `libchafa`：

```bash
# Debian/Ubuntu
sudo apt install libchafa-dev
```

如果不使用 chafa，darkroom 可自动使用终端内置协议（sixel/kitty）或 halfblocks 渲染。通过 `--no-default-features` 编译无需任何系统依赖的版本：

```bash
cargo install darkroom --no-default-features
```

**英文版 (README.md)**：同上，英文表述。

## 安装行为

| 安装方式 | 编译时需要 | 运行时需要 | 协议 |
|---------|----------|----------|------|
| `cargo install darkroom` | `libchafa-dev` | `libchafa.so` | chafa + sixel + kitty + halfblocks |
| `cargo install darkroom --no-default-features` | 无 | 无 | sixel + kitty + halfblocks |
| GitHub Release 二进制 | 无（static 链接）| 无 | chafa（static）+ sixel + kitty + halfblocks |

## 验证

```bash
# 默认构建（含 chafa-dyn）
cargo build
cargo run -- <测试目录>

# 无 chafa 构建
cargo build --no-default-features
cargo run -- <测试目录>

# static 链接构建（模拟 release）
cargo build --release --features chafa-static
cargo run --release -- <测试目录>
```

- 默认构建行为不变
- 无 chafa 构建：在支持 sixel/kitty 的终端中高质量显示，在普通终端中用 halfblocks 显示
- static 构建：chafa 高质量渲染 + 二进制无 libchafa.so 依赖
