# README 双语支持设计

## 目标

README 支持中文和英文，默认英文显示。中文用户可通过链接切换到中文版。

## 方案

**双文件结构**：`README.md`（英文，GitHub 默认渲染）+ `README.zh.md`（中文）。两个文件顶部有语言切换链接。

## 文件规划

| 文件 | 操作 | 说明 |
|------|------|------|
| `README.md` | **重写** | 英文版，面向国际读者，保留品牌一致性 |
| `README.zh.md` | **新建** | 当前中文 README.md 内容移入（原样保留） |

## README.md（英文版）内容结构

### 1. Header + Badges

语言切换栏 + badges + 终端 mockup：

```markdown
[中文版](README.zh.md)

# darkroom

> A darkroom for your terminal — develop every photo with chafa.

[badges]
[terminal mockup — English text]
```

### 2. 特性网格

6 项特性，与中文版一一对应，英文重新表达：

- High-quality chafa rendering
- Non-blocking async loading
- Original-size fullscreen
- Keyboard-only navigation
- Extremely lightweight
- Instant startup

### 3. 设计哲学

Efficiency first, features second. 三条原则，逐条翻译，并补充具体排除清单（slideshow / batch exporter / photo editor）。

### 4. 安装

四种方式 + 别名，与中文版结构一致：
1. Cargo（推荐）
2. 预编译二进制
3. .deb 包
4. 从源码编译

示例路径本地化为英文习惯（`~/Pictures`）。

### 5. 使用 + 快捷键

`Usage` 和 `Keybindings` 两节，与中文版对应翻译。

### 6. 技术栈 + 许可证

Tech Stack 和 License 两节，逐项翻译。

## README.zh.md（中文版）内容

当前 `README.md` 的全部内容原样移入，顶部增加语言切换链接：

```markdown
[English](README.md)
```

其余内容不变。

## 语言切换

两个文件顶部各有一个单行链接指向对方版本。不需要 badge 或图标，保持简洁。

## 同步策略

- README.md 和 README.zh.md **独立维护**，无需保持完全一致
- 安装方式、快捷键表等**事实性内容**需同步更新
- 设计哲学、特性描述等**表达性内容**可各自独立
- 不在 CI/工具层面做强制同步检查（与 "功能克制" 哲学一致）

## 验证

- 检查 GitHub 上 README.md 为英文，页面顶部有 `中文版` 链接
- 点击链接跳转到 README.zh.md，全中文，顶部有 `English` 链接
- `cargo publish` 发布时 README.md（英文）作为 crates.io 展示
