use super::*;

pub(super) struct ZoomRenderGeometry {
    pub(super) source_x: f64,
    pub(super) source_y: f64,
    pub(super) source_w: f64,
    pub(super) source_h: f64,
    pub(super) target_px_w: u32,
    pub(super) target_px_h: u32,
}

pub(super) struct ZoomDisplayGeometry {
    pub(super) scale: f64,
    pub(super) display_px_w: f64,
    pub(super) display_px_h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum RenderQuality {
    Interactive,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenderDirtyReason {
    Interaction,
    ContentOrViewport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct RenderKey {
    pub(super) image_key: ImageCacheKey,
    pub(super) viewport_w: u16,
    pub(super) viewport_h: u16,
    pub(super) font_w: u16,
    pub(super) font_h: u16,
    pub(super) zoom_percent: u16,
    pub(super) pan_x: i16,
    pub(super) pan_y: i16,
    pub(super) quality: RenderQuality,
}

impl RenderKey {
    pub(super) fn same_view(&self, other: &Self) -> bool {
        self.image_key == other.image_key
            && self.viewport_w == other.viewport_w
            && self.viewport_h == other.viewport_h
            && self.font_w == other.font_w
            && self.font_h == other.font_h
            && self.zoom_percent == other.zoom_percent
            && self.pan_x == other.pan_x
            && self.pan_y == other.pan_y
    }
}

pub(super) struct RenderRequest {
    pub(super) image_key: ImageCacheKey,
    pub(super) image: Arc<image::RgbaImage>,
    pub(super) viewport: Size,
    pub(super) font_size: FontSize,
    pub(super) zoom: f32,
    pub(super) pan_x: i16,
    pub(super) pan_y: i16,
    pub(super) key: RenderKey,
    pub(super) generation: u64,
}

pub(super) struct RenderResult {
    pub(super) image_key: ImageCacheKey,
    pub(super) protocol: Protocol,
    pub(super) key: RenderKey,
    pub(super) generation: u64,
}

pub(super) fn zoom_render_geometry(
    img_w: u32,
    img_h: u32,
    viewport_px_w: u32,
    viewport_px_h: u32,
    zoom: f32,
    pan_px_x: i32,
    pan_px_y: i32,
) -> ZoomRenderGeometry {
    let img_w = img_w.max(1);
    let img_h = img_h.max(1);
    let viewport_px_w = viewport_px_w.max(1);
    let viewport_px_h = viewport_px_h.max(1);
    let display = zoom_display_geometry(img_w, img_h, viewport_px_w, viewport_px_h, zoom);

    let visible_display_w = display.display_px_w.min(f64::from(viewport_px_w));
    let visible_display_h = display.display_px_h.min(f64::from(viewport_px_h));
    let target_px_w = rounded_px(visible_display_w);
    let target_px_h = rounded_px(visible_display_h);

    let max_display_x = (display.display_px_w - f64::from(viewport_px_w)).max(0.0);
    let max_display_y = (display.display_px_h - f64::from(viewport_px_h)).max(0.0);
    let display_x = if max_display_x > 0.0 {
        (max_display_x / 2.0 + f64::from(pan_px_x)).clamp(0.0, max_display_x)
    } else {
        0.0
    };
    let display_y = if max_display_y > 0.0 {
        (max_display_y / 2.0 + f64::from(pan_px_y)).clamp(0.0, max_display_y)
    } else {
        0.0
    };

    let source_w = (visible_display_w / display.scale).clamp(1.0, f64::from(img_w));
    let source_h = (visible_display_h / display.scale).clamp(1.0, f64::from(img_h));
    let max_source_x = (f64::from(img_w) - source_w).max(0.0);
    let max_source_y = (f64::from(img_h) - source_h).max(0.0);
    let source_x = (display_x / display.scale).clamp(0.0, max_source_x);
    let source_y = (display_y / display.scale).clamp(0.0, max_source_y);

    ZoomRenderGeometry {
        source_x,
        source_y,
        source_w,
        source_h,
        target_px_w,
        target_px_h,
    }
}

pub(super) fn zoom_display_geometry(
    img_w: u32,
    img_h: u32,
    viewport_px_w: u32,
    viewport_px_h: u32,
    zoom: f32,
) -> ZoomDisplayGeometry {
    let img_w = img_w.max(1);
    let img_h = img_h.max(1);
    let viewport_px_w = viewport_px_w.max(1);
    let viewport_px_h = viewport_px_h.max(1);
    let zoom = normalized_zoom(zoom);
    let fit_scale = (f64::from(viewport_px_w) / f64::from(img_w))
        .min(f64::from(viewport_px_h) / f64::from(img_h))
        .max(f64::EPSILON);
    let scale = fit_scale * f64::from(zoom);

    ZoomDisplayGeometry {
        scale,
        display_px_w: (f64::from(img_w) * scale).max(1.0),
        display_px_h: (f64::from(img_h) * scale).max(1.0),
    }
}

pub(super) fn normalized_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() {
        zoom.clamp(ZOOM_MIN, ZOOM_MAX)
    } else {
        ZOOM_MIN
    }
}

fn rounded_px(value: f64) -> u32 {
    value.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

pub(super) fn max_pan_cells(display_px: f64, viewport_px: u32, font_px: u16) -> i16 {
    let overflow = display_px - f64::from(viewport_px.max(1));
    if overflow <= 0.0 {
        return 0;
    }
    let cells = (overflow / 2.0 / f64::from(font_px.max(1))).ceil();
    cells.clamp(0.0, f64::from(i16::MAX)) as i16
}

pub(super) fn zoom_percent(zoom: f32) -> u16 {
    let zoom = normalized_zoom(zoom);
    (zoom * 100.0).round().clamp(1.0, u16::MAX as f32) as u16
}

pub(super) fn spawn_render_worker(
    picker: Picker,
) -> (Sender<RenderRequest>, Receiver<RenderResult>) {
    let (render_tx, render_rx) = std::sync::mpsc::channel::<RenderRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<RenderResult>();

    std::thread::spawn(move || {
        let mut resizer = fir::Resizer::new();
        while let Ok(mut request) = render_rx.recv() {
            while let Ok(next) = render_rx.try_recv() {
                request = next;
            }
            if let Some(protocol) = render_zoom_protocol(&picker, &mut resizer, &request) {
                let _ = done_tx.send(RenderResult {
                    image_key: request.image_key,
                    protocol,
                    key: request.key,
                    generation: request.generation,
                });
            }
        }
    });

    (render_tx, done_rx)
}

fn render_zoom_protocol(
    picker: &Picker,
    resizer: &mut fir::Resizer,
    request: &RenderRequest,
) -> Option<Protocol> {
    let viewport_px_w =
        (request.viewport.width as u32).saturating_mul(request.font_size.width as u32);
    let viewport_px_h =
        (request.viewport.height as u32).saturating_mul(request.font_size.height as u32);
    let pan_px_x = (request.pan_x as f32 * request.font_size.width as f32) as i32;
    let pan_px_y = (request.pan_y as f32 * request.font_size.height as f32) as i32;
    let geometry = zoom_render_geometry(
        request.image.width(),
        request.image.height(),
        viewport_px_w,
        viewport_px_h,
        request.zoom,
        pan_px_x,
        pan_px_y,
    );

    let resized =
        resize_with_fast_image_resize(resizer, &request.image, &geometry, request.key.quality)
            .unwrap_or_else(|| {
                resize_with_image_crate(&request.image, &geometry, request.key.quality)
            });
    let resized_img = image::DynamicImage::ImageRgba8(resized);
    picker
        .new_protocol(resized_img, request.viewport, Resize::Fit(None))
        .ok()
}

fn resize_with_fast_image_resize(
    resizer: &mut fir::Resizer,
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
    quality: RenderQuality,
) -> Option<image::RgbaImage> {
    let mut dst = image::RgbaImage::new(geometry.target_px_w, geometry.target_px_h);
    let algorithm = match quality {
        RenderQuality::Interactive => fir::ResizeAlg::Nearest,
        RenderQuality::Final => fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3),
    };
    let options = fir::ResizeOptions::new().resize_alg(algorithm).crop(
        geometry.source_x,
        geometry.source_y,
        geometry.source_w,
        geometry.source_h,
    );

    resizer.resize(image, &mut dst, Some(&options)).ok()?;
    Some(dst)
}

fn resize_with_image_crate(
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
    quality: RenderQuality,
) -> image::RgbaImage {
    let filter = match quality {
        RenderQuality::Interactive => image::imageops::FilterType::Nearest,
        RenderQuality::Final => image::imageops::FilterType::Lanczos3,
    };
    let (source_x, source_y, source_w, source_h) = integer_source_rect(image, geometry);
    let cropped =
        image::imageops::crop_imm(image, source_x, source_y, source_w, source_h).to_image();
    image::imageops::resize(&cropped, geometry.target_px_w, geometry.target_px_h, filter)
}

fn integer_source_rect(
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
) -> (u32, u32, u32, u32) {
    let img_w = image.width().max(1);
    let img_h = image.height().max(1);
    let source_x = geometry
        .source_x
        .floor()
        .clamp(0.0, f64::from(img_w.saturating_sub(1))) as u32;
    let source_y = geometry
        .source_y
        .floor()
        .clamp(0.0, f64::from(img_h.saturating_sub(1))) as u32;
    let max_w = img_w.saturating_sub(source_x).max(1);
    let max_h = img_h.saturating_sub(source_y).max(1);
    let source_w = rounded_px(geometry.source_w).min(max_w).max(1);
    let source_h = rounded_px(geometry.source_h).min(max_h).max(1);

    (source_x, source_y, source_w, source_h)
}
