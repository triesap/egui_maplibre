use crate::mercator::{MAX_LAT, MIN_LAT, pan_camera, zoom_camera_about_point};
use crate::types::Camera;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraState {
    camera: Camera,
    min_zoom: f64,
    max_zoom: f64,
    min_pitch: f64,
    max_pitch: f64,
}

impl CameraState {
    pub fn new(camera: Camera, min_zoom: f64, max_zoom: f64) -> Self {
        let (min_zoom, max_zoom) = normalize_zoom_limits(min_zoom, max_zoom);
        let mut state = Self {
            camera,
            min_zoom,
            max_zoom,
            min_pitch: 0.0,
            max_pitch: 60.0,
        };
        state.camera = state.constrain(state.camera);
        state
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = self.constrain(camera);
    }

    pub fn set_zoom_limits(&mut self, min_zoom: f64, max_zoom: f64) {
        let (min_zoom, max_zoom) = normalize_zoom_limits(min_zoom, max_zoom);
        self.min_zoom = min_zoom;
        self.max_zoom = max_zoom;
        self.camera = self.constrain(self.camera);
    }

    pub fn set_pitch_limits(&mut self, min_pitch: f64, max_pitch: f64) {
        self.min_pitch = min_pitch.min(max_pitch);
        self.max_pitch = min_pitch.max(max_pitch);
        self.camera = self.constrain(self.camera);
    }

    pub fn pan_pixels(&mut self, dx_px: f64, dy_px: f64) {
        self.camera = self.constrain(pan_camera(self.camera, dx_px, dy_px));
    }

    pub fn zoom_about_point(
        &mut self,
        target_zoom: f64,
        width_px: f64,
        height_px: f64,
        cursor_x_px: f64,
        cursor_y_px: f64,
    ) {
        let target_zoom = target_zoom.clamp(self.min_zoom, self.max_zoom);
        self.camera = self.constrain(zoom_camera_about_point(
            self.camera,
            target_zoom,
            width_px,
            height_px,
            cursor_x_px,
            cursor_y_px,
        ));
    }

    pub fn rotate_by(&mut self, delta_degrees: f64) {
        self.camera.bearing = normalize_bearing(self.camera.bearing + delta_degrees);
    }

    pub fn pitch_by(&mut self, delta_degrees: f64) {
        self.camera.pitch =
            (self.camera.pitch + delta_degrees).clamp(self.min_pitch, self.max_pitch);
    }

    pub fn constrain_camera(&self, camera: Camera) -> Camera {
        self.constrain(camera)
    }

    fn constrain(&self, camera: Camera) -> Camera {
        Camera {
            lng: normalize_lng(camera.lng),
            lat: camera.lat.clamp(MIN_LAT, MAX_LAT),
            zoom: camera.zoom.clamp(self.min_zoom, self.max_zoom),
            bearing: normalize_bearing(camera.bearing),
            pitch: camera.pitch.clamp(self.min_pitch, self.max_pitch),
        }
    }
}

impl Default for CameraState {
    fn default() -> Self {
        Self::new(Camera::default(), 0.0, 22.0)
    }
}

fn normalize_lng(lng: f64) -> f64 {
    (lng + 180.0).rem_euclid(360.0) - 180.0
}

fn normalize_bearing(bearing: f64) -> f64 {
    let normalized = (bearing + 180.0).rem_euclid(360.0) - 180.0;
    if normalized == -180.0 {
        180.0
    } else {
        normalized
    }
}

fn normalize_zoom_limits(min_zoom: f64, max_zoom: f64) -> (f64, f64) {
    (min_zoom.min(max_zoom), min_zoom.max(max_zoom))
}

#[cfg(test)]
mod tests {
    use crate::types::Camera;

    use super::CameraState;

    #[test]
    fn camera_state_clamps_bounds() {
        let mut state = CameraState::default();
        state.set_camera(Camera::new(361.0, 90.0, 42.0, 725.0, 85.0));
        let camera = state.camera();
        assert!(camera.lng <= 180.0);
        assert!(camera.lng >= -180.0);
        assert!(camera.lat <= 85.05112878);
        assert!(camera.lat >= -85.05112878);
        assert_eq!(camera.zoom, 22.0);
        assert_eq!(camera.pitch, 60.0);
    }

    #[test]
    fn camera_state_pan_updates_center() {
        let mut state = CameraState::new(Camera::new(0.0, 0.0, 4.0, 0.0, 0.0), 0.0, 22.0);
        let before = state.camera();
        state.pan_pixels(256.0, 0.0);
        let after = state.camera();
        assert_ne!(before.lng, after.lng);
        assert_eq!(before.zoom, after.zoom);
    }

    #[test]
    fn camera_state_zoom_about_point_respects_limits() {
        let mut state = CameraState::new(Camera::new(18.06, 59.33, 6.0, 0.0, 0.0), 1.0, 10.0);
        state.zoom_about_point(20.0, 1200.0, 800.0, 600.0, 400.0);
        assert_eq!(state.camera().zoom, 10.0);
    }
}
