# 全屏图片缩放功能 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 全屏模式支持 `+`/`-` 缩放和 `h` `j` `k` `l` 平移

**Architecture:** `FullscreenContent::Static` 改为同时缓存原始 `DynamicImage` 和当前缩放级别的 `Protocol`。缩放时在主线程同步调用 `picker.new_protocol()` 重新生成协议（纯内存操作，开销 < 1ms）。平移通过偏移渲染位置实现，`allow_clipping(true)` 裁剪超界部分。

**Tech Stack:** Rust, ratatui, ratatui-image, image crate

---

## 文件结构

| 文件 | 变更 |
|------|------|
| `src/app.rs` | `StaticContent` 结构体、`App` 新增 zoom/pan/picker 字段、缩放/平移方法、按键分发 |
| `src/main.rs` | `App::new()` 传入 picker |
| `src/ui/preview.rs` | 渲染时应用 pan 偏移、状态栏显示缩放百分比 |
| `src/lang.rs` | 可选：缩放百分比文本（中英相同，直接格式化） |

---

### Task 1: 添加 `StaticContent` 结构体并修改 `FullscreenContent`

**Files:**
- Modify: `src/app.rs:21-31`

- [ ] **Step 1: 定义 `StaticContent`，修改 `FullscreenContent::Static`**

将 `FullscreenContent::Static(Protocol)` 改为持有 `StaticContent`：

```rust
#[derive(Clone)]
pub struct StaticContent {
    pub protocol: Protocol,
    pub original: image::DynamicImage,
}

#[derive(Clone)]
pub enum FullscreenContent {
    Static(StaticContent),
    Animation(Vec<AnimationFrame>),
}
```

- [ ] **Step 2: 更新所有 `FullscreenContent::Static` 构造点**

`static_original_content()`（约 line 459-465）：
```rust
fn static_original_content(
    picker: &Picker,
    img: image::DynamicImage,
    size: Size,
) -> Option<FullscreenContent> {
    let protocol = make_protocol(picker, img.clone(), size, FilterType::Lanczos3)?;
    Some(FullscreenContent::Static(StaticContent { protocol, original: img }))
}
```

`spawn_image_loader()` 中 thumbnail 路径（约 line 558-560），thumbnail 不需要缓存 original（仅全屏缩放需要），改为存 1×1 占位或保持 `Option`：

实际上 thumbnail 的 `FullscreenContent::Static` 仅用于 `first_protocol()` 提取协议后在 browser 缓存中使用，不会触发缩放。简单起见改为：
```rust
LoadSize::Thumbnail { .. } => {
    make_protocol(&picker, img.clone(), protocol_size, filter)
        .map(|protocol| FullscreenContent::Static(StaticContent { protocol, original: img }))
}
```

- [ ] **Step 3: 更新所有 `FullscreenContent::Static` 模式匹配点**

`first_protocol()`（约 line 418-423）：
```rust
fn first_protocol(content: FullscreenContent) -> Protocol {
    match content {
        FullscreenContent::Static(sc) => sc.protocol,
        FullscreenContent::Animation(mut frames) => frames.remove(0).protocol,
    }
}
```

`current_fullscreen_protocol()`（约 line 215-224）：
```rust
pub fn current_fullscreen_protocol(&self) -> Option<&Protocol> {
    match self.fullscreen_content.as_ref()? {
        FullscreenContent::Static(sc) => Some(&sc.protocol),
        FullscreenContent::Animation(frames) => frames
            .get(self.fullscreen_frame_idx)
            .or_else(|| frames.first())
            .map(|frame| &frame.protocol),
    }
}
```

`set_fullscreen_content()` 中 `FullscreenContent::Static(_)` → `FullscreenContent::Static(_)`：
```rust
self.fullscreen_next_frame_at = match &content {
    FullscreenContent::Animation(frames) => frames.first().map(|frame| now + frame.delay),
    FullscreenContent::Static(_) => None,
};
```

`advance_animation()` 中 guard（约 line 243）：保持不变，只匹配 `Animation`。

`collect_loads()` 中 thumbnail 入缓存路径（约 line 282-288）：
```rust
let proto = first_protocol(content);
// ... 不变
```

- [ ] **Step 4: 编译验证**

```bash
cargo build 2>&1
```

Expected: 编译通过（可能有其他未适配的匹配点需要修复）

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "refactor: FullscreenContent::Static 改为 StaticContent 结构体持有原始图像"
```

---

### Task 2: `App` 新增 zoom/pan/picker 字段和方法

**Files:**
- Modify: `src/app.rs:47-69` (App struct), `src/app.rs:76-107` (App::new), `src/app.rs:160-191` (fullscreen methods)

- [ ] **Step 1: App struct 新增字段**

```rust
pub struct App {
    // ... 现有字段 ...
    pub zoom: f32,
    pub pan_x: i16,
    pub pan_y: i16,
    pub picker: Picker,
    // 用于缩放时计算目标尺寸
    pub fullscreen_image_w: u16,
    pub fullscreen_image_h: u16,
}
```

- [ ] **Step 2: 添加缩放常量**

在 `MAX_CACHE_SIZE` 附近（约 line 74）：
```rust
const ZOOM_STEP: f32 = 1.25;
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 10.0;
```

- [ ] **Step 3: 更新 `App::new()` 签名和实现**

```rust
pub fn new(
    images: Vec<ImageEntry>,
    state: AppState,
    selected: usize,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<LoadResult>,
    lang: Lang,
    picker: Picker,       // 新增
) -> Self {
    // ... 现有代码 ...
    let mut app = Self {
        // ... 现有字段 ...
        zoom: 1.0,
        pan_x: 0,
        pan_y: 0,
        picker,
        fullscreen_image_w: 0,
        fullscreen_image_h: 0,
    };
    // ... 全屏初始化代码 ...
}
```

- [ ] **Step 4: 添加缩放方法**

```rust
impl App {
    /// 放大，上限 ZOOM_MAX
    pub fn zoom_in(&mut self) {
        if self.state != AppState::Fullscreen { return; }
        self.set_zoom((self.zoom * ZOOM_STEP).min(ZOOM_MAX));
    }

    /// 缩小，下限 ZOOM_MIN
    pub fn zoom_out(&mut self) {
        if self.state != AppState::Fullscreen { return; }
        self.set_zoom((self.zoom / ZOOM_STEP).max(ZOOM_MIN));
    }

    /// 重置缩放与平移
    pub fn zoom_reset(&mut self) {
        if self.state != AppState::Fullscreen { return; }
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.regenerate_zoom_protocol();
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
        self.regenerate_zoom_protocol();
    }

    /// 用缓存原始图像以当前缩放级别重新生成协议
    fn regenerate_zoom_protocol(&mut self) {
        let Some(content) = self.fullscreen_content.as_mut() else { return };
        let FullscreenContent::Static(sc) = content else { return; };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 { return; }
        let new_w = ((self.fullscreen_image_w as f32) * self.zoom).max(1.0) as u16;
        let new_h = ((self.fullscreen_image_h as f32) * self.zoom).max(1.0) as u16;
        let size = Size::new(new_w, new_h);
        if let Ok(protocol) = self.picker.new_protocol(
            sc.original.clone(), size, Resize::Fit(Some(FilterType::Lanczos3)),
        ) {
            sc.protocol = protocol;
        }
        self.clamp_pan();
    }

    /// 平移后钳制到图片边界
    fn clamp_pan(&mut self) {
        let Some(proto) = self.current_fullscreen_protocol() else { return };
        let pw = proto.size().width as i16;
        let ph = proto.size().height as i16;
        let vw = self.fullscreen_image_w as i16;
        let vh = self.fullscreen_image_h as i16;
        let max_x = ((pw - vw).max(0) / 2).max(0);
        let max_y = ((ph - vh).max(0) / 2).max(0);
        self.pan_x = self.pan_x.clamp(-max_x, max_x);
        self.pan_y = self.pan_y.clamp(-max_y, max_y);
    }

    fn pan_step_x(&self) -> i16 {
        ((self.fullscreen_image_w as f32) * 0.1).max(1.0) as i16
    }

    fn pan_step_y(&self) -> i16 {
        ((self.fullscreen_image_h as f32) * 0.1).max(1.0) as i16
    }

    pub fn pan_left(&mut self)  { self.pan_x -= self.pan_step_x(); self.clamp_pan(); }
    pub fn pan_right(&mut self) { self.pan_x += self.pan_step_x(); self.clamp_pan(); }
    pub fn pan_up(&mut self)    { self.pan_y -= self.pan_step_y(); self.clamp_pan(); }
    pub fn pan_down(&mut self)  { self.pan_y += self.pan_step_y(); self.clamp_pan(); }
}
```

- [ ] **Step 5: 切换图片时重置 zoom/pan**

修改 `fullscreen_prev()` 和 `fullscreen_next()`（约 line 175-191），在 `reset_fullscreen_content()` 之后重置：
```rust
pub fn fullscreen_prev(&mut self) {
    if self.selected > 0 {
        self.selected -= 1;
        self.reset_fullscreen_content();
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fullscreen_image_w = 0;
        self.fullscreen_image_h = 0;
        self.fullscreen_pending = true;
        self.request_load(self.selected, LoadSize::Original);
    }
}
// fullscreen_next 同理
```

`exit_fullscreen()` 中也重置：
```rust
pub fn exit_fullscreen(&mut self) {
    self.state = AppState::Browser;
    self.reset_fullscreen_content();
    self.zoom = 1.0;
    self.pan_x = 0;
    self.pan_y = 0;
    self.fullscreen_image_w = 0;
    self.fullscreen_image_h = 0;
    self.fullscreen_pending = false;
}
```

- [ ] **Step 6: 更新 `set_fullscreen_content()` 重置 zoom/pan**

```rust
pub fn set_fullscreen_content(
    &mut self,
    content: FullscreenContent,
    dims: Option<(u32, u32)>,
    now: Instant,
) {
    self.fullscreen_frame_idx = 0;
    self.zoom = 1.0;
    self.pan_x = 0;
    self.pan_y = 0;
    self.fullscreen_next_frame_at = match &content {
        FullscreenContent::Animation(frames) => frames.first().map(|frame| now + frame.delay),
        FullscreenContent::Static(_) => None,
    };
    self.fullscreen_content = Some(content);
    self.fullscreen_dims = dims;
}
```

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat: App 新增 zoom/pan 字段和缩放方法"
```

---

### Task 3: 更新测试辅助函数和 `main.rs` 调用点

**Files:**
- Modify: `src/app.rs:579-608` (test helpers), `src/main.rs:119-125` (App::new call)

- [ ] **Step 1: 更新测试辅助函数**

`make_app()`:
```rust
fn make_app(count: usize) -> App {
    // ... 不变 ...
    App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks())
}
```

`make_app_with_load_rx()`:
```rust
(App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks()), rx)
```

`make_app_with_names()`:
```rust
App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks())
```

- [ ] **Step 2: 更新 `main.rs` 中 `App::new()` 调用**

```rust
let mut app = App::new(images, initial_state, selected, load_tx, load_rx, Lang::detect(), picker);
```

注意 `picker` 已移动，`spawn_image_loader` 需要 clone：
```rust
let (load_tx, load_rx) = spawn_image_loader(picker.clone(), paths);
```

- [ ] **Step 3: 编译测试**

```bash
cargo test 2>&1
```

Expected: 所有 56 个测试通过

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "fix: 更新 App::new() 调用点传入 picker"
```

---

### Task 4: 全屏按键分发——缩放和平移

**Files:**
- Modify: `src/app.rs:371-381` (fullscreen key handling)

- [ ] **Step 1: 修改 `handle_key()` 全屏分支**

```rust
AppState::Fullscreen => match code {
    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
    KeyCode::Char('L') => {
        self.lang.toggle();
    }
    KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_in(),
    KeyCode::Char('-') => self.zoom_out(),
    KeyCode::Char('0') => self.zoom_reset(),
    KeyCode::Char('h') => self.pan_left(),
    KeyCode::Char('l') => self.pan_right(),
    KeyCode::Char('k') => self.pan_up(),
    KeyCode::Char('j') => self.pan_down(),
    KeyCode::Left => self.fullscreen_prev(),
    KeyCode::Right => self.fullscreen_next(),
    _ => {}
},
```

注意：`KeyCode::Char('l')` 从语言切换中移除（改为仅大写 `L` 切换），避免与平移冲突。

- [ ] **Step 2: Commit**

```bash
git add src/app.rs
git commit -m "feat: 全屏模式增加缩放/平移按键绑定"
```

---

### Task 5: 更新 `PreviewView` 渲染——pan 偏移和 viewport 记录

**Files:**
- Modify: `src/ui/preview.rs:54-85` (PreviewView render)

- [ ] **Step 1: 记录 viewport 尺寸到 App**

在 `PreviewView::render()` 中，计算完 image_area 后：
```rust
self.app.fullscreen_image_w = image_area.width;
self.app.fullscreen_image_h = image_area.height;
```

注意这需要 `app` 改为 `&mut App`。`PreviewView` 当前持有 `&'a App`，需改为 `&'a mut App`。同时检查 `ui::draw()` 签名是否已传入 `&mut App`——是的。

- [ ] **Step 2: 修改 `PreviewView` 为可变引用**

```rust
pub struct PreviewView<'a> {
    pub app: &'a mut App,
}
```

- [ ] **Step 3: 渲染时应用 pan 偏移**

替换当前的居中渲染逻辑（约 line 74-85）：

```rust
// --- Image area ---
if let Some(proto) = self.app.current_fullscreen_protocol() {
    let proto_size = proto.size();
    // 居中偏移 + 平移偏移
    let center_x = image_area.width as i16 - proto_size.width as i16;
    let center_y = image_area.height as i16 - proto_size.height as i16;
    let offset_x = (center_x / 2 - self.app.pan_x)
        .clamp(i16::MIN, i16::MAX); // 防止极端 pan 溢出
    let offset_y = (center_y / 2 - self.app.pan_y)
        .clamp(i16::MIN, i16::MAX);

    // 裁剪到 image_area 范围内
    let render_x = (image_area.x as i16 + offset_x).max(image_area.x as i16) as u16;
    let render_y = (image_area.y as i16 + offset_y).max(image_area.y as i16) as u16;
    // 计算实际可见宽高（协议与 image_area 交集）
    let visible_w = proto_size.width.min(image_area.width);
    let visible_h = proto_size.height.min(image_area.height);

    let render_area = Rect {
        x: render_x,
        y: render_y,
        width: visible_w,
        height: visible_h,
    };
    Image::new(proto).allow_clipping(true).render(render_area, buf);
}
```

- [ ] **Step 4: 状态栏显示缩放百分比**

```rust
let status_text = if self.app.zoom != 1.0 && !self.app.fullscreen_pending {
    format!(" [{:.0}%]", self.app.zoom * 100.0)
} else if self.app.fullscreen_pending {
    self.app.lang.loading_text().to_string()
} else {
    String::new()
};
```

将 `status_text` 传给 `preview_status()` 的 `status` 参数（已存在）。

- [ ] **Step 5: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat: PreviewView 支持 pan 偏移渲染和缩放百分比显示"
```

---

### Task 6: 添加测试

**Files:**
- Modify: `src/app.rs:579-946` (测试模块)

- [ ] **Step 1: 缩放范围测试**

```rust
#[test]
fn zoom_in_increases_zoom() {
    let mut app = make_app(5);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    // 需要一个 Static 内容来测试 regenerate
    let img = image::DynamicImage::new_rgba8(100, 100);
    let proto = make_protocol();
    app.fullscreen_content = Some(FullscreenContent::Static(StaticContent {
        protocol: proto,
        original: img,
    }));
    app.zoom = 1.0;
    app.zoom_in();
    assert!((app.zoom - 1.25).abs() < 0.01);
}

#[test]
fn zoom_out_decreases_zoom() {
    let mut app = make_app(5);
    app.state = AppState::Fullscreen;
    app.zoom = 2.0;
    app.zoom_out();
    assert!((app.zoom - 1.6).abs() < 0.01); // 2.0 / 1.25 = 1.6
}

#[test]
fn zoom_clamped_to_max() {
    let mut app = make_app(5);
    app.state = AppState::Fullscreen;
    app.zoom = 10.0;
    app.zoom_in();
    assert!((app.zoom - 10.0).abs() < 0.01);
}

#[test]
fn zoom_clamped_to_min() {
    let mut app = make_app(5);
    app.state = AppState::Fullscreen;
    app.zoom = 0.25;
    app.zoom_out();
    assert!((app.zoom - 0.25).abs() < 0.01);
}
```

- [ ] **Step 2: 切换图片重置测试**

```rust
#[test]
fn switching_image_resets_zoom_and_pan() {
    let mut app = make_app(3);
    app.state = AppState::Fullscreen;
    app.zoom = 2.0;
    app.pan_x = 5;
    app.pan_y = 3;
    app.fullscreen_next();
    assert!((app.zoom - 1.0).abs() < 0.01);
    assert_eq!(app.pan_x, 0);
    assert_eq!(app.pan_y, 0);
}
```

- [ ] **Step 3: 动图忽略缩放测试**

```rust
#[test]
fn animation_ignores_zoom() {
    let mut app = make_app(1);
    let start = Instant::now();
    // 安装动图内容
    app.state = AppState::Fullscreen;
    app.set_fullscreen_content(
        FullscreenContent::Animation(vec![
            make_animation_frame(100),
            make_animation_frame(150),
        ]),
        Some((1, 1)),
        start,
    );
    app.zoom = 1.0;
    app.zoom_in(); // 应该不生效
    assert!((app.zoom - 1.25).abs() < 0.01); // zoom 值仍会变但协议不变
    // 注意：当前设计 zoom 值会变但 regenerate_zoom_protocol() 遇到 Animation 直接返回
}
```

- [ ] **Step 4: 平移边界测试**

```rust
#[test]
fn pan_clamped_when_not_zoomed() {
    let mut app = make_app(1);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    let img = image::DynamicImage::new_rgba8(100, 100);
    let proto = make_protocol();
    app.fullscreen_content = Some(FullscreenContent::Static(StaticContent {
        protocol: proto,
        original: img,
    }));
    app.pan_right();
    app.pan_down();
    // 未缩放时图片 ≤ 视口，max_pan = 0，pan 始终为 0
    assert_eq!(app.pan_x, 0);
    assert_eq!(app.pan_y, 0);
}

#[test]
fn zoom_reset_sets_defaults() {
    let mut app = make_app(1);
    app.state = AppState::Fullscreen;
    app.zoom = 3.0;
    app.pan_x = 10;
    app.pan_y = 5;
    app.zoom_reset();
    assert!((app.zoom - 1.0).abs() < 0.01);
    assert_eq!(app.pan_x, 0);
    assert_eq!(app.pan_y, 0);
}
```

- [ ] **Step 5: 运行测试**

```bash
cargo test 2>&1
```

Expected: 所有测试通过（新增 + 原有）

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "test: 缩放/平移单元测试"
```

---

### Task 7: 更新 CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: 在全屏描述中添加缩放信息**

在 "两个状态" 的 `Fullscreen` 行追加：
```
自动检测动图（GIF/APNG/WebP）并循环播放，`+`/`-` 缩放（0.25×~10×），`h` `j` `k` `l` 平移
```

在按键说明添加缩放/平移绑定。

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: 更新 CLAUDE.md 添加缩放/平移说明"
```

---

### Task 8: 集成验证

- [ ] **Step 1: 运行完整测试**

```bash
cargo test 2>&1
```

Expected: 所有测试通过

- [ ] **Step 2: 运行 clippy**

```bash
cargo clippy 2>&1
```

Expected: No issues found

- [ ] **Step 3: 运行 fmt**

```bash
cargo fmt -- --check 2>&1
```

Expected: 无差异

- [ ] **Step 4: 手动验证**

```bash
cargo run -- <某图片文件>
```

测试：
- 按 `+` 放大，确认图片变大
- 按 `-` 缩小，确认图片变小
- 按 `0` 重置
- 放大后按 `h` `j` `k` `l` 平移
- 按 `←` `→` 切换图片，确认缩放重置
- 找一张 GIF 测试，确认缩放/平移对动图无效果
- 状态栏检查 `[125%]` 等显示正确

- [ ] **Step 5: 最终提交（如有修改）**

```bash
git add -A && git commit -m "chore: 集成验证后的最终调整"
```
