use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use egui::{
    Color32, ColorImage, Pos2, Rect, Response, Sense, TextureHandle, TextureOptions, Ui, Vec2,
};
use instant::Instant;

use crate::camera::CameraState;
use crate::errors::RenderError;
use crate::mercator::{lon_lat_to_world, world_to_lon_lat};
use crate::types::{
    Camera, MapEvent, MapEventKind, MapHandle, MapInitOptions, MapRuntimeOptions, StyleSource,
    TileServerOptions,
};
use crate::util::image_convert::min_size;
use crate::util::rate_limit::RateLimitState;
use crate::worker::image_worker::ImageWorkerState;
use crate::worker::protocol::{RenderReason, WorkerCmd};

static NEXT_MAP_HANDLE: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug, PartialEq)]
struct WorkerConfigSnapshot {
    width_px: u32,
    height_px: u32,
    pixel_ratio: f32,
    tile: TileServerOptions,
}

pub struct EguiMapLibre {
    handle: MapHandle,
    style: StyleSource,
    camera_state: CameraState,
    interactive: bool,
    runtime: MapRuntimeOptions,
    worker: Option<ImageWorkerState>,
    last_worker_config: Option<WorkerConfigSnapshot>,
    texture: Option<TextureHandle>,
    texture_name: String,
    last_error: Option<RenderError>,
    drag_limiter: RateLimitState,
    dragging: bool,
    last_drag_delta: Vec2,
    pending_release_settle: bool,
    last_reported_camera: Camera,
}

impl EguiMapLibre {
    pub fn new(options: MapInitOptions) -> Self {
        let handle = MapHandle(NEXT_MAP_HANDLE.fetch_add(1, Ordering::Relaxed));
        let style =
            StyleSource::from_style_url_or_path(options.style_url.as_str()).unwrap_or_else(|_| {
                if let Ok(url) = "https://demotiles.maplibre.org/style.json".parse() {
                    StyleSource::Url(url)
                } else {
                    StyleSource::Path(PathBuf::from("style.json"))
                }
            });
        let camera = options.camera();
        let min_zoom = options.min_zoom.unwrap_or(0.0);
        let max_zoom = options.max_zoom.unwrap_or(22.0);
        let runtime = MapRuntimeOptions::default();

        Self {
            handle,
            style,
            camera_state: CameraState::new(camera, min_zoom, max_zoom),
            interactive: options.interactive.unwrap_or(true),
            drag_limiter: RateLimitState::new(runtime.quality.drag_max_fps),
            runtime,
            worker: None,
            last_worker_config: None,
            texture: None,
            texture_name: format!("egui_maplibre_{}", handle.0),
            last_error: None,
            dragging: false,
            last_drag_delta: Vec2::ZERO,
            pending_release_settle: false,
            last_reported_camera: camera,
        }
    }

    pub fn map_handle(&self) -> MapHandle {
        self.handle
    }

    pub fn camera(&self) -> Camera {
        self.camera_state.camera()
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera_state.set_camera(camera);
        self.push_camera_to_worker();
        self.request_render(RenderReason::Programmatic);
    }

    pub fn set_style(&mut self, style: StyleSource) {
        self.style = style.clone();
        self.send_worker(WorkerCmd::SetStyle(style));
        self.request_render(RenderReason::Programmatic);
    }

    pub fn set_style_url(&mut self, style_url: &str) -> Result<(), RenderError> {
        let style = StyleSource::from_style_url_or_path(style_url)?;
        self.set_style(style);
        Ok(())
    }

    pub fn fly_to(&mut self, lng: f64, lat: f64, zoom: Option<f64>, _duration_ms: Option<u32>) {
        let mut camera = self.camera();
        camera.lng = lng;
        camera.lat = lat;
        if let Some(zoom) = zoom {
            camera.zoom = zoom;
        }
        self.set_camera(camera);
    }

    pub fn add_geojson_source(
        &mut self,
        source_id: &str,
        geojson: &serde_json::Value,
        promote_id: Option<&str>,
    ) -> Result<(), RenderError> {
        let _ = source_id;
        let _ = geojson;
        let _ = promote_id;
        Err(RenderError::UnsupportedOperation(
            "geojson sources are not available",
        ))
    }

    pub fn update_geojson_source(
        &mut self,
        source_id: &str,
        geojson: &serde_json::Value,
    ) -> Result<(), RenderError> {
        let _ = source_id;
        let _ = geojson;
        Err(RenderError::UnsupportedOperation(
            "geojson sources are not available",
        ))
    }

    pub fn remove_source(&mut self, source_id: &str) -> Result<(), RenderError> {
        let _ = source_id;
        Err(RenderError::UnsupportedOperation(
            "sources are not available",
        ))
    }

    pub fn add_layer(
        &mut self,
        layer_id: &str,
        layer_spec: &serde_json::Value,
        before_id: Option<&str>,
    ) -> Result<(), RenderError> {
        let _ = layer_id;
        let _ = layer_spec;
        let _ = before_id;
        Err(RenderError::UnsupportedOperation(
            "layers are not available",
        ))
    }

    pub fn remove_layer(&mut self, layer_id: &str) -> Result<(), RenderError> {
        let _ = layer_id;
        Err(RenderError::UnsupportedOperation(
            "layers are not available",
        ))
    }

    pub fn set_feature_state(
        &mut self,
        source_id: &str,
        source_layer: Option<&str>,
        feature_id: &serde_json::Value,
        state: &serde_json::Value,
    ) -> Result<(), RenderError> {
        let _ = source_id;
        let _ = source_layer;
        let _ = feature_id;
        let _ = state;
        Err(RenderError::UnsupportedOperation(
            "feature state is not available",
        ))
    }

    pub fn ui_fill(&mut self, ui: &mut Ui) -> (Response, Vec<MapEvent>) {
        self.ui(ui, ui.available_size_before_wrap())
    }

    pub fn ui(&mut self, ui: &mut Ui, desired_size: Vec2) -> (Response, Vec<MapEvent>) {
        let desired_size = Vec2::new(desired_size.x.max(1.0), desired_size.y.max(1.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        self.ensure_worker();
        self.apply_worker_frames(ui);

        let pixels_per_point = ui.ctx().pixels_per_point();
        let mut camera_changed = false;
        let mut event_kind = MapEventKind::Move;

        if self.interactive {
            if response.drag_started() {
                self.dragging = true;
                self.last_drag_delta = Vec2::ZERO;
                self.drag_limiter.reset();
            }

            if response.dragged() {
                let drag_delta = response.drag_delta();
                let frame_delta = drag_delta - self.last_drag_delta;
                self.last_drag_delta = drag_delta;
                if frame_delta != Vec2::ZERO {
                    let (shift, ctrl) = ui.input(|input| {
                        (
                            input.modifiers.shift,
                            input.modifiers.ctrl || input.modifiers.command,
                        )
                    });
                    if shift {
                        self.camera_state.rotate_by(f64::from(frame_delta.x) * 0.2);
                    } else if ctrl {
                        self.camera_state.pitch_by(-f64::from(frame_delta.y) * 0.2);
                    } else {
                        self.camera_state.pan_pixels(
                            f64::from(frame_delta.x * pixels_per_point),
                            f64::from(frame_delta.y * pixels_per_point),
                        );
                    }
                    camera_changed = true;
                    event_kind = MapEventKind::Move;
                }
            }

            if response.hovered() {
                let (zoom_delta, scroll_y, hover_pos) = ui.input(|input| {
                    (
                        input.zoom_delta(),
                        input.smooth_scroll_delta.y + input.raw_scroll_delta.y,
                        input.pointer.hover_pos(),
                    )
                });
                let zoom_from_gesture = (zoom_delta - 1.0).abs() > f32::EPSILON;
                let zoom_from_scroll = scroll_y.abs() > f32::EPSILON;
                if zoom_from_gesture || zoom_from_scroll {
                    let zoom_offset = if zoom_from_gesture {
                        f64::from(zoom_delta.log2())
                    } else {
                        f64::from(scroll_y) * 0.01
                    };
                    let target_zoom = self.camera().zoom + zoom_offset;
                    let cursor = hover_pos
                        .map(|pos| pos - rect.min)
                        .unwrap_or(Vec2::new(rect.width() * 0.5, rect.height() * 0.5));
                    self.camera_state.zoom_about_point(
                        target_zoom,
                        f64::from(rect.width() * pixels_per_point),
                        f64::from(rect.height() * pixels_per_point),
                        f64::from(cursor.x * pixels_per_point),
                        f64::from(cursor.y * pixels_per_point),
                    );
                    camera_changed = true;
                    event_kind = MapEventKind::Zoom;
                }
            }

            if response.drag_stopped() {
                self.dragging = false;
                self.last_drag_delta = Vec2::ZERO;
                if self.runtime.quality.render_full_res_on_release {
                    self.pending_release_settle = true;
                }
            }
        }

        if camera_changed {
            self.push_camera_to_worker();
            if self.dragging {
                let now = Instant::now();
                self.configure_worker(
                    rect,
                    pixels_per_point,
                    self.runtime.quality.drag_resolution_scale,
                );
                if self.drag_limiter.allow_now(now) {
                    self.request_render(RenderReason::Interaction);
                }
                ui.ctx().request_repaint();
            } else {
                self.configure_worker(rect, pixels_per_point, 1.0);
                self.request_render(RenderReason::Programmatic);
            }
        } else {
            if self.dragging {
                self.configure_worker(
                    rect,
                    pixels_per_point,
                    self.runtime.quality.drag_resolution_scale,
                );
                ui.ctx().request_repaint();
            } else {
                self.configure_worker(rect, pixels_per_point, 1.0);
            }
        }

        if self.pending_release_settle {
            self.pending_release_settle = false;
            self.push_camera_to_worker();
            self.configure_worker(rect, pixels_per_point, 1.0);
            self.request_render(RenderReason::ReleaseSettle);
            ui.ctx().request_repaint();
            event_kind = MapEventKind::Idle;
        }

        if self.runtime.quality.continuous_idle_render && !self.dragging {
            self.push_camera_to_worker();
            self.request_render(RenderReason::Programmatic);
            ui.ctx().request_repaint();
        }

        self.paint(ui, rect);

        let mut events = Vec::new();
        let camera = self.camera();
        if camera != self.last_reported_camera {
            events.push(MapEvent {
                kind: event_kind,
                view: camera.into(),
            });
            self.last_reported_camera = camera;
        }

        (response, events)
    }

    pub fn last_error(&self) -> Option<&RenderError> {
        self.last_error.as_ref()
    }

    fn ensure_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let worker = ImageWorkerState::spawn(self.camera());
        self.worker = Some(worker);
        self.send_worker(WorkerCmd::SetStyle(self.style.clone()));
        self.push_camera_to_worker();
        self.request_render(RenderReason::Initial);
    }

    fn configure_worker(&mut self, rect: Rect, pixels_per_point: f32, scale: f32) {
        let scale = scale.clamp(0.1, 1.0);
        let size = min_size(
            (rect.width() * pixels_per_point * scale).round() as u32,
            (rect.height() * pixels_per_point * scale).round() as u32,
        );
        let snapshot = WorkerConfigSnapshot {
            width_px: size.width_px,
            height_px: size.height_px,
            pixel_ratio: (pixels_per_point * scale).max(0.1),
            tile: self.runtime.tile.clone(),
        };
        if self.last_worker_config.as_ref() == Some(&snapshot) {
            return;
        }
        self.last_worker_config = Some(snapshot.clone());
        self.send_worker(WorkerCmd::Configure {
            width_px: snapshot.width_px,
            height_px: snapshot.height_px,
            pixel_ratio: snapshot.pixel_ratio,
            tile: snapshot.tile,
        });
    }

    fn push_camera_to_worker(&mut self) {
        self.send_worker(WorkerCmd::SetCamera(self.camera()));
    }

    fn request_render(&mut self, reason: RenderReason) {
        self.send_worker(WorkerCmd::RenderNow(reason));
    }

    fn send_worker(&mut self, cmd: WorkerCmd) {
        let Some(worker) = self.worker.as_ref() else {
            return;
        };
        if let Err(error) = worker.send(cmd) {
            self.last_error = Some(error);
        }
    }

    fn apply_worker_frames(&mut self, ui: &Ui) {
        let Some(worker) = self.worker.as_ref() else {
            return;
        };
        let Some(frame) = worker.drain_latest() else {
            return;
        };
        match frame {
            Ok(frame) => {
                self.last_error = None;
                self.camera_state.set_camera(frame.camera);
                let image = ColorImage::from_rgba_unmultiplied(
                    [frame.width_px as usize, frame.height_px as usize],
                    &frame.rgba,
                );
                if let Some(texture) = self.texture.as_mut() {
                    texture.set(image, TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ui.ctx().load_texture(
                        self.texture_name.clone(),
                        image,
                        TextureOptions::LINEAR,
                    ));
                }
            }
            Err(error) => {
                self.last_error = Some(error);
            }
        }
    }

    fn paint(&self, ui: &Ui, rect: Rect) {
        if let Some(texture) = self.texture.as_ref() {
            ui.painter().image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(rect, 0.0, Color32::from_gray(20));
        }
    }

    pub fn click_event_at(
        &self,
        rect: Rect,
        pointer_pos: Pos2,
        pixels_per_point: f32,
    ) -> (f64, f64) {
        let camera = self.camera();
        let (center_x, center_y) = lon_lat_to_world(camera.lng, camera.lat, camera.zoom);
        let offset_x = f64::from((pointer_pos.x - rect.min.x) * pixels_per_point)
            - f64::from(rect.width() * pixels_per_point) * 0.5;
        let offset_y = f64::from((pointer_pos.y - rect.min.y) * pixels_per_point)
            - f64::from(rect.height() * pixels_per_point) * 0.5;
        world_to_lon_lat(center_x + offset_x, center_y + offset_y, camera.zoom)
    }
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, Rect, Vec2};

    use crate::types::MapInitOptions;

    use super::EguiMapLibre;

    #[test]
    fn widget_initializes_from_options() {
        let map = EguiMapLibre::new(MapInitOptions::default_demo());
        let camera = map.camera();
        assert_eq!(camera.lng, 0.0);
        assert_eq!(camera.lat, 0.0);
        assert_eq!(camera.zoom, 1.5);
    }

    #[test]
    fn fly_to_updates_camera() {
        let mut map = EguiMapLibre::new(MapInitOptions::default_demo());
        map.fly_to(11.2, 58.9, Some(6.0), Some(500));
        let camera = map.camera();
        assert!((camera.lng - 11.2).abs() < 1e-10);
        assert!((camera.lat - 58.9).abs() < 1e-10);
        assert!((camera.zoom - 6.0).abs() < 1e-10);
    }

    #[test]
    fn invalid_style_url_is_rejected() {
        let mut map = EguiMapLibre::new(MapInitOptions::default_demo());
        let error = map
            .set_style_url("")
            .expect_err("invalid style should fail");
        assert!(matches!(error, crate::RenderError::InvalidStyle));
    }

    #[test]
    fn click_coordinate_projection_returns_world_coords() {
        let map = EguiMapLibre::new(MapInitOptions::default_demo());
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1000.0, 600.0));
        let (lng, lat) = map.click_event_at(rect, Pos2::new(500.0, 300.0), 1.0);
        assert!((lng - map.camera().lng).abs() < 1e-7);
        assert!((lat - map.camera().lat).abs() < 1e-7);
    }
}
