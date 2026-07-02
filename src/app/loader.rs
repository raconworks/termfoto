use super::*;

mod animation;
mod control;
mod process;
mod types;
mod worker;

pub use control::LoadControl;
pub use types::{
    AnimationContent, AnimationFrame, FullscreenContent, ImageCacheKey, LoadRequest, LoadResult,
    LoadSize, OriginalLoadKind, StaticContent,
};
pub use worker::spawn_image_loader;

pub(in crate::app) use animation::animation_frame_estimated_bytes;
#[cfg(test)]
pub(in crate::app) use animation::{
    animation_content_from_frames, animation_frames_estimated_bytes, frame_delay,
    static_original_content, try_decode_animation,
};
#[cfg(all(feature = "bench-internals", not(test)))]
pub(in crate::app) use process::process_thumbnail_request;
pub(in crate::app) use process::{
    load_content_is_terminal, process_load_request_with_control_to_sender,
};
#[cfg(test)]
pub(in crate::app) use process::{
    process_load_request_with_control, process_original_request, process_thumbnail_request,
    OriginalRequestParts,
};
pub(in crate::app) use types::LoadContent;
