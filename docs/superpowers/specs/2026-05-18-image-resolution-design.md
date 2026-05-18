# Image Resolution Improvement — Design Spec

**Date:** 2026-05-18
**Feature:** Higher quality thumbnail rendering and zoom-adaptive resolution

---

## Problem

Grid thumbnails currently look blocky/pixelated because:
1. `load_visible_thumbnails` pre-loads thumbnails at ~18×20 pixels using `FilterType::Nearest`
2. `render_thumbnail_to_buf` re-scales to display size using `FilterType::Nearest`

The `Nearest` filter (nearest-neighbor) produces aliased, pixelated output. Using a high-quality downsampling filter (`Lanczos3`) dramatically improves visual quality when reducing from a large image to a small display area.

For preview zoom, the rendering quality is already acceptable (Lanczos3 for zoom < 1.0, pixel-perfect crop for zoom > 1.0), but the approach works because we open the full-resolution image on every zoom change.

---

## Solution

### Fix 1: Increase thumbnail pre-load resolution (4×)

In `src/ui/mod.rs`, increase `thumb_w` and `thumb_h` by 4× before passing to `load_visible_thumbnails`:

**Before:**
```rust
let thumb_w = (crate::app::CELL_WIDTH as u32).saturating_sub(4);       // ~18px
let thumb_h = (crate::app::CELL_HEIGHT as u32).saturating_sub(4) * 2;  // ~20px
```

**After:**
```rust
let thumb_w = (crate::app::CELL_WIDTH as u32).saturating_sub(2) * 4;       // ~80px
let thumb_h = (crate::app::CELL_HEIGHT as u32).saturating_sub(3) * 2 * 4;  // ~88px
```

Rationale: The display pixel size is about 20×22 half-block pixels. Storing at 4× (80×88px) means `render_thumbnail_to_buf` can downsample from a rich source rather than upscaling from a tiny one. Downsampling with Lanczos3 gives dramatically better results than upscaling.

Memory cost: ~28KB per thumbnail vs ~1.5KB. For 500 cached thumbnails: ~14MB vs ~0.75MB — acceptable.

### Fix 2: Upgrade filter in load_visible_thumbnails

In `src/app.rs`, `load_visible_thumbnails`:

**Before:**
```rust
let thumb = img.resize(thumb_w, thumb_h, image::imageops::FilterType::Nearest);
```

**After:**
```rust
let thumb = img.thumbnail(thumb_w, thumb_h);
```

Use `thumbnail()` which internally uses `FilterType::Triangle` (fast bilinear), or use `resize` with `FilterType::Lanczos3` for best quality at the cost of slightly slower pre-loading. We use `Lanczos3`:

```rust
let thumb = img.resize(thumb_w, thumb_h, image::imageops::FilterType::Lanczos3);
```

### Fix 3: Upgrade filter in render_thumbnail_to_buf

In `src/ui/grid.rs`, `render_thumbnail_to_buf`:

**Before:**
```rust
let scaled = img.resize_exact(pixel_w, pixel_h, image::imageops::FilterType::Nearest);
```

**After:**
```rust
let scaled = img.resize_exact(pixel_w, pixel_h, image::imageops::FilterType::Lanczos3);
```

This is the most impactful change. Every frame render of the grid uses this path.

### Fix 4: Invalidate cached thumbnails when display size changes

Currently, thumbnails are cached with `if entry.thumbnail.is_none()`. If the thumb size parameters change (they're now computed from constants so they don't change at runtime), stale thumbnails won't be re-loaded. Since we're now computing `thumb_w/h` from constants (not terminal size), this is not an issue for this change — the values are deterministic and consistent.

No change needed for cache invalidation in this iteration.

---

## Zoom-Adaptive Resolution (Preview)

The preview zoom path already handles resolution correctly:
- `zoom > 1.0`: center-crop from full-resolution image (opened fresh via `image::open()`) → no quality loss
- `zoom < 1.0`: shrink with `Lanczos3` then center on black canvas → already high quality
- Protocol rebuild triggered on every zoom change → always uses fresh full-res source

No changes needed for preview zoom quality.

---

## Files Changed

| File | Change |
|------|--------|
| `src/ui/mod.rs` | Increase `thumb_w` / `thumb_h` by 4× |
| `src/app.rs` | `load_visible_thumbnails`: `Nearest` → `Lanczos3` |
| `src/ui/grid.rs` | `render_thumbnail_to_buf`: `Nearest` → `Lanczos3` |

---

## Performance Notes

- `Lanczos3` is ~5-10× slower than `Nearest` for the resize operation
- For `load_visible_thumbnails`: this runs lazily (only unloaded thumbnails), and thumbnails are cached after first load. One-time cost per image, not per frame.
- For `render_thumbnail_to_buf`: this runs every frame during grid rendering. However, downsampling 80×88 → 20×22 at Lanczos3 is still very fast (~0.1ms per thumbnail). With up to ~50 visible thumbnails, total: ~5ms per frame. Terminal frame rate is typically 20fps (50ms poll). This is acceptable.
- If profiling shows this is too slow, `Triangle` (bilinear) is the fallback — still much better than `Nearest` with ~2× the speed.

---

## Tests

- `test_render_thumbnail_to_buf_does_not_panic`: ensure it still works with Lanczos3 (smoke test)
- `test_render_thumbnail_to_buf_zero_area_no_panic`: unchanged
- No new tests needed — the filter change is a quality improvement, not a behavior change. Existing tests verify non-panic behavior.

---

## Out of Scope

- Per-frame dynamic resolution based on terminal font metrics
- Progressive thumbnail loading (load at low-res first, then high-res)
- Cache invalidation on terminal resize (thumbnails are fixed-constant-sized)
