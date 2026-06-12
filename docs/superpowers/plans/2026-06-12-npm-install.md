# npm install 薄包装实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加 `npm install -g termfoto` 安装方式——薄包装自动下载 GitHub Releases 二进制

**Architecture:** 3 个文件：`npm/package.json`（npm 元信息 + postinstall hook）、`npm/install.js`（平台检测 → 下载 → chmod）、`.github/workflows/release.yml`（加 npm publish job）。README 加安装条目。

**Tech Stack:** Node.js 内置模块（`https`、`fs`、`path`、`os`），无外部依赖

**说明：** 当前 release 仅构建 Linux x86_64。未来扩展多平台时增加 os/arch 映射即可。

---

### Task 1: 创建 npm/package.json

**Files:**
- Create: `npm/package.json`

- [ ] **Step 1: 创建 package.json**

```json
{
  "name": "termfoto",
  "version": "0.1.0",
  "description": "Fast terminal photo viewer — keyboard-driven, chafa-rendered",
  "repository": "PineWhisperStudio/termfoto",
  "license": "MIT",
  "os": ["linux"],
  "bin": {
    "termfoto": "termfoto"
  },
  "scripts": {
    "postinstall": "node install.js"
  },
  "files": [
    "install.js"
  ],
  "keywords": [
    "tui",
    "image",
    "viewer",
    "terminal",
    "chafa"
  ]
}
```

- [ ] **Step 2: 提交**

```bash
git add npm/package.json
git commit -m "feat: 添加 npm package.json（薄包装）"
```

---

### Task 2: 创建 npm/install.js

**Files:**
- Create: `npm/install.js`

- [ ] **Step 1: 创建 install.js**

```javascript
#!/usr/bin/env node
// termfoto npm install script — downloads prebuilt binary from GitHub Releases

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');

const PLATFORM_MAP = {
  linux: 'linux',
  darwin: 'macos',
  win32: 'windows',
};

const ARCH_MAP = {
  x64: 'amd64',
  arm64: 'arm64',
};

function getBinaryName() {
  const platform = os.platform();
  if (platform === 'win32') return 'termfoto.exe';
  return 'termfoto';
}

function getDownloadUrl(version, binaryName) {
  // Currently only linux x64 is built. Future: add platform/arch suffix to binary names.
  return `https://github.com/PineWhisperStudio/termfoto/releases/download/v${version}/${binaryName}`;
}

function install() {
  const pkg = require('./package.json');
  const version = pkg.version;
  const binaryName = getBinaryName();
  const url = getDownloadUrl(version, binaryName);
  const dest = path.join(__dirname, binaryName);

  console.log(`termfoto: downloading v${version}...`);

  https.get(url, (res) => {
    if (res.statusCode === 302 || res.statusCode === 301) {
      // Follow redirect
      https.get(res.headers.location, (redirectRes) => {
        downloadToFile(redirectRes, dest, binaryName);
      }).on('error', (err) => {
        handleError(err, url);
      });
      return;
    }

    if (res.statusCode !== 200) {
      console.error(`termfoto: HTTP ${res.statusCode} — binary not found for this platform`);
      console.error(`termfoto: URL: ${url}`);
      process.exit(1);
    }

    downloadToFile(res, dest, binaryName);
  }).on('error', (err) => {
    handleError(err, url);
  });
}

function downloadToFile(res, dest, binaryName) {
  const file = fs.createWriteStream(dest, { mode: 0o755 });
  res.pipe(file);

  file.on('finish', () => {
    file.close();
    // Ensure executable
    fs.chmodSync(dest, 0o755);
    console.log(`termfoto v${require('./package.json').version} installed successfully`);
  });

  file.on('error', (err) => {
    fs.unlink(dest, () => {});
    console.error(`termfoto: failed to write binary — ${err.message}`);
    process.exit(1);
  });
}

function handleError(err, url) {
  console.error(`termfoto: download failed — ${err.message}`);
  console.error(`termfoto: URL: ${url}`);
  console.error(`termfoto: if your platform is not yet supported, install via cargo instead: cargo install termfoto`);
  process.exit(1);
}

install();
```

- [ ] **Step 2: 提交**

```bash
git add npm/install.js
git commit -m "feat: 添加 npm install.js（自动下载二进制）"
```

---

### Task 3: 在 release.yml 中加 npm publish job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: 在 publish job 之后追加 npm-publish job**

在文件末尾追加：

```yaml
  npm-publish:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: 从 tag 同步版本号
        run: |
          TAG_VERSION="${GITHUB_REF_NAME#v}"
          cd npm
          npm version "${TAG_VERSION}" --no-git-tag-version --allow-same-version
          echo "Synced npm version to ${TAG_VERSION}"

      - name: 发布到 npm
        working-directory: npm
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
        run: npm publish --access public
```

- [ ] **Step 2: 提交**

```bash
git add .github/workflows/release.yml
git commit -m "ci: release workflow 加 npm publish job"
```

---

### Task 4: README 中加 npm 安装方式

**Files:**
- Modify: `README.md`
- Modify: `README.zh.md`

- [ ] **Step 1: 在英文 README 安装区插入 Option: npm**

在 `### Option 1: Cargo (recommended)` 之前插入：

```markdown
### Option 0: npm

```bash
npm install -g termfoto
```
```

然后把 `Option 1` 改为 `Option 1`、`Option 2` → `Option 2`……保持编号连贯即可。

实际改动：在英文 README 的 `## 📦 Installation` 段中，**在 Cargo 之前**插入 npm，其余编号顺延。

具体编辑——在 `### Option 1: Cargo (recommended)` 的行之前插入：

```markdown
### Option 0: npm

```bash
npm install -g termfoto
```

```

（注意空行分隔）

- [ ] **Step 2: 在中文 README 同样插入**

在 `### 方式一：Cargo（推荐）` 之前插入：

```markdown
### 方式零：npm

```bash
npm install -g termfoto
```
```

- [ ] **Step 3: 提交**

```bash
git add README.md README.zh.md
git commit -m "docs: README 安装区加 npm install"
```

---

### Task 5: 最终验证

- [ ] **Step 1: 确认文件完整**

```bash
ls npm/package.json npm/install.js
node -c npm/install.js  # 语法检查
cat npm/package.json | python3 -m json.tool > /dev/null && echo "valid JSON"
```

- [ ] **Step 2: 查看所有改动**

```bash
git status
git log --oneline -5
```

- [ ] **Step 3: 推送**

```bash
git push
```
