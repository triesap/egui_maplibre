#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageFrameMeta {
    pub width_px: u32,
    pub height_px: u32,
}

#[cfg(feature = "image")]
#[must_use]
pub fn from_native_image(image: &maplibre_native::Image) -> (ImageFrameMeta, Vec<u8>) {
    let buffer = image.as_image();
    (
        ImageFrameMeta {
            width_px: buffer.width(),
            height_px: buffer.height(),
        },
        buffer.as_raw().to_vec(),
    )
}

#[must_use]
pub fn min_size(width_px: u32, height_px: u32) -> ImageFrameMeta {
    ImageFrameMeta {
        width_px: width_px.max(1),
        height_px: height_px.max(1),
    }
}
