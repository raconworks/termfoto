# darkroom — 设计文档

**日期：** 2026-05-14  
**状态：** 已批准，待实现

---

## 项目简介

一个终端原生的图片查看工具，用 Rust 实现。支持网格浏览目录中的图片和全屏预览单张图片。目标是轻量、快速、兼容主流终端。

---

## 设计原则

- **Terminal First**：所有功能围绕终端工作流设计
- **Fast First**：低内存，快启动，懒加载缩略图
- **兼容性**：自动检测终端能力，Kitty 优先，Unicode Block 回落
- **YAGNI**：MVP 只实现查看，不涉及编辑、OCR、AI 功能

---

## CLI 入口

二进制名称：`darkroom`，支持短别名 `dr`。

```
darkroom                # 扫描当前目录，进入网格视图
darkroom <directory>    # 扫描指定目录，进入网格视图
darkroom <image_file>   # 直接进入单图全屏预览

dr                      # 等同于 darkroom（短别名）
dr <directory>
dr <image_file>
```

短别名通过安装时创建符号链接（`ln -s darkroom dr`）或 shell alias 实现。

---

## 整体架构

```
main.rs
  ├── args 解析（clap）
  │     ├── 无参数   → 扫描当前目录 → GridView
  │     ├── <dir>    → 扫描指定目录 → GridView
  │     └── <file>   → 直接 → PreviewView
  │
  ├── App（状态机）
  │     ├── state: AppState { Grid, Preview }
  │     ├── images: Vec<ImageEntry>
  │     ├── selected: usize
  │     └── grid_cols: u16（根据终端宽度动态计算）
  │
  ├── GridView widget    ← 缩略图网格
  ├── PreviewView widget ← 全屏单图预览
  └── render loop（crossterm + ratatui 事件循环）
```

### 状态转换

```
GridView  ──[Enter]──►  PreviewView
          ◄──[Esc/q]──
GridView  ──[q]──► 退出
```

---

## 图片扫描

- 扫描指定目录（**不递归**子目录）
- 支持格式：`png`, `jpg/jpeg`, `webp`, `gif`（第一帧）, `bmp`, `tiff`, `ico`
- 通过 [`image`](https://crates.io/crates/image) crate 解码
- 文件按**文件名字典序**排列

### ImageEntry

```rust
struct ImageEntry {
    path: PathBuf,
    filename: String,
    thumbnail: Option<DynamicImage>,  // 懒加载，进入视口时生成
}
```

缩略图按网格单元尺寸**等比缩放**，只在滚动到视口范围时才生成，避免启动时全量加载。

---

## 网格视图（GridView）

### 布局

- 每格固定宽度约 20 字符列，`cols = terminal_width / 20`，最少 1 列
- 每格显示缩略图 + 文件名（超长截断）
- 选中格子：青色高亮边框；其余：灰色边框

```
┌──────────────────┐
│                  │
│    [缩略图]      │
│                  │
│  filename.png    │
└──────────────────┘
```

### 键盘操作

| 键 | 动作 |
|---|---|
| ← → | 左右切换格子 |
| ↑ ↓ | 上下移动一行 |
| Page Down / Space | 向下翻一页 |
| Page Up | 向上翻一页 |
| Home | 跳到第一张 |
| End | 跳到最后一张 |
| Enter | 进入全屏预览 |
| q | 退出程序 |

### 滚动

图片数量超出屏幕时纵向滚动，保持选中项始终在视口内。

---

## 单图全屏预览（PreviewView）

- 图片**等比缩放**铺满终端可用区域（不裁剪）
- 底部状态栏显示：文件名、原始分辨率、格式
- 使用 `ratatui-image` 自动选择最佳渲染后端（Kitty / Sixel / Unicode Block）

### 键盘操作

| 键 | 动作 |
|---|---|
| Esc / q | 返回网格视图 |
| ← | 切换上一张图片 |
| → | 切换下一张图片 |

---

## 渲染策略

使用 [`ratatui-image`](https://crates.io/crates/ratatui-image)，自动检测终端能力：

| 优先级 | 后端 | 终端支持 |
|---|---|---|
| 1 | Kitty Graphics Protocol | Kitty, WezTerm |
| 2 | Sixel | xterm, mlterm |
| 3 | Unicode Block | tmux, SSH, 通用终端 |

---

## 项目结构

```
darkroom/
├── src/
│   ├── main.rs          # args 解析 + app 启动
│   ├── app.rs           # App 状态机 + 事件循环
│   ├── scanner.rs       # 目录扫描 + ImageEntry
│   ├── ui/
│   │   ├── grid.rs      # GridView widget
│   │   └── preview.rs   # PreviewView widget
│   └── types.rs         # 共享类型（AppState 等）
├── Cargo.toml
└── docs/
    └── superpowers/specs/
        └── 2026-05-14-terminal-image-viewer-design.md
```

### Cargo.toml 依赖

```toml
[dependencies]
ratatui        = "0.29"
crossterm      = "0.28"
ratatui-image  = "2"
image          = "0.25"
clap           = { version = "4", features = ["derive"] }
```

---

## 性能目标

- 启动时间：< 100ms（不含图片解码）
- 内存占用：< 50MB（正常使用，懒加载缩略图）
- 缩略图懒加载：只渲染视口内的图片

---

## 不在 MVP 范围内

- 图片编辑（裁剪、调色等）
- OCR
- 递归扫描子目录
- 图片删除/移动
- 缩放操作（预览时缩放）
- 动态 GIF
- AI 功能
