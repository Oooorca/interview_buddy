use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, ColorType};

use super::model::{CaptureError, CaptureResult, RegionSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScaledBounds {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn encode_capture(image: image::RgbaImage) -> Result<CaptureResult, CaptureError> {
    let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 82)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    Ok(CaptureResult {
        data_url: format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg)),
    })
}

pub(super) fn scaled_selection_bounds(
    selection: &RegionSelection,
    scale: f64,
    image_size: Option<(u32, u32)>,
) -> Result<ScaledBounds, CaptureError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(CaptureError::InvalidScale);
    }
    let mut left = (selection.x * scale).round() as i32;
    let mut top = (selection.y * scale).round() as i32;
    let mut right = ((selection.x + selection.width) * scale).round() as i32;
    let mut bottom = ((selection.y + selection.height) * scale).round() as i32;
    if let Some((image_width, image_height)) = image_size {
        let max_x = image_width.min(i32::MAX as u32) as i32;
        let max_y = image_height.min(i32::MAX as u32) as i32;
        left = left.clamp(0, max_x);
        top = top.clamp(0, max_y);
        right = right.clamp(0, max_x);
        bottom = bottom.clamp(0, max_y);
    }
    let width = left.abs_diff(right);
    let height = top.abs_diff(bottom);
    if width < 2 || height < 2 {
        return Err(CaptureError::OutOfBounds);
    }
    Ok(ScaledBounds {
        left: left.min(right),
        top: top.min(bottom),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retina_selection_is_scaled_once_into_capture_pixels() {
        let selection = RegionSelection {
            x: 100.0,
            y: 50.0,
            width: 400.0,
            height: 300.0,
        };
        assert_eq!(
            scaled_selection_bounds(&selection, 2.0, Some((3456, 2234))).unwrap(),
            ScaledBounds {
                left: 200,
                top: 100,
                width: 800,
                height: 600,
            }
        );
    }

    #[test]
    fn scaled_selection_is_clamped_to_captured_monitor() {
        let selection = RegionSelection {
            x: 1600.0,
            y: 1000.0,
            width: 300.0,
            height: 300.0,
        };
        assert_eq!(
            scaled_selection_bounds(&selection, 2.0, Some((3456, 2234))).unwrap(),
            ScaledBounds {
                left: 3200,
                top: 2000,
                width: 256,
                height: 234,
            }
        );
    }
}
