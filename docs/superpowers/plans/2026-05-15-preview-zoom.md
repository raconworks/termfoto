# Preview Zoom Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Ctrl++/Ctrl+-/Ctrl+0 zoom controls to preview mode, persisting zoom level across image navigation.

**Architecture:** Add `zoom_factor: f32` to `App` with `zoom_in/out/reset()` methods. In `run()`, track `last_zoom_factor` alongside `last_preview_index` and rebuild the ratatui-image `StatefulProtocol` whenever either changes. Before passing the image to the protocol, call `preprocess_image_for_zoom()` which center-crops (zoom > 1.0) or shrinks+centers on a black canvas (zoom < 1.0). The status bar in `PreviewView` reads `app.zoom_factor` directly.

**Tech Stack:** Rust, ratatui 0.28, ratatui-image v2 (Kitty/Sixel/Unicode auto-detect), image 0.25 (`crop_imm`, `resize`, `imageops::overlay`), crossterm 0.28, clap 4

---

## File Map

| File | Change |
|------|--------|
| `src/app.rs` | Add `zoom_factor: f32` field; `zoom_in/out/reset()` methods; 6 new tests |
| `src/ui/preview.rs` | Read `self.app.zoom_factor` in render; update status bar string; extend existing test |
| `src/main.rs` | Add zoom key bindings in `handle_key()`; add `last_zoom_factor`; add `preprocess_image_for_zoom()` fn; update rebuild condition; 4 new tests |

No changes needed to `src/ui/mod.rs`, `src/ui/grid.rs`, `src/scanner.rs`, or `Cargo.toml`.

---

## Task 1: App zoom state

**Files:**
- Modify: `src/app.rs`

### Background

`App` struct currently has 5 fields (`state`, `images`, `selected`, `scroll_offset`, `grid_cols`). We add `zoom_factor: f32` with range 0.1–5.0 and step 0.1. All zoom operations round to 1 decimal place to prevent floating-point drift (e.g. repeated 0.1 additions would otherwise drift to `0.30000000000000004`).

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)]` block at the bottom of `src/app.rs`:

```rust
    #[test]
    fn zoom_in_increments_by_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_in();
        assert_eq!(app.zoom_factor, 1.1);
    }

    #[test]
    fn zoom_in_capped_at_5() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 5.0;
        app.zoom_in();
        assert_eq!(app.zoom_factor, 5.0);
    }

    #[test]
    fn zoom_out_decrements_by_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_out();
        assert_eq!(app.zoom_factor, 0.9);
    }

    #[test]
    fn zoom_out_floored_at_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 0.1;
        app.zoom_out();
        assert_eq!(app.zoom_factor, 0.1);
    }

    #[test]
    fn zoom_reset_returns_to_1() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 3.5;
        app.zoom_reset();
        assert_eq!(app.zoom_factor, 1.0);
    }

    #[test]
    fn zoom_no_float_drift() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 0.1;
        app.zoom_in(); // 0.2
        app.zoom_in(); // 0.3
        app.zoom_in(); // 0.4
        assert_eq!(app.zoom_factor, 0.4);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test zoom_ -- --test-thread=1 2>&1 | head -30
```

Expected: compilation error "no field `zoom_factor`" or "no method `zoom_in`".

- [ ] **Step 3: Add zoom_factor field and methods**

In `src/app.rs`, update the `App` struct:

```rust
pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub grid_cols: usize,
    pub zoom_factor: f32,
}
```

Update `App::new()`:

```rust
pub fn new(images: Vec<ImageEntry>, state: AppState) -> Self {
    Self {
        state,
        images,
        selected: 0,
        scroll_offset: 0,
        grid_cols: 1,
        zoom_factor: 1.0,
    }
}
```

Add the three zoom methods (insert after `preview_next`, before `load_visible_thumbnails`):

```rust
    pub fn zoom_in(&mut self) {
        self.zoom_factor = ((self.zoom_factor + 0.1) * 10.0).round() / 10.0;
        self.zoom_factor = self.zoom_factor.min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom_factor = ((self.zoom_factor - 0.1) * 10.0).round() / 10.0;
        self.zoom_factor = self.zoom_factor.max(0.1);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom_factor = 1.0;
    }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test zoom_ 2>&1 | tail -10
```

Expected: `6 passed; 0 failed`

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass (previously ~40, now ~46).

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: add zoom_factor state and zoom_in/out/reset methods"
```

---

## Task 2: Status bar zoom display

**Files:**
- Modify: `src/ui/preview.rs`

### Background

`PreviewView` already has `pub app: &'a App`, so it can read `self.app.zoom_factor` directly — no struct changes needed. The status bar currently shows `" {filename} [{idx}/{total}]  ← → 切换  Esc/q 返回"`. We append the zoom level: `"Fit"` when `zoom_factor == 1.0`, else `"200%"` etc. Float comparison is safe here because zoom_factor is always set by exact operations (`1.0`, `x ± 0.1` rounded).

- [ ] **Step 1: Write the failing test**

In `src/ui/preview.rs`, add a test after `test_preview_status_bar_shows_filename`:

```rust
    #[test]
    fn test_preview_status_bar_shows_zoom() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.zoom_factor = 2.0;

        terminal
            .draw(|f| {
                let widget = PreviewView { app: &app, image_state: None };
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let last_row: String = (0..80u16)
            .map(|x| buf.cell((x, 23)).map(|c| c.symbol().chars().next().unwrap_or(' ')).unwrap_or(' '))
            .collect();
        assert!(last_row.contains("200%"), "Status bar should show zoom level, got: {last_row:?}");
    }

    #[test]
    fn test_preview_status_bar_shows_fit_at_default_zoom() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = make_app(); // zoom_factor == 1.0

        terminal
            .draw(|f| {
                let widget = PreviewView { app: &app, image_state: None };
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let last_row: String = (0..80u16)
            .map(|x| buf.cell((x, 23)).map(|c| c.symbol().chars().next().unwrap_or(' ')).unwrap_or(' '))
            .collect();
        assert!(last_row.contains("Fit"), "Status bar should show 'Fit' at default zoom, got: {last_row:?}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_preview_status_bar_shows_zoom 2>&1 | tail -10
```

Expected: FAIL — status bar does not yet contain "200%" or "Fit".

- [ ] **Step 3: Update the status bar render**

In `src/ui/preview.rs`, find the `if let Some(entry) = self.app.images.get(self.app.selected)` block and replace the `info` format string:

Old code:
```rust
        if let Some(entry) = self.app.images.get(self.app.selected) {
            let info = format!(
                " {} [{}/{}]  ← → 切换  Esc/q 返回",
                entry.filename,
                self.app.selected + 1,
                self.app.images.len()
            );
```

New code:
```rust
        if let Some(entry) = self.app.images.get(self.app.selected) {
            let zoom_str = if self.app.zoom_factor == 1.0 {
                "Fit".to_string()
            } else {
                format!("{}%", (self.app.zoom_factor * 100.0).round() as u32)
            };
            let info = format!(
                " {} [{}/{}]  ← → 切换  Ctrl+/- 缩放  Ctrl+0 重置  Esc/q 返回  {}",
                entry.filename,
                self.app.selected + 1,
                self.app.images.len(),
                zoom_str
            );
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test test_preview_status_bar 2>&1 | tail -10
```

Expected: `4 passed; 0 failed` (2 existing + 2 new).

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat: show zoom level in preview status bar"
```

---

## Task 3: Keyboard bindings

**Files:**
- Modify: `src/main.rs`

### Background

`handle_key()` is a plain function in `src/main.rs`. In Preview mode, we add three new match arms for `Ctrl++`/`Ctrl+=`, `Ctrl+-`, and `Ctrl+0`. On most terminals, pressing Ctrl++ sends `KeyCode::Char('+')` or `KeyCode::Char('=')` (since `+` requires Shift on most keyboards); we handle both to be robust.

- [ ] **Step 1: Write the failing tests**

`src/main.rs` currently has no `#[cfg(test)]` block. Add one at the very end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use crate::scanner::ImageEntry;
    use std::path::PathBuf;

    fn make_preview_app() -> App {
        let images = vec![ImageEntry {
            path: PathBuf::from("test.png"),
            filename: "test.png".to_string(),
            thumbnail: None,
        }];
        App::new(images, AppState::Preview)
    }

    #[test]
    fn ctrl_plus_zooms_in() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('+'), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor > before, "zoom_factor should increase");
    }

    #[test]
    fn ctrl_equals_also_zooms_in() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('='), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor > before, "Ctrl+= should also zoom in");
    }

    #[test]
    fn ctrl_minus_zooms_out() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('-'), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor < before, "zoom_factor should decrease");
    }

    #[test]
    fn ctrl_zero_resets_zoom() {
        let mut app = make_preview_app();
        app.zoom_factor = 3.0;
        handle_key(&mut app, KeyCode::Char('0'), KeyModifiers::CONTROL, 1);
        assert_eq!(app.zoom_factor, 1.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test ctrl_ 2>&1 | tail -15
```

Expected: FAIL — zoom keys are unhandled (fall through to `_ => {}`), so zoom_factor doesn't change.

- [ ] **Step 3: Add zoom key bindings in handle_key**

In `src/main.rs`, find the `AppState::Preview => match code {` block and add three new arms:

Old:
```rust
        AppState::Preview => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.exit_preview(),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => app.preview_prev(),
            KeyCode::Right => app.preview_next(),
            _ => {}
        },
```

New:
```rust
        AppState::Preview => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.exit_preview(),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => app.preview_prev(),
            KeyCode::Right => app.preview_next(),
            KeyCode::Char('+') | KeyCode::Char('=') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_in(),
            KeyCode::Char('-') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_out(),
            KeyCode::Char('0') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_reset(),
            _ => {}
        },
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test ctrl_ 2>&1 | tail -10
```

Expected: `4 passed; 0 failed`

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add Ctrl+/- zoom and Ctrl+0 reset key bindings"
```

---

## Task 4: Image preprocessing and protocol rebuild

**Files:**
- Modify: `src/main.rs`

### Background

Currently `run()` rebuilds the `StatefulProtocol` only when `app.selected` changes. We also rebuild when `app.zoom_factor` changes (tracked via `last_zoom_factor`). Before calling `picker.new_resize_protocol(img)`, we pre-process the image:

- `zoom == 1.0`: pass through unchanged
- `zoom > 1.0`: center-crop the image to `(w/zoom, h/zoom)` — the cropped region fills the full terminal area when fit-to-window renders it, producing the zoom-in effect
- `zoom < 1.0`: shrink the image then paste it centered on a black canvas sized to terminal pixel dimensions — produces the zoom-out effect with letterboxing

`picker.font_size` is the `(font_w, font_h): (u16, u16)` tuple stored on the `Picker` struct (public field in ratatui-image v2).

- [ ] **Step 1: Write the failing tests**

Inside the existing `#[cfg(test)] mod tests` block in `src/main.rs` (added in Task 3), add:

```rust
    #[test]
    fn preprocess_zoom_1_returns_same_dimensions() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        let result = preprocess_image_for_zoom(img, 1.0, (8, 12), 80, 24);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 80);
    }

    #[test]
    fn preprocess_zoom_in_crops_to_fraction() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        // zoom=2.0 → vis_w=50, vis_h=40
        let result = preprocess_image_for_zoom(img, 2.0, (8, 12), 80, 24);
        assert_eq!(result.width(), 50);
        assert_eq!(result.height(), 40);
    }

    #[test]
    fn preprocess_zoom_out_produces_canvas_size() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        // canvas = area_cols * font_w × area_rows * font_h = 40*8 × 12*12 = 320×144
        let result = preprocess_image_for_zoom(img, 0.5, (8, 12), 40, 12);
        assert_eq!(result.width(), 320);
        assert_eq!(result.height(), 144);
    }

    #[test]
    fn preprocess_zoom_in_is_centered_crop() {
        // Create 100x80 image where center pixel is red
        let mut img = image::RgbaImage::new(100, 80);
        img.put_pixel(50, 40, image::Rgba([255, 0, 0, 255]));
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        // zoom=2.0 → crop 50x40 centered at (25, 20)..(75, 60)
        // Center pixel (50,40) in original → (25,20) in cropped
        let result = preprocess_image_for_zoom(dyn_img, 2.0, (8, 12), 80, 24);
        let rgba = result.to_rgba8();
        assert_eq!(rgba.get_pixel(25, 20), &image::Rgba([255, 0, 0, 255]));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test preprocess_ 2>&1 | tail -10
```

Expected: compilation error — `preprocess_image_for_zoom` not defined yet.

- [ ] **Step 3: Add the preprocess_image_for_zoom function**

Add this function to `src/main.rs` (insert before `fn main()`):

```rust
fn preprocess_image_for_zoom(
    img: image::DynamicImage,
    zoom: f32,
    font_size: (u16, u16),
    area_cols: u16,
    area_rows: u16,
) -> image::DynamicImage {
    if zoom == 1.0 {
        return img;
    }

    if zoom > 1.0 {
        let vis_w = ((img.width() as f32 / zoom).round() as u32).max(1);
        let vis_h = ((img.height() as f32 / zoom).round() as u32).max(1);
        let x = img.width().saturating_sub(vis_w) / 2;
        let y = img.height().saturating_sub(vis_h) / 2;
        img.crop_imm(x, y, vis_w, vis_h)
    } else {
        use image::{DynamicImage, RgbaImage, imageops};
        let canvas_w = (area_cols as u32 * font_size.0 as u32).max(1);
        let canvas_h = (area_rows as u32 * font_size.1 as u32).max(1);
        let target_w = ((canvas_w as f32 * zoom).round() as u32).max(1);
        let target_h = ((canvas_h as f32 * zoom).round() as u32).max(1);
        let scaled = img.resize(target_w, target_h, imageops::FilterType::Lanczos3);
        let mut canvas = RgbaImage::from_pixel(canvas_w, canvas_h, image::Rgba([0, 0, 0, 255]));
        let paste_x = canvas_w.saturating_sub(scaled.width()) / 2;
        let paste_y = canvas_h.saturating_sub(scaled.height()) / 2;
        imageops::overlay(&mut canvas, &scaled.to_rgba8(), paste_x as i64, paste_y as i64);
        DynamicImage::ImageRgba8(canvas)
    }
}
```

Also add `use image;` at the top of `src/main.rs` if not already present (check the existing imports — it may not be there since the existing code uses `image::open()` via the full path).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test preprocess_ 2>&1 | tail -10
```

Expected: `4 passed; 0 failed`

- [ ] **Step 5: Update run() to track zoom and rebuild on zoom change**

In `src/main.rs`, find the `fn run(...)` function. Make two changes:

**Change A:** Add `last_zoom_factor` variable after `last_preview_index`:

Old:
```rust
    let mut last_preview_index: Option<usize> = None;
```

New:
```rust
    let mut last_preview_index: Option<usize> = None;
    let mut last_zoom_factor: f32 = 1.0;
```

**Change B:** Update the rebuild block to also trigger on zoom change and use `preprocess_image_for_zoom`:

Old:
```rust
        if app.state == AppState::Preview {
            if last_preview_index != Some(app.selected) {
                if let Some(entry) = app.images.get(app.selected) {
                    match image::open(&entry.path) {
                        Ok(img) => {
                            preview_state = Some(picker.new_resize_protocol(img));
                        }
                        Err(_) => {
                            preview_state = None;
                        }
                    }
                    last_preview_index = Some(app.selected);
                }
            }
        }
```

New:
```rust
        if app.state == AppState::Preview {
            if last_preview_index != Some(app.selected) || last_zoom_factor != app.zoom_factor {
                if let Some(entry) = app.images.get(app.selected) {
                    match image::open(&entry.path) {
                        Ok(img) => {
                            let (font_w, font_h) = picker.font_size;
                            let area_cols = size.width;
                            let area_rows = size.height.saturating_sub(1);
                            let processed = preprocess_image_for_zoom(
                                img,
                                app.zoom_factor,
                                (font_w, font_h),
                                area_cols,
                                area_rows,
                            );
                            preview_state = Some(picker.new_resize_protocol(processed));
                        }
                        Err(_) => {
                            preview_state = None;
                        }
                    }
                    last_preview_index = Some(app.selected);
                    last_zoom_factor = app.zoom_factor;
                }
            }
        }
```

- [ ] **Step 6: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests pass (previously ~46, now ~54).

- [ ] **Step 7: Build release binary**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: no errors, no warnings. (`picker.font_size` is a public field in ratatui-image v2 — if the compiler says it doesn't exist, try `picker.font_size()` as a method call instead.)

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat: preprocess image for zoom, rebuild protocol on zoom change"
```

---

## Verification Checklist

After all 4 tasks are committed:

- [ ] `cargo test` — all tests pass
- [ ] `cargo build --release` — clean build, no warnings
- [ ] `./target/release/darkroom` — open a folder with images, press Enter to preview, press `Ctrl++` several times (zoom shows `110%`, `120%`...), press `Ctrl+-`, press `Ctrl+0` to reset to `Fit`
- [ ] Navigate between images with `←→` while zoomed — zoom level is preserved
- [ ] `Ctrl+0` resets to Fit from any zoom level
- [ ] Status bar updates correctly at each zoom level
