# README 推广优化设计

## 目标

提升 termfoto README 的发现、试用、传播三个环节的转化率。目标用户：开发者为主、兼顾普通 Linux 桌面用户。

## 语言策略

- README.md 英文为主，README.zh.md 中文翻译同步更新
- Cargo.toml `description` 改为英文
- Cargo.toml `repository` 统一为 `raconworks/termfoto`

## 结构变更

```
当前                              新
───────────────────────────────    ───────────────────────────────
中英文链接                         中英文链接
标题 + slogan                      标题 + slogan（新 slogan）
4 个假 badge                       GitHub Actions + crates.io v + crates.io d + license（真实 badge）
ASCII 示意图                       移除（asciinema 替代）
                                  🆕 asciinema 录屏
───────────────────────────────    ───────────────────────────────
特性表格                           特性表格（措辞微调）
设计哲学                           设计哲学
                                  🆕 为什么不用其他工具（viu/timg/ranger/lf）
───────────────────────────────    ───────────────────────────────
安装（4 种方式）                   安装（4 种方式，修 bug）
使用                               使用（加 --help / --version）
快捷键                             快捷键（翻页拆分 + 搜索模式补充）
技术栈                             技术栈（删除 clap）
许可证                             许可证
                                  🆕 CTA 区（star / 分享 / issues / crates.io / Releases）
                                  🆕 raconworks 署名
```

## 逐区设计

### 1. Hero 区

- **slogan**: "Browse images at the speed of your terminal"（强调速度，区别于 GUI）
- **badge**:
  - GitHub Actions 真实 CI（`raconworks/termfoto/actions/workflows/ci.yml/badge.svg`，需确认 CI workflow 文件名）
  - crates.io version 动态（`crates.io/crates/termfoto?label=version`）
  - crates.io downloads 动态（`crates.io/crates/termfoto?label=downloads`）
  - license 静态保持
  - 移除 rust stable（开发者不需要 badge 证明）
- **asciinema 录屏**: 替换 ASCII 示意图。如果用户还没录制，先准备好占位链接和录制指南。录屏内容建议：启动→浏览→搜索→全屏→切换图片→退出

### 2. 特性表格

不变，仅修正 "4 core dependencies" 等措辞。

### 3. 为什么不用其他工具

| 工具 | 定位 | termfoto 差异 |
|------|------|-------------|
| `viu` | 单图预览 | 目录浏览 + 键盘导航 + 全屏 |
| `timg` | 图片/视频播放 | 专注图片，启动更快更轻 |
| `ranger/lf` | 文件管理器 | 图片优先，交互浏览体验 |

### 4. 安装区修正

| 问题 | 修正 |
|------|------|
| `ln -s ... ~/.local/bin/dr` | `ln -s ... ~/.local/bin/termfoto` |
| Repo URL `raconworks` → 已确认正确 | 保持不变 |
| Release 下载 URL | 保持不变 |

### 5. 使用区

加 `termfoto --help` 和 `termfoto --version` 示例。

### 6. 快捷键表

- 翻页拆分为两行：`Space · PgDn`（下翻）、`PgUp`（上翻）
- 新增搜索模式 3 行：`Esc` 取消、`Tab · Shift+Tab` 切换结果、`Enter` 全屏
- 其余不变

### 7. 技术栈

删除 `clap` 行（已改为手动解析）。

### 8. CTA 区（全新）

```
🌟 Like termfoto?
⭐ Star this repo — helps others discover it
📣 Share it — with your terminal-loving friends
🐛 Report bugs — GitHub Issues
💡 Suggest features — before requesting, ask: "will it make browsing slower?"

📦 Also available on
crates.io · GitHub Releases

Made with ❤️ by raconworks
```

### 9. Cargo.toml 同步修改

- `description` → `"Fast terminal photo viewer — keyboard-driven, chafa-rendered"`
- `repository` → `"https://github.com/raconworks/termfoto"`

## 行动清单

1. 录制 asciinema 录屏（约 30 秒演示）
2. 创建 GitHub Actions CI workflow（如果还没有）
3. 修改 Cargo.toml（description、repository）
4. 重写 README.md（按上述结构）
5. 同步 README.zh.md（中文翻译）
6. 提交并推送
