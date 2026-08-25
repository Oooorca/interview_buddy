use crate::{
    error::{AppError, AppResult},
    storage::StorageManager,
};

pub(crate) fn build_main_window(
    app: &tauri::AppHandle,
    config: &tauri::utils::config::WindowConfig,
    _storage: &StorageManager,
) -> AppResult<tauri::WebviewWindow> {
    Ok(tauri::WebviewWindowBuilder::from_config(app, config)
        .map_err(|error| error.to_string())?
        .visible_on_all_workspaces(true)
        .build()
        .map_err(|error| error.to_string())?)
}

fn overlay_behavior(
    mut behavior: objc2_app_kit::NSWindowCollectionBehavior,
) -> objc2_app_kit::NSWindowCollectionBehavior {
    use objc2_app_kit::NSWindowCollectionBehavior;

    behavior.remove(
        NSWindowCollectionBehavior::FullScreenPrimary
            | NSWindowCollectionBehavior::FullScreenNone
            | NSWindowCollectionBehavior::Primary
            | NSWindowCollectionBehavior::Auxiliary,
    );
    behavior.insert(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::CanJoinAllApplications,
    );
    behavior
}

pub(crate) fn configure_overlay(window: &tauri::WebviewWindow) -> AppResult<()> {
    use objc2_app_kit::NSWindow;

    let pointer = window
        .ns_window()
        .map_err(|error| format!("读取 macOS 原生窗口失败：{error}"))?
        .cast::<NSWindow>();
    let ns_window =
        unsafe { pointer.as_ref() }.ok_or_else(|| AppError::from("macOS 原生窗口指针为空"))?;
    let behavior = overlay_behavior(ns_window.collectionBehavior());
    ns_window.setCollectionBehavior(behavior);
    Ok(())
}

pub(crate) fn verify_capture_protection(_window: &tauri::WebviewWindow) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_joins_spaces_and_full_screen_without_becoming_primary() {
        use objc2_app_kit::NSWindowCollectionBehavior;

        let behavior = overlay_behavior(
            NSWindowCollectionBehavior::Managed
                | NSWindowCollectionBehavior::FullScreenPrimary
                | NSWindowCollectionBehavior::FullScreenNone
                | NSWindowCollectionBehavior::Primary
                | NSWindowCollectionBehavior::Auxiliary,
        );
        assert!(behavior.contains(NSWindowCollectionBehavior::Managed));
        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllApplications));
        assert!(!behavior.contains(NSWindowCollectionBehavior::FullScreenPrimary));
        assert!(!behavior.contains(NSWindowCollectionBehavior::FullScreenNone));
        assert!(!behavior.contains(NSWindowCollectionBehavior::Primary));
        assert!(!behavior.contains(NSWindowCollectionBehavior::Auxiliary));
    }
}
