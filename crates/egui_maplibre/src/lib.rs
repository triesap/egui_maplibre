#![forbid(unsafe_code)]

pub mod camera;
pub mod errors;
pub mod mercator;
pub mod types;
pub mod util;
pub mod widget;
pub mod worker;

pub use errors::RenderError;
pub use types::{
    Camera, FeatureHit, MapClickEvent, MapControlAnchor, MapEvent, MapEventKind, MapHandle,
    MapInitOptions, MapRuntimeOptions, MapViewState, NativeControlOptions, QualityPolicy,
    StyleSource, TileServerOptions,
};
pub use widget::EguiMapLibre;

pub fn set_style(map: &mut EguiMapLibre, style_url: &str) -> Result<(), RenderError> {
    map.set_style_url(style_url)
}

pub fn fly_to(
    map: &mut EguiMapLibre,
    lng: f64,
    lat: f64,
    zoom: Option<f64>,
    duration_ms: Option<u32>,
) {
    map.fly_to(lng, lat, zoom, duration_ms);
}

pub fn add_geojson_source(
    map: &mut EguiMapLibre,
    source_id: &str,
    geojson: &serde_json::Value,
    promote_id: Option<&str>,
) -> Result<(), RenderError> {
    map.add_geojson_source(source_id, geojson, promote_id)
}

pub fn update_geojson_source(
    map: &mut EguiMapLibre,
    source_id: &str,
    geojson: &serde_json::Value,
) -> Result<(), RenderError> {
    map.update_geojson_source(source_id, geojson)
}

pub fn remove_source(map: &mut EguiMapLibre, source_id: &str) -> Result<(), RenderError> {
    map.remove_source(source_id)
}

pub fn add_layer(
    map: &mut EguiMapLibre,
    layer_id: &str,
    layer_spec: &serde_json::Value,
    before_id: Option<&str>,
) -> Result<(), RenderError> {
    map.add_layer(layer_id, layer_spec, before_id)
}

pub fn remove_layer(map: &mut EguiMapLibre, layer_id: &str) -> Result<(), RenderError> {
    map.remove_layer(layer_id)
}

pub fn set_feature_state(
    map: &mut EguiMapLibre,
    source_id: &str,
    source_layer: Option<&str>,
    feature_id: &serde_json::Value,
    state: &serde_json::Value,
) -> Result<(), RenderError> {
    map.set_feature_state(source_id, source_layer, feature_id, state)
}

#[cfg(test)]
mod tests {
    use super::{EguiMapLibre, MapInitOptions, fly_to, set_style};

    #[test]
    fn wrapper_fly_to_updates_map_camera() {
        let mut map = EguiMapLibre::new(MapInitOptions::default_demo());
        fly_to(&mut map, 13.4, 52.5, Some(6.5), Some(250));
        let camera = map.camera();
        assert!((camera.lng - 13.4).abs() < 1e-10);
        assert!((camera.lat - 52.5).abs() < 1e-10);
        assert!((camera.zoom - 6.5).abs() < 1e-10);
    }

    #[test]
    fn wrapper_set_style_rejects_invalid_input() {
        let mut map = EguiMapLibre::new(MapInitOptions::default_demo());
        let error = set_style(&mut map, "").expect_err("empty style should fail");
        assert!(matches!(error, crate::RenderError::InvalidStyle));
    }
}
