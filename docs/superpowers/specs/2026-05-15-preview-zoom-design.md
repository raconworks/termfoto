# darkroom Preview Zoom — Design Spec

**Date:** 2026-05-15  
**Feature:** Interactive zoom in/out in preview mode via keyboard shortcuts

---

## Overview

Add zoom support to darkroom's preview mode. Users can magnify or shrink the displayed image using Ctrl++ / Ctrl+- in 10% steps, reset to fit-window with Ctrl+0, and the zoom level persists when navigating between images.

---

## Keyboard Bindings (Preview Mode)

| Key | Action |
|-----|--------|
| `Ctrl++` or `Ctrl+=` | Zoom in 10% |
| `Ctrl+-` | Zoom out 10% |
| `Ctrl+0` | Reset to fit-window (zoom = 1.0) |

All other preview bindings remain unchanged (`q`/`Esc` back to grid, `Ctrl-C` quit, `←→` prev/next image).

---

## State

### New field in `App`

```rust
pub zoom_factor: f32  // default: 1.0
```

**Semantics:**
- `1.0` = fit to terminal window (current default behavior)
- `2.0` = 2× zoom (center-crop shows 50% of image width/height)
- `0.5` = 0.5× zoom (image rendered at half window size, centered on black background)

**Range:** `0.1 – 5.0` (inclusive), step `0.1`

**Precision:** All zoom operations round to 1 decimal place to avoid floating-point drift (e.g., `0.1 + 0.2 ≠ 0.3`):
```rust
self.zoom_factor = ((self.zoom_factor + 0.1) * 10.0).round() / 10.0;
```

### New methods on `App`

```rust
pub fn zoom_in(&mut self)    // zoom_factor += 0.1, capped at 5.0
pub fn zoom_out(&mut self)   // zoom_factor -= 0.1, floored at 0.1
pub fn zoom_reset(&mut self) // zoom_factor = 1.0
```

### Zoom persists across images

`zoom_factor` is not reset when navigating between images (`preview_prev`, `preview_next`). The user resets explicitly with `Ctrl+0`.

---

## Rendering

### Protocol rebuild trigger (`run()` in `main.rs`)

Rebuild the ratatui-image `StatefulProtocol` when either of these changes:
- `app.selected` (image changed)
- `app.zoom_factor` (zoom changed)

Track via `last_preview_index: Option<usize>` and `last_zoom_factor: f32` (default `1.0`).

### Image pre-processing (before `picker.new_resize_protocol`)

Pre-process the `DynamicImage` based on `zoom_factor`. This ensures correct centering across all rendering backends (Kitty, Sixel, Unicode half-block).

**Input values needed:**
- `picker.font_size()` → `(font_w: u16, font_h: u16)` — pixel dimensions of one terminal cell
- Terminal area → `(terminal_width, terminal_height - 1)` character cells (minus 1 row for status bar)
- Terminal pixel area → `(area_w * font_w, area_h * font_h)`

**zoom == 1.0** — No preprocessing:
```
picker.new_resize_protocol(img)   // Resize::Fit(None) in widget
```

**zoom > 1.0** — Center-crop (magnify):
```
visible_w = img.width()  / zoom_factor
visible_h = img.height() / zoom_factor
x = (img.width()  - visible_w) / 2
y = (img.height() - visible_h) / 2
cropped = img.crop_imm(x, y, visible_w, visible_h)
picker.new_resize_protocol(cropped)   // Resize::Fit(None) fills terminal
```

**zoom < 1.0** — Shrink and center on black canvas:
```
scaled_w = (term_pixel_w as f32 * zoom_factor) as u32
scaled_h = (term_pixel_h as f32 * zoom_factor) as u32
scaled   = img.resize(scaled_w, scaled_h, FilterType::Lanczos3)

canvas   = RgbaImage::new(term_pixel_w, term_pixel_h)  // black (0,0,0,255)
paste_x  = (term_pixel_w - scaled.width())  / 2
paste_y  = (term_pixel_h - scaled.height()) / 2
overlay(&mut canvas, &scaled.to_rgba8(), paste_x, paste_h)

picker.new_resize_protocol(DynamicImage::ImageRgba8(canvas))
```

`image::imageops::{overlay, FilterType}` are already available via the `image` crate.

---

## Status Bar

Append zoom status to the existing status bar in `src/ui/preview.rs`.

**Format:** `filename  [3/12]  Fit` or `filename  [3/12]  200%`

```rust
let zoom_str = if app.zoom_factor == 1.0 {
    "Fit".to_string()
} else {
    format!("{}%", (app.zoom_factor * 100.0).round() as u32)
};
// status line: "{filename}  [{idx}/{total}]  {zoom_str}"
```

`PreviewView` receives `zoom_factor: f32` as a constructor parameter (same pattern as existing parameters).

---

## Files Changed

| File | Change |
|------|--------|
| `src/app.rs` | Add `zoom_factor` field, `zoom_in/out/reset()` methods, tests |
| `src/main.rs` | Add `last_zoom_factor`; rebuild logic; `preprocess_image()` helper |
| `src/ui/preview.rs` | Accept `zoom_factor`; update status bar rendering |
| `src/ui/mod.rs` | Pass `app.zoom_factor` to `PreviewView` |

No changes to `Cargo.toml` — all required image operations (`crop_imm`, `resize`, `overlay`) are already available in the `image = "0.25"` dependency.

---

## Tests

New unit tests in `src/app.rs`:

```rust
#[test]
fn zoom_in_increments_by_0_1() { ... }       // 1.0 → 1.1

#[test]
fn zoom_in_capped_at_5() { ... }             // 5.0 + step = 5.0

#[test]
fn zoom_out_decrements_by_0_1() { ... }      // 1.0 → 0.9

#[test]
fn zoom_out_floored_at_0_1() { ... }         // 0.1 - step = 0.1

#[test]
fn zoom_reset_returns_to_1() { ... }         // any → 1.0

#[test]
fn zoom_no_float_drift() { ... }             // 0.1 * 3 steps = 0.4 exactly
```

Status bar zoom display is covered by existing `PreviewView` render tests (extend them).

---

## Out of Scope

- Panning (image always centered)
- Per-image zoom memory (single zoom level for the session)
- Smooth/animated zoom transitions
- Mouse scroll zoom
