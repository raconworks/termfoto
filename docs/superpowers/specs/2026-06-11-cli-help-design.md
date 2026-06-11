# CLI --help / --version 设计

## 目标

在不引入新依赖的前提下，为 termfoto 添加 `--help` / `-h` / `--version` / `-V` 支持。

## 方案

手动解析 `std::env::args()[1]`，匹配已知标志：
- `-h` / `--help` → 打印帮助文本，`exit(0)`
- `-V` / `--version` → 打印版本号（从 `env!("CARGO_PKG_VERSION")` 获取），`exit(0)`
- 其他 → 现有逻辑不变

## 改动范围

仅 `src/main.rs` — 在 `main()` 函数开头、`let path = ...` 之前插入参数检查。

## 帮助文本

```
termfoto — 终端图片浏览器

用法: termfoto [路径]

  <路径>    图片文件或目录（默认当前目录）

选项:
  -h, --help        显示此帮助
  -V, --version     显示版本号
```
