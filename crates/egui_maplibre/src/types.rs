use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub lng: f64,
    pub lat: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}

impl Camera {
    pub const fn new(lng: f64, lat: f64, zoom: f64, bearing: f64, pitch: f64) -> Self {
        Self {
            lng,
            lat,
            zoom,
            bearing,
            pitch,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(0.0, 0.0, 1.5, 0.0, 0.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapHandle(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapControlAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeControlOptions {
    pub navigation: Option<MapControlAnchor>,
    pub scale: Option<MapControlAnchor>,
    pub fullscreen: Option<MapControlAnchor>,
    pub geolocate: Option<MapControlAnchor>,
    pub attribution: Option<MapControlAnchor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MapInitOptions {
    pub style_url: String,
    pub center_lng: f64,
    pub center_lat: f64,
    pub zoom: f64,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub bounds: Option<[f64; 4]>,
    pub pitch: Option<f64>,
    pub bearing: Option<f64>,
    pub interactive: Option<bool>,
    pub attribution_control: Option<bool>,
    pub antialias: Option<bool>,
    pub native_controls: Option<NativeControlOptions>,
}

impl MapInitOptions {
    pub fn default_demo() -> Self {
        Self {
            style_url: "https://demotiles.maplibre.org/style.json".to_owned(),
            center_lng: 0.0,
            center_lat: 0.0,
            zoom: 1.5,
            min_zoom: Some(0.0),
            max_zoom: Some(22.0),
            bounds: None,
            pitch: Some(0.0),
            bearing: Some(0.0),
            interactive: Some(true),
            attribution_control: Some(false),
            antialias: Some(false),
            native_controls: None,
        }
    }

    pub fn camera(&self) -> Camera {
        Camera::new(
            self.center_lng,
            self.center_lat,
            self.zoom,
            self.bearing.unwrap_or(0.0),
            self.pitch.unwrap_or(0.0),
        )
    }
}

impl Default for MapInitOptions {
    fn default() -> Self {
        Self::default_demo()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualityPolicy {
    pub drag_resolution_scale: f32,
    pub drag_max_fps: f32,
    pub render_full_res_on_release: bool,
    pub continuous_idle_render: bool,
}

impl QualityPolicy {
    pub fn validate(&self) -> bool {
        self.drag_resolution_scale > 0.0
            && self.drag_resolution_scale <= 1.0
            && self.drag_max_fps > 0.0
    }
}

impl Default for QualityPolicy {
    fn default() -> Self {
        Self {
            drag_resolution_scale: 0.5,
            drag_max_fps: 20.0,
            render_full_res_on_release: true,
            continuous_idle_render: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StyleSource {
    Url(Url),
    Path(PathBuf),
}

impl StyleSource {
    pub fn from_style_url_or_path(style_url: &str) -> Result<Self, crate::errors::RenderError> {
        if let Ok(url) = Url::parse(style_url) {
            return Ok(Self::Url(url));
        }
        let path = PathBuf::from(style_url);
        if path.as_os_str().is_empty() {
            return Err(crate::errors::RenderError::InvalidStyle);
        }
        Ok(Self::Path(path))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TileServerOptions {
    pub size: (u32, u32),
    pub pixel_ratio: f32,
    pub cache_path: Option<PathBuf>,
    pub asset_root: Option<PathBuf>,
    pub base_url: Option<Url>,
    pub uri_scheme_alias: Option<String>,
    pub source_template: Option<String>,
    pub style_template: Option<String>,
    pub sprites_template: Option<String>,
    pub glyphs_template: Option<String>,
    pub tile_template: Option<String>,
    pub api_key_parameter_name: Option<String>,
    pub api_key: Option<String>,
    pub requires_api_key: bool,
}

impl Default for TileServerOptions {
    fn default() -> Self {
        Self {
            size: (512, 512),
            pixel_ratio: 1.0,
            cache_path: None,
            asset_root: None,
            base_url: None,
            uri_scheme_alias: None,
            source_template: None,
            style_template: None,
            sprites_template: None,
            glyphs_template: None,
            tile_template: None,
            api_key_parameter_name: None,
            api_key: None,
            requires_api_key: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeatureHit {
    pub layer_id: String,
    pub properties: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MapClickEvent {
    pub lng: f64,
    pub lat: f64,
    pub screen_x: f64,
    pub screen_y: f64,
    pub features: Vec<FeatureHit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapEventKind {
    Move,
    Zoom,
    Idle,
    StyleData,
    Data,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapViewState {
    pub center_lng: f64,
    pub center_lat: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}

impl From<Camera> for MapViewState {
    fn from(value: Camera) -> Self {
        Self {
            center_lng: value.lng,
            center_lat: value.lat,
            zoom: value.zoom,
            bearing: value.bearing,
            pitch: value.pitch,
        }
    }
}

impl From<MapViewState> for Camera {
    fn from(value: MapViewState) -> Self {
        Self {
            lng: value.center_lng,
            lat: value.center_lat,
            zoom: value.zoom,
            bearing: value.bearing,
            pitch: value.pitch,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapEvent {
    pub kind: MapEventKind,
    pub view: MapViewState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapRuntimeOptions {
    pub quality: QualityPolicy,
    pub tile: TileServerOptions,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Camera, FeatureHit, MapClickEvent, MapControlAnchor, MapEvent, MapEventKind,
        MapInitOptions, MapRuntimeOptions, MapViewState, NativeControlOptions,
    };

    #[test]
    fn map_init_options_roundtrip() {
        let options = MapInitOptions {
            style_url: "https://demotiles.maplibre.org/style.json".to_string(),
            center_lng: 15.0,
            center_lat: 60.0,
            zoom: 6.0,
            min_zoom: Some(2.0),
            max_zoom: Some(18.0),
            bounds: Some([-179.0, -80.0, 179.0, 80.0]),
            pitch: Some(35.0),
            bearing: Some(15.0),
            interactive: Some(true),
            attribution_control: Some(false),
            antialias: Some(true),
            native_controls: Some(NativeControlOptions {
                navigation: Some(MapControlAnchor::TopRight),
                scale: Some(MapControlAnchor::BottomLeft),
                fullscreen: None,
                geolocate: None,
                attribution: Some(MapControlAnchor::BottomRight),
            }),
        };

        let encoded = serde_json::to_string(&options).expect("serialize map options");
        let decoded: MapInitOptions =
            serde_json::from_str(&encoded).expect("deserialize map options");
        assert_eq!(decoded, options);
    }

    #[test]
    fn map_click_event_roundtrip() {
        let event = MapClickEvent {
            lng: 11.2,
            lat: 58.9,
            screen_x: 256.0,
            screen_y: 384.0,
            features: vec![FeatureHit {
                layer_id: "lots-fill".to_string(),
                properties: json!({"lot":"SE-123"}),
            }],
        };

        let encoded = serde_json::to_string(&event).expect("serialize click");
        let decoded: MapClickEvent = serde_json::from_str(&encoded).expect("deserialize click");
        assert_eq!(decoded, event);
    }

    #[test]
    fn map_event_roundtrip() {
        let event = MapEvent {
            kind: MapEventKind::Move,
            view: MapViewState {
                center_lng: 18.06,
                center_lat: 59.33,
                zoom: 8.5,
                bearing: 15.0,
                pitch: 30.0,
            },
        };

        let encoded = serde_json::to_string(&event).expect("serialize map event");
        let decoded: MapEvent = serde_json::from_str(&encoded).expect("deserialize map event");
        assert_eq!(decoded, event);
    }

    #[test]
    fn camera_conversion_roundtrip() {
        let camera = Camera::new(18.06, 59.33, 8.5, 15.0, 30.0);
        let state: MapViewState = camera.into();
        let camera_back: Camera = state.into();
        assert_eq!(camera_back, camera);
    }

    #[test]
    fn map_runtime_options_defaults_are_valid() {
        let options = MapRuntimeOptions::default();
        assert!(options.quality.validate());
    }

    #[test]
    fn map_init_options_default_demo_camera() {
        let options = MapInitOptions::default_demo();
        let camera = options.camera();
        assert_eq!(camera.lng, 0.0);
        assert_eq!(camera.lat, 0.0);
        assert_eq!(camera.zoom, 1.5);
        assert_eq!(camera.bearing, 0.0);
        assert_eq!(camera.pitch, 0.0);
    }
}
