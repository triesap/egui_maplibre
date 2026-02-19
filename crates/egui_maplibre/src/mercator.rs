use std::f64::consts::PI;

use crate::types::Camera;

pub const TILE_SIZE: f64 = 256.0;
pub const MIN_LAT: f64 = -85.051_128_78;
pub const MAX_LAT: f64 = 85.051_128_78;

#[must_use]
pub fn clamp_lat(lat: f64) -> f64 {
    lat.clamp(MIN_LAT, MAX_LAT)
}

#[must_use]
pub fn world_size(zoom: f64) -> f64 {
    TILE_SIZE * 2.0_f64.powf(zoom)
}

#[must_use]
pub fn lon_lat_to_world(lng: f64, lat: f64, zoom: f64) -> (f64, f64) {
    let lat = clamp_lat(lat);
    let scale = world_size(zoom);
    let x = (lng + 180.0) / 360.0 * scale;
    let lat_rad = lat.to_radians();
    let merc = (lat_rad.tan() + (1.0 / lat_rad.cos())).ln();
    let y = (1.0 - merc / PI) / 2.0 * scale;
    (x, y)
}

#[must_use]
pub fn world_to_lon_lat(x: f64, y: f64, zoom: f64) -> (f64, f64) {
    let scale = world_size(zoom);
    let lon = ((x / scale * 360.0 - 180.0) + 540.0).rem_euclid(360.0) - 180.0;
    let t = PI * (1.0 - 2.0 * y / scale);
    let lat = t.sinh().atan().to_degrees();
    (lon, clamp_lat(lat))
}

#[must_use]
pub fn pan_camera(camera: Camera, dx_px: f64, dy_px: f64) -> Camera {
    let (center_x, center_y) = lon_lat_to_world(camera.lng, camera.lat, camera.zoom);
    let next_x = center_x - dx_px;
    let next_y = center_y - dy_px;
    let (next_lng, next_lat) = world_to_lon_lat(next_x, next_y, camera.zoom);
    Camera {
        lng: next_lng,
        lat: next_lat,
        ..camera
    }
}

#[must_use]
pub fn zoom_camera_about_point(
    camera: Camera,
    target_zoom: f64,
    width_px: f64,
    height_px: f64,
    cursor_x_px: f64,
    cursor_y_px: f64,
) -> Camera {
    if width_px <= 0.0 || height_px <= 0.0 {
        return Camera {
            zoom: target_zoom,
            ..camera
        };
    }
    let (center_x0, center_y0) = lon_lat_to_world(camera.lng, camera.lat, camera.zoom);
    let point_x0 = center_x0 + (cursor_x_px - width_px / 2.0);
    let point_y0 = center_y0 + (cursor_y_px - height_px / 2.0);
    let scale = 2.0_f64.powf(target_zoom - camera.zoom);
    let point_x1 = point_x0 * scale;
    let point_y1 = point_y0 * scale;
    let center_x1 = point_x1 - (cursor_x_px - width_px / 2.0);
    let center_y1 = point_y1 - (cursor_y_px - height_px / 2.0);
    let (next_lng, next_lat) = world_to_lon_lat(center_x1, center_y1, target_zoom);
    Camera {
        lng: next_lng,
        lat: next_lat,
        zoom: target_zoom,
        ..camera
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Camera;

    use super::{lon_lat_to_world, pan_camera, world_to_lon_lat, zoom_camera_about_point};

    const EPS: f64 = 1e-7;

    #[test]
    fn mercator_forward_inverse_roundtrip() {
        let samples = [
            (0.0, 0.0, 1.5),
            (18.06, 59.33, 7.25),
            (-74.006, 40.7128, 9.0),
            (151.2093, -33.8688, 4.5),
        ];

        for (lng, lat, zoom) in samples {
            let (x, y) = lon_lat_to_world(lng, lat, zoom);
            let (lng_back, lat_back) = world_to_lon_lat(x, y, zoom);
            assert!((lng_back - lng).abs() < EPS, "lng mismatch");
            assert!((lat_back - lat).abs() < EPS, "lat mismatch");
        }
    }

    #[test]
    fn mercator_pan_moves_center_by_pixel_delta() {
        let camera = Camera::new(0.0, 0.0, 3.0, 0.0, 0.0);
        let panned = pan_camera(camera, 128.0, 64.0);
        let (x0, y0) = lon_lat_to_world(camera.lng, camera.lat, camera.zoom);
        let (x1, y1) = lon_lat_to_world(panned.lng, panned.lat, panned.zoom);
        assert!((x1 - (x0 - 128.0)).abs() < EPS);
        assert!((y1 - (y0 - 64.0)).abs() < EPS);
    }

    #[test]
    fn cursor_anchor_zoom_keeps_world_point_under_cursor() {
        let camera = Camera::new(18.06, 59.33, 6.0, 0.0, 0.0);
        let width = 1200.0;
        let height = 800.0;
        let cursor_x = 910.0;
        let cursor_y = 120.0;
        let target_zoom = 8.0;

        let (center_x0, center_y0) = lon_lat_to_world(camera.lng, camera.lat, camera.zoom);
        let point_x0 = center_x0 + (cursor_x - width / 2.0);
        let point_y0 = center_y0 + (cursor_y - height / 2.0);
        let (lng0, lat0) = world_to_lon_lat(point_x0, point_y0, camera.zoom);

        let next = zoom_camera_about_point(camera, target_zoom, width, height, cursor_x, cursor_y);
        let (center_x1, center_y1) = lon_lat_to_world(next.lng, next.lat, next.zoom);
        let point_x1 = center_x1 + (cursor_x - width / 2.0);
        let point_y1 = center_y1 + (cursor_y - height / 2.0);
        let (lng1, lat1) = world_to_lon_lat(point_x1, point_y1, next.zoom);

        assert!((lng1 - lng0).abs() < EPS);
        assert!((lat1 - lat0).abs() < EPS);
    }
}
