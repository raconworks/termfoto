# 性能与架构优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除主线程阻塞、统一加载管线、优化代码架构

**Architecture:** 所有图片加载统一走后台线程（load_tx → channel → spawn线程 → done_rx），浏览器缩略图异步化，Protocol 缓存有界化，终端管理 RAII 化，按键分发移入 App

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.29, ratatui-image 11 (chafa-dyn), image 0.25

---

## 执行顺序

Task 1-2 是独立重构（无依赖），可先做。Task 3 是核心改动，Task 4-5 基于 Task 3。Task 6 最后收尾。

---

### Task 1: TermGuard — RAII 终端管理

**Files:**
- Modify: `src/main.rs:1-85`

- [ ] **Step 1: 在 main.rs 顶部添加 TermGuard 结构体**

在 `mod ui;` 之后、`use` 块之前插入：

```rust
use std::io::{self, Stdout};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

/// RAII guard that restores terminal state on drop.
struct TermGuard {
    _stdout: Stdout, // kept alive so LeaveAlternateScreen sees the same handle
}

impl TermGuard {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Self { _stdout: stdout })
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
```

注意：原有的 `use crossterm::{...}` 导入中删除 `execute`、`disable_raw_mode`、`enable_raw_mode`、`EnterAlternateScreen`、`LeaveAlternateScreen`，这些在 TermGuard 内部重新导入。

- [ ] **Step 2: 简化 main() 函数**

将 `main()` 中的终端初始化替换为 `TermGuard::enter()`，删除手动 cleanup 代码：

```rust
fn main() -> Result<()> {
    let args = Args::parse();

    let (images, initial_state) = match args.path {
        None => {
            let images = scan_directory(&std::env::current_dir()?)?;
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_dir() => {
            let images = scan_directory(p)?;
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_file() && scanner::is_supported_image(p) => {
            let entry = scanner::ImageEntry {
                path: p.clone(),
                filename: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            };
            (vec![entry], AppState::Fullscreen)
        }
        Some(ref p) => {
            eprintln!(
                "darkroom: '{}' is not a supported image or directory",
                p.display()
            );
            std::process::exit(1);
        }
    };

    if images.is_empty() && matches!(initial_state, AppState::Browser) {
        eprintln!("darkroom: no images found in the specified directory");
        std::process::exit(0);
    }

    let _term = TermGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    run(&mut terminal, images, initial_state)
    // TermGuard::drop 自动清理，无论 run() 成功还是错误
}
```

注意：删除原有的 `enable_raw_mode()?` + `execute!(stdout, EnterAlternateScreen)` + 错误路径手动 cleanup（共约 20 行），替换为一行 `let _term = TermGuard::enter()?;`。

- [ ] **Step 3: 编译验证**

```bash
cargo build
```

预期：编译通过，无错误。

- [ ] **Step 4: 提交**

```bash
git add src/main.rs
git commit -m "refactor: 封装 TermGuard RAII 终端管理

- 新增 TermGuard 结构体，构造时 enter raw mode + alternate screen
- Drop 时自动恢复终端状态
- main.rs 瘦身 ~20 行，消除手动 cleanup 重复代码
- 即使 run() 提前错误返回也能正确恢复终端"
```

---

### Task 2: handle_key 移入 App + visible_rows 字段

**Files:**
- Modify: `src/app.rs:36-57` (App::new), `src/app.rs:142-171` (新增 handle_key)
- Modify: `src/main.rs:118-171` (handle_key 函数 + 调用点)

- [ ] **Step 1: App 新增 visible_rows 字段和 setter**

在 `src/app.rs` 的 `App` struct 中添加字段：

```rust
pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    pub selected: usize,
    pub scroll_row: usize,
    pub picker: Picker,
    pub protocol_cache: HashMap<usize, Protocol>,
    pub fullscreen_protocol: Option<Protocol>,
    pub fullscreen_pending: bool,
    pub cache_width: u16,
    pub visible_rows: usize,      // 新增
    load_tx: Sender<usize>,
    load_rx: Receiver<(usize, Protocol)>,
}
```

在 `App::new` 中初始化：

```rust
visible_rows: 1,  // 新增，默认值
```

- [ ] **Step 2: 将 handle_key 移为 App 方法**

在 `src/app.rs` 的 `impl App` 块中，`clear_protocol_cache` 之后添加 `handle_key` 方法（注意：需要调整 imports）：

```rust
/// Handle a key event. Returns true if the app should quit.
pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match self.state {
        AppState::Browser => match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => self.navigate_left(),
            KeyCode::Right => self.navigate_right(),
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::PageDown | KeyCode::Char(' ') => self.navigate_page_down(self.visible_rows),
            KeyCode::PageUp => self.navigate_page_up(self.visible_rows),
            KeyCode::Home => self.navigate_home(),
            KeyCode::End => self.navigate_end(),
            KeyCode::Enter => self.enter_fullscreen(),
            _ => {}
        },
        AppState::Fullscreen => match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => self.fullscreen_prev(),
            KeyCode::Right => self.fullscreen_next(),
            _ => {}
        },
    }
    false
}
```

需要添加到 `app.rs` 顶部的 import：

```rust
use crossterm::event::{KeyCode, KeyModifiers};
```

- [ ] **Step 3: main.rs 删除 handle_key 函数，改用 App 方法**

在 `src/main.rs` 的 `run()` 函数中：

```rust
// 修改前
let should_quit = handle_key(&mut app, key.code, key.modifiers, visible_rows.max(1));

// 修改后
let should_quit = app.handle_key(key.code, key.modifiers);
```

在 `run()` 中添加 `app.visible_rows` 的更新（放在循环内、渲染前）：

```rust
loop {
    let size = terminal.size()?;
    let visible_rows = (size.height / CELL_HEIGHT as u16) as usize;
    // ...
    app.visible_rows = visible_rows.max(1);  // 新增
    // ...
}
```

删除 `main.rs` 底部的整个 `handle_key` 自由函数（约 30 行）。

同时删除 `main.rs` 中不再需要的 `KeyCode` 和 `KeyModifiers` 导入（如果它们只在 handle_key 中使用）。

- [ ] **Step 4: 编译验证**

```bash
cargo build
```

预期：编译通过。

- [ ] **Step 5: 运行已有测试**

```bash
cargo test
```

预期：全部通过（handle_key 的测试暂无，但现有导航测试应通过）。

- [ ] **Step 6: 提交**

```bash
git add src/app.rs src/main.rs
git commit -m "refactor: handle_key 移入 App，visible_rows 作为字段

- App 新增 visible_rows 字段，main.rs 循环中更新
- handle_key 从 main.rs 自由函数变为 App 方法
- main.rs 事件循环简化，删除 ~30 行"
```

---

### Task 3: Loader 统一加载管线（核心）

**Files:**
- Modify: `src/app.rs:110-161` (enter_fullscreen, collect_loads)
- Modify: `src/ui/browser.rs:173-212` (populate_protocol_cache)
- Modify: `src/main.rs:102-117` (事件循环调用)

这是最关键的改动——浏览器缩略图加载从主线程同步变为后台异步。

- [ ] **Step 1: App 新增 load_tx 克隆方法**

在 `src/app.rs` 中，`App` 需要能够克隆 sender 给外部使用（populate_protocol_cache 需要通过 App 发请求）。在 `impl App` 中添加：

```rust
/// Request background loading for an image index.
pub fn request_load(&self, idx: usize) {
    let _ = self.load_tx.send(idx);
}
```

（已有此方法，确认它仍然存在且正确）

- [ ] **Step 2: 扩展 collect_loads 处理浏览器缓存**

修改 `src/app.rs` 中的 `collect_loads` 方法：

```rust
/// Check for completed background image loads.
/// In Browser mode, results go into protocol_cache.
/// In Fullscreen mode, result for the selected image becomes fullscreen_protocol.
pub fn collect_loads(&mut self) {
    while let Ok((idx, proto)) = self.load_rx.try_recv() {
        if self.state == AppState::Fullscreen && idx == self.selected {
            self.fullscreen_protocol = Some(proto);
            self.fullscreen_pending = false;
        } else {
            // Browser mode: insert into cache. Also accept preloaded
            // fullscreen images that arrive when state already switched.
            self.protocol_cache.insert(idx, proto);
        }
    }
}
```

- [ ] **Step 3: 定义 LoadSize / LoadRequest，改造后台线程**

在 `src/app.rs` 顶部（`spawn_image_loader` 之前）定义两种加载模式：

```rust
/// Size mode for background image loading.
#[derive(Debug, Clone)]
pub enum LoadSize {
    /// Browser thumbnail at fixed cell dimensions.
    Thumbnail { w: u16, h: u16 },
    /// Fullscreen: original image size computed from font metrics.
    Original,
}

/// A request sent to the background loader.
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub idx: usize,
    pub size: LoadSize,
}
```

修改 `spawn_image_loader` 签名——channel 从 `usize` 改为 `LoadRequest`：

```rust
pub fn spawn_image_loader(
    picker: Picker,
    paths: Vec<std::path::PathBuf>,
) -> (Sender<LoadRequest>, Receiver<(usize, Protocol)>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<(usize, Protocol)>();

    std::thread::spawn(move || {
        while let Ok(req) = load_rx.recv() {
            if let Some(path) = paths.get(req.idx) {
                if let Ok(img) = image::open(path) {
                    let size = match req.size {
                        LoadSize::Thumbnail { w, h } => Size::new(w, h),
                        LoadSize::Original => {
                            let font_size = picker.font_size();
                            let nat_w = img.width().div_ceil(font_size.width as u32) as u16;
                            let nat_h = img.height().div_ceil(font_size.height as u32) as u16;
                            Size::new(nat_w.max(1), nat_h.max(1))
                        }
                    };
                    if let Ok(proto) = picker.new_protocol(
                        img,
                        size,
                        Resize::Fit(Some(FilterType::Lanczos3)),
                    ) {
                        let _ = done_tx.send((req.idx, proto));
                    }
                }
            }
        }
    });

    (load_tx, done_rx)
}
```

- [ ] **Step 4: 更新 App 中的 channel 类型和 request_load**

`App` struct 中 `load_tx` 类型变更：

```rust
pub struct App {
    // ...
    load_tx: Sender<LoadRequest>,        // 改：原为 Sender<usize>
    load_rx: Receiver<(usize, Protocol)>, // 不变
}
```

`request_load` 方法更新签名（`&self` 即可，`Sender::send` 不需要 `&mut`）：

```rust
pub fn request_load(&self, idx: usize, size: LoadSize) {
    let _ = self.load_tx.send(LoadRequest { idx, size });
}
```

`App::new` 签名适配：

```rust
pub fn new(
    images: Vec<ImageEntry>,
    state: AppState,
    picker: Picker,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<(usize, Protocol)>,
) -> Self { ... }
```

- [ ] **Step 5: enter_fullscreen / fullscreen_prev/next 使用 LoadSize::Original**

`enter_fullscreen` 保持不变（不需要额外参数），内部改用 `LoadSize::Original`：

```rust
pub fn enter_fullscreen(&mut self) {
    if !self.images.is_empty() {
        self.state = AppState::Fullscreen;
        self.fullscreen_protocol = None;
        self.fullscreen_pending = true;
        self.request_load(self.selected, LoadSize::Original);
    }
}
```

`fullscreen_prev` / `fullscreen_next` 同理将 `self.request_load(self.selected)` 改为：
```rust
self.request_load(self.selected, LoadSize::Original);
```

- [ ] **Step 6: populate_protocol_cache 改为纯异步**

在 `src/ui/browser.rs` 中重写——只发请求，不阻塞：

```rust
/// Request chafa protocol generation for visible images (async, non-blocking).
pub fn populate_protocol_cache(
    app: &App,
    cell_w: u16,
    cell_h: u16,
    terminal_width: u16,
    visible_rows: usize,
) {
    if cell_w < 2 || cell_h < 2 {
        return;
    }

    // 宽度变化清空缓存由 main.rs 的事件循环处理（已有 last_terminal_width 检查）。
    // 此处只负责发请求，不重复做清空。

    let thumb_w = cell_w.saturating_sub(2);
    let thumb_h = cell_h.saturating_sub(3);
    let start = app.scroll_row * IMAGES_PER_ROW;
    let end = (start + visible_rows * IMAGES_PER_ROW).min(app.images.len());

    for slot in start..end {
        if app.protocol_cache.contains_key(&slot) {
            continue;
        }
        app.request_load(slot, LoadSize::Thumbnail { w: thumb_w, h: thumb_h });
    }
}
```

注意：
- 现在 `app` 是 `&App`（不可变引用），`request_load` 只需要 `&self`
- 删除了 `image::open()` 和 `picker.new_protocol()` 同步调用
- 删除了 `ratatui_image::{Image, Resize, FilterType}` 导入
- `cache_width` 变化清空缓存的逻辑保留在 `main.rs`（下一步处理）

- [ ] **Step 7: 扩展 collect_loads 处理浏览器缓存**

修改 `collect_loads`，收到结果时根据状态分发：

```rust
pub fn collect_loads(&mut self) {
    while let Ok((idx, proto)) = self.load_rx.try_recv() {
        if self.state == AppState::Fullscreen && idx == self.selected {
            self.fullscreen_protocol = Some(proto);
            self.fullscreen_pending = false;
        } else {
            self.protocol_cache.insert(idx, proto);
        }
    }
}
```

- [ ] **Step 8: 更新 main.rs**

适配新类型和函数签名：

```rust
// main.rs: spawn_image_loader 的 channel 类型自动适配（LoadRequest）
let (load_tx, load_rx) = spawn_image_loader(picker.clone(), paths);

// 事件循环中，缓存清空逻辑保留在 main.rs：
if size.width != last_terminal_width {
    app.clear_protocol_cache();
    last_terminal_width = size.width;
}

// 浏览器模式调用（去掉 prefetch 参数）：
if app.state == AppState::Browser {
    populate_protocol_cache(&app, cell_w, cell_h, size.width, visible_rows.max(1));
}
```

移除 `main.rs` 中的 `use ui::browser::populate_protocol_cache;` 如果用到了不变。

- [ ] **Step 9: 更新测试**

`src/app.rs` 测试适配新类型：

```rust
fn make_app(count: usize) -> App {
    let images = (0..count)
        .map(|i| ImageEntry {
            path: PathBuf::from(format!("img{:03}.png", i)),
            filename: format!("img{:03}.png", i),
        })
        .collect();
    let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<(usize, Protocol)>();
    App::new(images, AppState::Browser, Picker::halfblocks(), tx, rx2)
}
```

- [ ] **Step 10: 编译并运行测试**

```bash
cargo test
```

预期：全部测试通过。

- [ ] **Step 11: 提交**

```bash
git add src/app.rs src/main.rs src/ui/browser.rs
git commit -m "refactor: 统一加载管线，浏览器缩略图异步化

- LoadRequest + LoadSize 替代 usize，支持 Thumbnail/Original 两种模式
- populate_protocol_cache 改为纯异步：只发请求不阻塞
- collect_loads 统一分发浏览器缓存和全屏 Protocol
- 主线程永不调用 image::open() + picker.new_protocol()"
```

---

### Task 4: 有界 FIFO 缓存

**Files:**
- Modify: `src/app.rs:19-31` (App struct), `src/app.rs:143-151` (collect_loads)

- [ ] **Step 1: App 添加缓存容量常量和插入方法**

在 `src/app.rs` 顶部添加常量：

```rust
/// Maximum number of cached protocols before eviction.
const MAX_CACHE_SIZE: usize = 200;
```

在 `impl App` 中修改 `collect_loads`，将直接 `insert` 改为带淘汰逻辑的插入：

```rust
pub fn collect_loads(&mut self) {
    while let Ok((idx, proto)) = self.load_rx.try_recv() {
        if self.state == AppState::Fullscreen && idx == self.selected {
            self.fullscreen_protocol = Some(proto);
            self.fullscreen_pending = false;
        } else {
            self.insert_cache(idx, proto);
        }
    }
}

fn insert_cache(&mut self, idx: usize, proto: Protocol) {
    self.protocol_cache.insert(idx, proto);
    if self.protocol_cache.len() > MAX_CACHE_SIZE {
        // 清掉最早的 MAX_CACHE_SIZE/2 个条目
        let remove_count = MAX_CACHE_SIZE / 2;
        let stale: Vec<usize> = self
            .protocol_cache
            .keys()
            .take(remove_count)
            .copied()
            .collect();
        for k in stale {
            self.protocol_cache.remove(&k);
        }
    }
}
```

- [ ] **Step 2: 编译并测试**

```bash
cargo test
cargo clippy
```

预期：全部通过。

- [ ] **Step 3: 提交**

```bash
git add src/app.rs
git commit -m "feat: protocol_cache FIFO 有界淘汰

- 容量上限 MAX_CACHE_SIZE = 200
- 超量时淘汰最早插入的一半条目（利用 HashMap 插入顺序）
- 防止大目录下内存无上限增长"
```

---

### Task 5: 相邻预取

**Files:**
- Modify: `src/app.rs:19-31` (App struct, 新增 requested 字段)
- Modify: `src/ui/browser.rs:173-212` (populate_protocol_cache)

- [ ] **Step 1: App 新增 requested 去重集**

在 `src/app.rs` 的 `App` struct 中使用 `std::collections::HashSet`：

```rust
use std::collections::{HashMap, HashSet};

pub struct App {
    // ... 现有字段 ...
    pub requested: HashSet<usize>,   // 新增：已发送但未收到的加载请求
}
```

在 `App::new` 中初始化：

```rust
requested: HashSet::new(),
```

- [ ] **Step 2: request_load 添加去重标记**

```rust
pub fn request_load(&mut self, idx: usize, size: LoadSize) {
    if self.requested.contains(&idx) {
        return;
    }
    self.requested.insert(idx);
    let _ = self.load_tx.send(LoadRequest { idx, size });
}
```

注意：`request_load` 现在需要 `&mut self`（因为要修改 `requested` set）。

- [ ] **Step 3: collect_loads 清除去重标记**

在收到结果后从 `requested` 移除：

```rust
pub fn collect_loads(&mut self) {
    while let Ok((idx, proto)) = self.load_rx.try_recv() {
        self.requested.remove(&idx);  // 新增
        if self.state == AppState::Fullscreen && idx == self.selected {
            self.fullscreen_protocol = Some(proto);
            self.fullscreen_pending = false;
        } else {
            self.insert_cache(idx, proto);
        }
    }
}
```

- [ ] **Step 4: clear_protocol_cache 也清除 requested**

```rust
pub fn clear_protocol_cache(&mut self) {
    self.protocol_cache.clear();
    self.requested.clear();   // 新增
    self.cache_width = 0;
}
```

- [ ] **Step 5: populate_protocol_cache 实现预取**

修改 `src/ui/browser.rs`，利用 `prefetch` 参数：

```rust
pub fn populate_protocol_cache(
    app: &mut App,   // 改为 &mut（request_load 需要）
    cell_w: u16,
    cell_h: u16,
    terminal_width: u16,
    visible_rows: usize,
) {
    if cell_w < 2 || cell_h < 2 {
        return;
    }

    // 终端宽度变化时清空缓存（cell 尺寸变了，旧 Protocol 作废）
    if app.cache_width != terminal_width {
        app.clear_protocol_cache();
        app.cache_width = terminal_width;
    }

    let thumb_w = cell_w.saturating_sub(2);
    let thumb_h = cell_h.saturating_sub(3);
    let size = LoadSize::Thumbnail { w: thumb_w, h: thumb_h };

    let start = app.scroll_row * IMAGES_PER_ROW;
    let visible_end = (start + visible_rows * IMAGES_PER_ROW).min(app.images.len());

    // 预取：前后各扩展一行
    let prefetch_start = start.saturating_sub(IMAGES_PER_ROW);
    let prefetch_end = (visible_end + IMAGES_PER_ROW).min(app.images.len());

    for slot in prefetch_start..prefetch_end {
        if app.protocol_cache.contains_key(&slot) || app.requested.contains(&slot) {
            continue;
        }
        app.request_load(slot, size.clone());
    }
}
```

注意：
- 函数签名不再需要 `prefetch: bool` 参数（始终预取）
- `app` 参数改为 `&mut App`（因为 `request_load` 需要 `&mut`）
- 预取范围从 `prefetch_start` 到 `prefetch_end`，包含可见区域
- 同时检查 `protocol_cache` 和 `requested`，避免重复请求

- [ ] **Step 6: 更新 main.rs 调用**

`populate_protocol_cache` 签名变了（去掉了 `prefetch` 参数），更新调用：

```rust
if app.state == AppState::Browser {
    populate_protocol_cache(&mut app, cell_w, cell_h, size.width, visible_rows.max(1));
}
```

- [ ] **Step 7: 编译并测试**

```bash
cargo test
cargo clippy
```

预期：全部通过。

- [ ] **Step 8: 提交**

```bash
git add src/app.rs src/ui/browser.rs src/main.rs
git commit -m "feat: 相邻预取 + requested 去重

- 新增 requested: HashSet<usize> 防止重复发送同一 slot 的加载请求
- populate_protocol_cache 扩展请求范围为可见区域 ±1 行
- 收到结果后自动从 requested 移除
- 缓存清空时也清空 requested"
```

---

### Task 6: 消除 take/put-back，简化 draw 签名

**Files:**
- Modify: `src/main.rs:120-124` (渲染部分)
- Modify: `src/ui/mod.rs:10-27` (draw 函数)
- Modify: `src/ui/preview.rs:11-14` (PreviewView, 直接从 app 读取)

- [ ] **Step 1: preview.rs 直接从 app 读取 protocol**

`PreviewView` 已有 `app: &'a App`，通过 `self.app.fullscreen_protocol` 访问：

```rust
impl<'a> Widget for PreviewView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status_height = 1u16;
        let image_area = Rect {
            height: area.height.saturating_sub(status_height),
            ..area
        };
        let status_area = Rect {
            y: area.y + image_area.height,
            height: status_height,
            ..area
        };

        // 直接从 app 读取，不再需要外部 protocol 参数
        if let Some(ref proto) = self.app.fullscreen_protocol {
            let proto_size = proto.size();
            let offset_x = image_area
                .width
                .saturating_sub(proto_size.width)
                / 2;
            let offset_y = image_area
                .height
                .saturating_sub(proto_size.height)
                / 2;
            let centered = Rect {
                x: image_area.x + offset_x,
                y: image_area.y + offset_y,
                width: proto_size.width.min(image_area.width),
                height: proto_size.height.min(image_area.height),
            };
            Image::new(proto).allow_clipping(true).render(centered, buf);
        } else {
            Block::default()
                .borders(Borders::NONE)
                .render(image_area, buf);
        }

        // 状态栏保持不变
        if let Some(entry) = self.app.images.get(self.app.selected) {
            let status = if self.app.fullscreen_pending {
                " ⏳ 加载中..."
            } else {
                ""
            };
            let info = format!(
                " {} [{}/{}]  原图尺寸  ← → 切换  Enter/Esc/q 返回{}",
                entry.filename,
                self.app.selected + 1,
                self.app.images.len(),
                status,
            );
            let span = Span::styled(info, Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(span)
                .alignment(Alignment::Left)
                .render(status_area, buf);
        }
    }
}
```

关键改动：`self.protocol` → `self.app.fullscreen_protocol`，且用 `if let Some(ref proto) = ...` 模式匹配。

- [ ] **Step 2: ui/mod.rs 移除 protocol 参数**

```rust
pub fn draw(
    frame: &mut Frame,
    app: &mut App,
    cell_w: u16,
    cell_h: u16,
) {
    let area = frame.area();
    match app.state {
        AppState::Browser => {
            frame.render_widget(BrowserView { app, cell_w, cell_h }, area);
        }
        AppState::Fullscreen => {
            frame.render_widget(PreviewView { app }, area);  // 不再传 protocol
        }
    }
}
```

`PreviewView` 构造不再需要 `protocol` 字段。

- [ ] **Step 3: PreviewView 结构体移除 protocol 字段**

```rust
pub struct PreviewView<'a> {
    pub app: &'a App,
}
```

从 imports 中移除 `use ratatui_image::protocol::Protocol;`（如果不再需要）。

- [ ] **Step 4: main.rs 移除 take/put-back**

```rust
// 修改前
let proto = app.fullscreen_protocol.take();
let proto_ref = proto.as_ref();
terminal.draw(|f| ui::draw(f, &mut app, cell_w, cell_h, proto_ref))?;
app.fullscreen_protocol = proto;

// 修改后
terminal.draw(|f| ui::draw(f, &mut app, cell_w, cell_h))?;
```

- [ ] **Step 5: 清理 ui/mod.rs 的 imports**

移除不再需要的 `use ratatui_image::protocol::Protocol;` 导入。

- [ ] **Step 6: 编译并运行测试**

```bash
cargo test
cargo clippy
```

预期：全部通过。

- [ ] **Step 7: 提交**

```bash
git add src/main.rs src/ui/mod.rs src/ui/preview.rs
git commit -m "refactor: 消除 take/put-back，PreviewView 直接从 app 读取

- PreviewView 不再需要外部 protocol 参数，通过 self.app 直接访问
- draw() 签名简化为 draw(frame, app, cell_w, cell_h)
- main.rs 删除 take/put-back 变通代码"
```

---

## 验证清单

全部 Tasks 完成后运行：

```bash
cargo test                    # 全部测试通过
cargo clippy                  # 无 lint 警告
cargo build --release         # release 编译通过
cargo run -- <图片目录>       # 手动验证：滚动无卡顿，全屏切换流畅
```
