# npm install 安装方式设计

## 目标

添加 `npm install -g termfoto` 安装方式，通过薄 npm 包自动从 GitHub Releases 下载对应平台二进制。

## 方案

薄包装（~10KB）。`package.json` + `install.js`，postinstall 时检测平台 → 从 GitHub Releases 下载预编译二进制 → chmod +x。

## 文件结构

```
npm/
  package.json   # npm 包元信息，bin 指向下载后的二进制
  install.js     # 检测平台 → 下载 → chmod
```

## install.js 流程

1. 读取 `package.json` 中的 version
2. 检测 `process.platform`（linux/darwin/win32）+ `process.arch`（x64/arm64）
3. 构造 URL：`https://github.com/PineWhisperStudio/termfoto/releases/download/v{version}/termfoto`
4. 使用 Node.js 内置 `https` 模块下载到包目录
5. `chmod 755`
6. 二进制名 = `termfoto`（Windows 加 `.exe`）

## package.json 要点

- `name`: `"termfoto"`
- `bin`: `{ "termfoto": "termfoto"}` （相对路径，npm link 自动处理）
- `scripts.postinstall`: `"node install.js"`
- `os`: `["linux", "darwin", "win32"]`（仅支持的平台）
- `files`: `["install.js"]`（不打包二进制本身）

## 改动范围

| 文件 | 改动 |
|------|------|
| `npm/package.json` | 新建 |
| `npm/install.js` | 新建 |
| `.github/workflows/release.yml` | 加 `npm-publish` job（release body 注释或 tag push 触发） |
| `README.md` | 安装区加 Option: npm |
| `README.zh.md` | 同步 |

## CI 集成

在 `release.yml` 的 `publish` job 之后加 `npm-publish` job：

1. Checkout（检出 `npm/` 目录或完整仓库）
2. 用 `npm version $VERSION --no-git-tag-version` 同步版本
3. `npm publish`（需要 `NPM_TOKEN` secret）

## 新增 README 内容

```markdown
### Option: npm

```bash
npm install -g termfoto
```
```

中文版对应翻译。

## 依赖

无外部 npm 依赖——仅 Node.js 内置模块（`https`、`fs`、`path`、`os`）。
