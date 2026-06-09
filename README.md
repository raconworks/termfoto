# darkroom

终端图片浏览器。效率优先，功能其次。

## 设计哲学

- **效率优先** — 图片加载和渲染全部异步，不阻塞 UI；chafa 编码在后台线程执行
- **功能其次** — 只做浏览和查看，不搞花哨的缩放、滤镜、幻灯片
- **终端原生** — 用 chafa + Unicode 半块字符渲染，充分利用终端 true color 能力

## 安装

```bash
# 系统依赖
sudo apt install libchafa-dev

# 编译
cargo build --release

# 可选：创建软链接
ln -s $(pwd)/target/release/darkroom ~/.local/bin/dr
```

## 使用

```bash
darkroom              # 浏览当前目录
darkroom ~/图片       # 浏览指定目录
darkroom photo.jpg    # 直接打开单张图片
```

### 快捷键

| 模式 | 按键 | 功能 |
|------|------|------|
| 浏览器 | `←` `→` `↑` `↓` | 导航 |
| 浏览器 | `Space` `PageUp/Down` | 翻页 |
| 浏览器 | `Home` `End` | 跳到首/尾 |
| 浏览器 | `Enter` | 全屏查看 |
| 浏览器 | `q` `Ctrl+C` | 退出 |
| 全屏 | `←` `→` | 切换图片 |
| 全屏 | `Enter` `Esc` `q` | 返回浏览器 |
| 全屏 | `Ctrl+C` | 退出 |

## 依赖

- [ratatui](https://github.com/ratatui/ratatui) — TUI 框架
- [ratatui-image](https://github.com/ratatui/ratatui-image) — 终端图片渲染（Kitty/Sixel/iTerm2/Halfblocks）
- [chafa](https://hpjansson.org/chafa/) — 高质量图片转字符
- [image](https://github.com/image-rs/image) — 图片解码
