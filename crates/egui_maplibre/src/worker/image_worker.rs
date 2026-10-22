use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(feature = "image")]
use super::protocol::RenderedFrame;
use crate::errors::RenderError;
use crate::types::Camera;
#[cfg(feature = "image")]
use crate::types::{StyleSource, TileServerOptions};
#[cfg(feature = "image")]
use crate::util::image_convert::{from_native_image, min_size};

use super::protocol::{WorkerCmd, WorkerFrameResult};

pub struct ImageWorkerState {
    cmd_tx: Sender<WorkerCmd>,
    frame_rx: Receiver<WorkerFrameResult>,
    join_handle: Option<JoinHandle<()>>,
}

impl ImageWorkerState {
    pub fn spawn(initial_camera: Camera) -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (frame_tx, frame_rx) = unbounded();
        let join_handle = thread::Builder::new()
            .name("egui_maplibre_image_worker".to_string())
            .spawn(move || run_worker(initial_camera, cmd_rx, frame_tx))
            .ok();
        Self {
            cmd_tx,
            frame_rx,
            join_handle,
        }
    }

    pub fn send(&self, cmd: WorkerCmd) -> Result<(), RenderError> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| RenderError::WorkerChannelClosed)
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<WorkerFrameResult> {
        self.frame_rx.try_recv().ok()
    }

    #[must_use]
    pub fn drain_latest(&self) -> Option<WorkerFrameResult> {
        let mut latest = self.try_recv();
        while let Some(frame) = self.try_recv() {
            latest = Some(frame);
        }
        latest
    }
}

impl Drop for ImageWorkerState {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(WorkerCmd::Shutdown);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

#[cfg(feature = "image")]
#[derive(Debug)]
struct WorkerRuntime {
    camera: Camera,
    style: Option<StyleSource>,
    tile: TileServerOptions,
    width_px: u32,
    height_px: u32,
    pixel_ratio: f32,
    renderer: Option<maplibre_native::ImageRenderer<maplibre_native::Static>>,
}

#[cfg(feature = "image")]
impl WorkerRuntime {
    fn new(initial_camera: Camera) -> Self {
        let tile = TileServerOptions::default();
        Self {
            camera: initial_camera,
            style: None,
            width_px: tile.size.0.max(1),
            height_px: tile.size.1.max(1),
            pixel_ratio: tile.pixel_ratio.max(0.1),
            tile,
            renderer: None,
        }
    }

    fn configure(
        &mut self,
        width_px: u32,
        height_px: u32,
        pixel_ratio: f32,
        tile: TileServerOptions,
    ) -> Result<(), RenderError> {
        let size = min_size(width_px, height_px);
        let pixel_ratio = pixel_ratio.max(0.1);
        let rebuild = self.renderer.is_none()
            || self.width_px != size.width_px
            || self.height_px != size.height_px
            || (self.pixel_ratio - pixel_ratio).abs() > f32::EPSILON
            || self.tile != tile;

        self.width_px = size.width_px;
        self.height_px = size.height_px;
        self.pixel_ratio = pixel_ratio;
        self.tile = tile;

        if rebuild {
            self.rebuild_renderer()?;
        }
        Ok(())
    }

    fn set_style(&mut self, style: StyleSource) -> Result<(), RenderError> {
        self.style = Some(style.clone());
        if let Some(renderer) = self.renderer.as_mut() {
            load_style(renderer, &style)?;
        }
        Ok(())
    }

    fn render(
        &mut self,
        reason: super::protocol::RenderReason,
    ) -> Result<RenderedFrame, RenderError> {
        let Some(renderer) = self.renderer.as_mut() else {
            return Err(RenderError::RendererNotInitialized);
        };
        let image = renderer.render_static(
            self.camera.lat,
            self.camera.lng,
            self.camera.zoom,
            self.camera.bearing,
            self.camera.pitch,
        )?;
        let (meta, rgba) = from_native_image(&image);
        Ok(RenderedFrame {
            width_px: meta.width_px,
            height_px: meta.height_px,
            rgba,
            camera: self.camera,
            reason,
        })
    }

    fn rebuild_renderer(&mut self) -> Result<(), RenderError> {
        let width_px =
            std::num::NonZeroU32::new(self.width_px).ok_or(RenderError::RendererNotInitialized)?;
        let height_px =
            std::num::NonZeroU32::new(self.height_px).ok_or(RenderError::RendererNotInitialized)?;

        let mut builder = maplibre_native::ImageRendererBuilder::new()
            .with_size(width_px, height_px)
            .with_pixel_ratio(self.pixel_ratio);

        builder = apply_tile_options(builder, &self.tile);

        let mut renderer = builder.build_static_renderer();
        if let Some(style) = self.style.as_ref() {
            load_style(&mut renderer, style)?;
        }
        self.renderer = Some(renderer);
        Ok(())
    }
}

#[cfg(feature = "image")]
fn apply_tile_options(
    mut builder: maplibre_native::ImageRendererBuilder,
    tile: &TileServerOptions,
) -> maplibre_native::ImageRendererBuilder {
    if let Some(cache_path) = tile.cache_path.as_ref() {
        builder = builder.with_cache_path(cache_path);
    }
    if let Some(asset_root) = tile.asset_root.as_ref() {
        builder = builder.with_asset_root(asset_root);
    }
    if let Some(base_url) = tile.base_url.as_ref() {
        builder = builder.with_base_url(base_url.clone());
    }
    if let Some(uri_scheme_alias) = tile.uri_scheme_alias.as_ref() {
        builder = builder.with_uri_scheme_alias(uri_scheme_alias);
    }
    if let Some(source_template) = tile.source_template.as_ref() {
        builder = builder.with_source_template(source_template);
    }
    if let Some(style_template) = tile.style_template.as_ref() {
        builder = builder.with_style_template(style_template);
    }
    if let Some(sprites_template) = tile.sprites_template.as_ref() {
        builder = builder.with_sprites_template(sprites_template);
    }
    if let Some(glyphs_template) = tile.glyphs_template.as_ref() {
        builder = builder.with_glyphs_template(glyphs_template);
    }
    if let Some(tile_template) = tile.tile_template.as_ref() {
        builder = builder.with_tile_template(tile_template);
    }
    if let Some(api_key_parameter_name) = tile.api_key_parameter_name.as_ref() {
        builder = builder.with_api_key_parameter_name(api_key_parameter_name);
    }
    if let Some(api_key) = tile.api_key.as_ref() {
        builder = builder.with_api_key(api_key);
    }
    builder.set_requires_api_key(tile.requires_api_key)
}

#[cfg(feature = "image")]
fn load_style(
    renderer: &mut maplibre_native::ImageRenderer<maplibre_native::Static>,
    style: &StyleSource,
) -> Result<(), RenderError> {
    match style {
        StyleSource::Url(url) => {
            renderer.load_style_from_url(url);
            Ok(())
        }
        StyleSource::Path(path) => renderer
            .load_style_from_path(path)
            .map(|_| ())
            .map_err(|_| RenderError::InvalidStyle),
    }
}

#[cfg(feature = "image")]
fn run_worker(
    initial_camera: Camera,
    cmd_rx: Receiver<WorkerCmd>,
    frame_tx: Sender<WorkerFrameResult>,
) {
    let mut runtime = WorkerRuntime::new(initial_camera);

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            WorkerCmd::Configure {
                width_px,
                height_px,
                pixel_ratio,
                tile,
            } => {
                if let Err(error) = runtime.configure(width_px, height_px, pixel_ratio, tile) {
                    let _ = frame_tx.send(Err(error));
                }
            }
            WorkerCmd::SetStyle(style) => {
                if let Err(error) = runtime.set_style(style) {
                    let _ = frame_tx.send(Err(error));
                }
            }
            WorkerCmd::SetCamera(camera) => {
                runtime.camera = camera;
            }
            WorkerCmd::RenderNow(reason) => {
                let _ = frame_tx.send(runtime.render(reason));
            }
            WorkerCmd::Shutdown => break,
        }
    }
}

#[cfg(not(feature = "image"))]
fn run_worker(
    _initial_camera: Camera,
    cmd_rx: Receiver<WorkerCmd>,
    frame_tx: Sender<WorkerFrameResult>,
) {
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            WorkerCmd::Shutdown => break,
            WorkerCmd::SetCamera(_) => {}
            WorkerCmd::Configure { .. } | WorkerCmd::SetStyle(_) | WorkerCmd::RenderNow(_) => {
                let _ = frame_tx.send(Err(RenderError::UnsupportedOperation(
                    "image feature is disabled",
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageWorkerState, WorkerCmd};
    use crate::types::Camera;
    #[cfg(not(feature = "image"))]
    use crate::worker::protocol::RenderReason;
    #[cfg(not(feature = "image"))]
    use std::time::Duration;

    #[test]
    fn worker_accepts_shutdown() {
        let worker = ImageWorkerState::spawn(Camera::default());
        assert!(worker.send(WorkerCmd::Shutdown).is_ok());
    }

    #[cfg(not(feature = "image"))]
    #[test]
    fn worker_reports_disabled_image_feature() {
        let worker = ImageWorkerState::spawn(Camera::default());
        assert!(
            worker
                .send(WorkerCmd::RenderNow(RenderReason::Initial))
                .is_ok()
        );
        std::thread::sleep(Duration::from_millis(20));
        let frame = worker.try_recv().expect("expected worker frame");
        assert!(frame.is_err());
    }
}
