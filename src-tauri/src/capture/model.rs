use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::platform::CaptureContext;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegionSelection {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl RegionSelection {
    pub(crate) fn validate(&self) -> Result<(), CaptureError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.x < 0.0
            || self.y < 0.0
            || self.width < 2.0
            || self.height < 2.0
        {
            return Err(CaptureError::InvalidSelection);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureResult {
    pub(crate) data_url: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MonitorGeometry {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegionCaptureSession {
    pub(crate) restore_main_window: bool,
    pub(crate) context: CaptureContext,
}

#[derive(Debug)]
pub(crate) enum CaptureError {
    AlreadyOpen,
    SessionEnded,
    InvalidSelection,
    TooSmall,
    OutOfBounds,
    InvalidScale,
    CursorUnavailable,
    MainWindowMissing,
    CreateSelector(String),
    ShowSelector(String),
    Task(String),
    Operation(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyOpen => formatter.write_str("区域截图选择器已经打开"),
            Self::SessionEnded => formatter.write_str("区域截图会话已经结束"),
            Self::InvalidSelection => formatter.write_str("截图区域无效或尺寸过小"),
            Self::TooSmall => formatter.write_str("截图区域太小"),
            Self::OutOfBounds => formatter.write_str("截图区域超出当前显示器或尺寸过小"),
            Self::InvalidScale => formatter.write_str("显示器缩放比例无效"),
            Self::CursorUnavailable => formatter.write_str("读取鼠标位置失败"),
            Self::MainWindowMissing => formatter.write_str("没有找到主窗口"),
            Self::CreateSelector(detail) => write!(formatter, "无法创建区域截图选择器：{detail}"),
            Self::ShowSelector(detail) => write!(formatter, "无法显示区域截图选择器：{detail}"),
            Self::Task(detail) => write!(formatter, "区域截图任务失败：{detail}"),
            Self::Operation(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for CaptureError {}

impl From<CaptureError> for AppError {
    fn from(error: CaptureError) -> Self {
        Self::from_message(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_validation_rejects_non_finite_and_tiny_regions() {
        let non_finite = RegionSelection {
            x: f64::NAN,
            y: 0.0,
            width: 40.0,
            height: 40.0,
        };
        assert!(matches!(
            non_finite.validate(),
            Err(CaptureError::InvalidSelection)
        ));

        let tiny = RegionSelection {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 40.0,
        };
        assert!(matches!(
            tiny.validate(),
            Err(CaptureError::InvalidSelection)
        ));
    }

    #[test]
    fn typed_capture_errors_keep_stable_public_codes() {
        let out_of_bounds = AppError::from(CaptureError::OutOfBounds);
        assert_eq!(out_of_bounds.code(), "capture.outOfBounds");

        let create = AppError::from(CaptureError::CreateSelector("denied".into()));
        assert_eq!(create.code(), "capture.createFailed");
        assert_eq!(create.detail(), Some("denied"));

        let task = AppError::from(CaptureError::Task("cancelled".into()));
        assert_eq!(task.code(), "capture.failed");
        assert_eq!(task.detail(), Some("cancelled"));
    }
}
