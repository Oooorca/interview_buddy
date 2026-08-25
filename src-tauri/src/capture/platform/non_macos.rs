use xcap::Monitor;

use super::super::{
    crop::encode_capture,
    model::{CaptureError, CaptureResult},
};

pub(super) fn capture_absolute_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<CaptureResult, CaptureError> {
    if width < 2 || height < 2 {
        return Err(CaptureError::TooSmall);
    }
    let monitor =
        Monitor::from_point(x, y).map_err(|error| CaptureError::Operation(error.to_string()))?;
    let monitor_x = monitor
        .x()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let monitor_y = monitor
        .y()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let full = monitor
        .capture_image()
        .map_err(|error| CaptureError::Operation(error.to_string()))?;
    let max_x = full.width() as i32;
    let max_y = full.height() as i32;
    let left = (x - monitor_x).clamp(0, max_x);
    let top = (y - monitor_y).clamp(0, max_y);
    let right = (x.saturating_add(width as i32) - monitor_x).clamp(0, max_x);
    let bottom = (y.saturating_add(height as i32) - monitor_y).clamp(0, max_y);
    let cropped_width = left.abs_diff(right);
    let cropped_height = top.abs_diff(bottom);
    if cropped_width < 2 || cropped_height < 2 {
        return Err(CaptureError::OutOfBounds);
    }
    encode_capture(
        image::imageops::crop_imm(
            &full,
            left.min(right) as u32,
            top.min(bottom) as u32,
            cropped_width,
            cropped_height,
        )
        .to_image(),
    )
}
