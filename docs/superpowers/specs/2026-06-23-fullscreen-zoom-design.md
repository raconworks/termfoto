# 全屏图片缩放功能设计

**日期**: 2026-06-23
**状态**: 已确认

## 概述

全屏模式新增图片缩放与平移功能，弥补当前只能查看适配视口尺寸图片的局限。

## 状态变化

### `App` 新增字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `zoom` | `f32` | `1.0` | 缩放倍率，1.0 = 适配视口 |
| `pan_x` | `i16` | `0` | 水平平移偏移（终端列），正=右移 |
| `pan_y` | `i16` | `0` | 垂直平移偏移（终端行），正=下移 |

### 切换图片重置

`fullscreen_prev()` / `fullscreen_next()` 中重置 `zoom=1.0, pan_x=0, pan_y=0`。

### `FullscreenContent` 变更

`Static` 变体改为同时缓存原始 `DynamicImage`：

```rust
Static {
    protocol: Protocol,       // 当前缩放级别的协议
    original: DynamicImage,   // 原始解码图像，用于重新缩放
}
```

`Animation` 变体不变，动图不支持缩放。

## 按键绑定（全屏模式新增）

| 键 | 功能 | 说明 |
|---|---|---|
| `+` / `=` | 放大 | zoom × 1.25，上限 10× |
| `-` | 缩小 | zoom × 0.8，下限 0.25× |
| `0` | 重置 | zoom = 1.0, pan = (0, 0) |
| `h` | 左平移 | pan_x -= 平移步长 |
| `l` | 右平移 | pan_x += 平移步长 |
| `j` | 下平移 | pan_y += 平移步长 |
| `k` | 上平移 | pan_y -= 平移步长 |

平移步长 = 视口宽度/高度的 10%（≥ 1 列/行）。

## 协议重新生成

缩放变更时的流程：

```
zoom_changed(zoom) →
  viewport_size = 当前图像区域大小(终端行列)
  target_size  = Size(viewport_size.w * zoom, viewport_size.h * zoom)
  protocol     = picker.new_protocol(cached_original, target_size, Resize::Fit)
  pan_x, pan_y = 重新钳制到新图片边界
  更新 Static { protocol, original }
```

此操作在主线程同步执行，`picker.new_protocol()` 纯内存缩放开销很小（< 1ms），不会阻塞渲染。

## 渲染变更

`PreviewView::render()` 中：

```
proto_size = protocol.size()
// 计算居中偏移，加上平移偏移
offset_x = (image_area.w - proto_size.w) / 2 - pan_x
offset_y = (image_area.h - proto_size.h) / 2 - pan_y
// 渲染在 image_area 内，allow_clipping(true) 裁剪超界部分
```

平移边界钳制：`pan_x` 范围为 `[-half_image_w, half_image_w]`，`pan_y` 类似，确保不会移动到图片之外。

## 状态栏

缩放不为 1.0 时状态栏追加 `[N%]` 文字（如 `[125%]`），`Loading...` 时隐含重置不显示。通过 `preview_status()` 的 `status` 参数传入。

`Lang` 新增 `zoom_status()` 方法：中 `[N%]`，英 `[N%]`。

## 约束

- 缩放范围：0.25× ~ 10×
- 步长：1.25×（放大）/ 0.8×（缩小）
- 动图：忽略缩放操作（全屏检测到 `Animation` 变体时，缩放按键无效果）
- 平移仅当 `zoom > 1.0` 时有效（1.0× 时图片小于等于视口，无需平移）

## 测试

- 缩放倍率边界（0.25 下限，10 上限）
- 切换图片重置 zoom/pan
- 平移边界钳制
- 动图忽略缩放
- 状态栏显示百分比
