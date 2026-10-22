use crate::errors::RenderError;
use crate::types::{Camera, StyleSource, TileServerOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderReason {
    #[default]
    Initial,
    Interaction,
    ReleaseSettle,
    Programmatic,
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "Keep the public owned-command API without allocating each configuration message"
)]
pub enum WorkerCmd {
    Configure {
        width_px: u32,
        height_px: u32,
        pixel_ratio: f32,
        tile: TileServerOptions,
    },
    SetStyle(StyleSource),
    SetCamera(Camera),
    RenderNow(RenderReason),
    Shutdown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedFrame {
    pub width_px: u32,
    pub height_px: u32,
    pub rgba: Vec<u8>,
    pub camera: Camera,
    pub reason: RenderReason,
}

pub type WorkerFrameResult = Result<RenderedFrame, RenderError>;
