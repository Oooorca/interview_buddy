use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip)]
    source_message: String,
}

impl AppError {
    pub fn from_message(message: impl Into<String>) -> Self {
        let message = message.into();
        let (code, detail) = classify(&message);
        Self {
            code: code.into(),
            detail,
            source_message: message,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.source_message)
    }
}

impl std::error::Error for AppError {}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::from_message(message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::from_message(message)
    }
}

fn classify(message: &str) -> (&'static str, Option<String>) {
    let exact = match message {
        "请先配置 API Key" => Some("api.keyRequired"),
        "新的 API Key 不能为空" => Some("api.emptyKey"),
        "录音为空" => Some("audio.emptyRecording"),
        "系统音频已经在录制" => Some("audio.alreadyStarted"),
        "系统音频尚未开始录制" => Some("audio.notStarted"),
        "当前平台暂不支持系统音频采集" => Some("audio.unsupported"),
        "当前平台不支持安全设置存储" => Some("security.unsupported"),
        "加密设置密钥不存在" | "macOS Keychain 中没有设置密钥" => {
            Some("security.keyMissing")
        }
        "无法验证或解密设置" => Some("security.authenticationFailed"),
        "加密设置版本、用途或算法不受支持" => Some("security.unsupportedEnvelope"),
        "安全设置存储不可用" => Some("security.storeUnavailable"),
        "设置内容超过 1 MiB 安全上限" | "解密后的设置超过 1 MiB 安全上限" => {
            Some("security.plaintextTooLarge")
        }
        "加密设置文件超过 2 MiB 安全上限" => Some("security.envelopeTooLarge"),
        "设置密钥长度无效" | "DPAPI 密钥长度无效" | "macOS Keychain 中的设置密钥长度无效" => {
            Some("security.invalidKey")
        }
        "加密设置外壳格式无效" | "加密设置 nonce 无效" | "加密设置 nonce 长度无效"
        | "加密设置密文无效" => Some("security.invalidEnvelope"),
        "设置加密失败" => Some("security.encryptFailed"),
        "Windows DPAPI 返回了空数据" => Some("security.dpapiFailed"),
        "DPAPI 输入过大" => Some("security.plaintextTooLarge"),
        "存储位置已经修改，请重启应用后再进行下一次修改" => {
            Some("storage.restartRequired")
        }
        "存储目录必须是绝对路径" | "存储引导文件中的目录不是绝对路径"
        | "存储引导文件中的迁移目录不是绝对路径" => Some("storage.absolutePathRequired"),
        "拒绝操作未经 Interview Buddy 标记的目录" => Some("storage.unmanagedRoot"),
        "拒绝清理 WebView2 数据目录之外的路径" | "拒绝删除不安全的目录路径" => {
            Some("storage.unsafeOperation")
        }
        "明文存储引导文件超过 64 KiB 安全上限" => Some("storage.pointerTooLarge"),
        "区域截图选择器已经打开" => Some("capture.alreadyOpen"),
        "区域截图选择器已经关闭" | "区域截图会话已经结束" => Some("capture.sessionEnded"),
        "截图区域太小" | "截图区域无效或尺寸过小" => Some("capture.tooSmall"),
        "截图区域超出当前显示器或尺寸过小" => Some("capture.outOfBounds"),
        "读取鼠标位置失败" => Some("capture.cursorFailed"),
        "没有找到主窗口" => Some("capture.mainWindowMissing"),
        "当前窗口不是 Win32 窗口" => Some("window.unsupported"),
        "缺少 main 窗口配置" => Some("window.mainConfigurationMissing"),
        "没有找到窗口所在显示器" => Some("window.monitorUnavailable"),
        "WASAPI 启动超时" => Some("audio.startTimeout"),
        "WASAPI 采集线程异常退出" | "ScreenCaptureKit 采集线程异常退出" => {
            Some("audio.captureThreadFailed")
        }
        "没有找到显示器" => Some("audio.displayMissing"),
        "系统音频启动超时。若 macOS 的授权开关已经打开，请先关闭再重新打开 Interview Buddy 的“屏幕与系统音频录制”，然后彻底退出并重启应用。本地临时签名在重新构建后可能需要重新授权。" => {
            Some("audio.macosPermission")
        }
        "LLM 流式响应单行超过 1 MiB 安全上限" => Some("llm.streamLineTooLarge"),
        "LLM 流式响应中没有文本内容" | "LLM 响应中没有文本内容" => {
            Some("llm.emptyResponse")
        }
        "百炼 qwen3-asr-flash 单次音频不能超过 10MB" => {
            Some("transcription.dashscopeTooLarge")
        }
        "无法确定应用配置目录" | "无法确定应用数据目录"
        | "无法确定系统应用数据目录" | "无法确定程序所在目录"
        | "无法确定安全文件目录" | "无法确定设置文件目录" => {
            Some("storage.resolveFailed")
        }
        "旧明文设置超过 1 MiB 安全上限" => Some("security.plaintextTooLarge"),
        "加密设置迁移校验失败" => Some("settings.verificationFailed"),
        _ => None,
    };
    if let Some(code) = exact {
        return (code, None);
    }

    const PREFIXES: &[(&str, &str)] = &[
        ("读取当前显示器失败：", "window.monitorReadFailed"),
        ("读取主显示器失败：", "window.monitorReadFailed"),
        ("读取窗口尺寸失败：", "window.sizeReadFailed"),
        ("调整窗口尺寸失败：", "window.resizeFailed"),
        ("居中窗口失败：", "window.centerFailed"),
        ("安全设置已锁定：", "security.locked"),
        ("无法读取 DPAPI 密钥：", "security.keyReadFailed"),
        ("无法重置 DPAPI 密钥：", "security.keyResetFailed"),
        ("Windows DPAPI 操作失败：", "security.dpapiFailed"),
        ("无法打开 macOS Keychain：", "security.keychainOpenFailed"),
        (
            "无法保存 macOS Keychain 密钥：",
            "security.keychainSaveFailed",
        ),
        ("读取流式回答失败：", "llm.streamReadFailed"),
        ("LLM 流包含无效 UTF-8：", "llm.invalidUtf8"),
        ("无法解析流式回答：", "llm.invalidStream"),
        ("读取普通回答失败：", "llm.responseReadFailed"),
        ("LLM 返回了无法解析的响应：", "llm.invalidResponse"),
        ("LLM 服务返回 ", "llm.serviceError"),
        ("流式回答失败（", "llm.fallbackFailed"),
        ("读取转写响应失败：", "transcription.responseReadFailed"),
        ("转写服务返回 HTTP ", "transcription.serviceError"),
        ("转写响应中没有可用文本：", "transcription.emptyResponse"),
        (
            "连接百炼转写服务失败：",
            "transcription.dashscopeConnectFailed",
        ),
        ("读取百炼转写响应失败：", "transcription.responseReadFailed"),
        ("百炼返回无法解析的响应：", "transcription.invalidResponse"),
        ("百炼转写返回 HTTP ", "transcription.serviceError"),
        ("百炼转写响应中没有文本：", "transcription.emptyResponse"),
        ("创建音频设备枚举器失败：", "audio.deviceEnumerationFailed"),
        ("枚举系统输出设备失败：", "audio.deviceEnumerationFailed"),
        ("打开指定输出设备失败：", "audio.openDeviceFailed"),
        ("打开默认输出设备失败：", "audio.openDeviceFailed"),
        ("激活 WASAPI 客户端失败：", "audio.wasapiFailed"),
        ("读取输出格式失败：", "audio.formatFailed"),
        ("初始化 WASAPI loopback 失败：", "audio.wasapiFailed"),
        ("获取 WASAPI 采集接口失败：", "audio.wasapiFailed"),
        ("启动系统音频失败：", "audio.startFailed"),
        ("停止系统音频失败：", "audio.stopFailed"),
        ("读取屏幕内容失败：", "audio.screenContentFailed"),
        ("无法创建区域截图选择器：", "capture.createFailed"),
        ("无法显示区域截图选择器：", "capture.showFailed"),
        ("区域截图任务失败：", "capture.failed"),
        ("读取窗口捕获保护失败：", "window.captureProtectionFailed"),
        ("无法创建存储目录：", "storage.createFailed"),
        ("无法解析存储目录：", "storage.resolveFailed"),
        ("存储目录不可写：", "storage.notWritable"),
        ("无法完成目录写入测试：", "storage.notWritable"),
        ("无法初始化存储目录：", "storage.initializeFailed"),
        (
            "无法创建 WebView2 数据目录：",
            "storage.webviewCreateFailed",
        ),
        ("无法确定程序路径：", "storage.resolveFailed"),
        ("无法检查存储引导文件大小：", "storage.pointerReadFailed"),
        ("无法读取存储引导文件：", "storage.pointerReadFailed"),
        ("存储引导文件格式错误：", "storage.pointerInvalid"),
        ("无法创建存储恢复目录：", "storage.migrationFailed"),
        ("无法隔离存储位置文件：", "storage.migrationFailed"),
        ("无法读取加密存储位置：", "storage.pointerReadFailed"),
        ("加密存储位置格式无效：", "storage.pointerInvalid"),
        ("无法删除旧明文存储位置：", "storage.migrationFailed"),
        ("无法安排缓存清理：", "storage.cleanupScheduleFailed"),
        ("无法移除清理标记：", "storage.cleanupFailed"),
        ("清理 ", "storage.cleanupFailed"),
        ("迁移设置失败：", "storage.migrationFailed"),
        ("迁移旧设置失败：", "storage.migrationFailed"),
        ("迁移便携版设置失败：", "storage.migrationFailed"),
        ("删除旧数据目录失败：", "storage.migrationFailed"),
        ("清理旧设置失败：", "storage.migrationFailed"),
        ("准备迁移目标目录失败：", "storage.migrationFailed"),
        ("无法读取旧设置：", "settings.legacyReadFailed"),
        ("旧设置格式无效：", "settings.legacyInvalid"),
        ("无法检查旧设置大小：", "settings.legacyReadFailed"),
        ("无法创建恢复目录：", "settings.quarantineFailed"),
        ("无法隔离旧设置：", "settings.quarantineFailed"),
        ("无法隔离损坏设置：", "settings.quarantineFailed"),
        ("无法删除已迁移的旧明文设置：", "settings.migrationFailed"),
        ("无法创建安全文件目录：", "settings.writeFailed"),
        ("无法创建安全临时文件：", "settings.writeFailed"),
        ("无法写入安全临时文件：", "settings.writeFailed"),
        ("无法同步安全临时文件：", "settings.writeFailed"),
        ("无法创建设置目录：", "settings.writeFailed"),
        ("无法创建设置临时文件：", "settings.writeFailed"),
        ("无法写入设置临时文件：", "settings.writeFailed"),
        ("无法同步设置临时文件：", "settings.writeFailed"),
        ("无法轮换设置备份：", "settings.writeFailed"),
        ("无法创建设置备份：", "settings.writeFailed"),
        ("解密设置格式无效：", "settings.invalidDecrypted"),
        ("加密设置回读校验失败：", "settings.verificationFailed"),
        ("无法读取加密设置：", "settings.readFailed"),
        ("无法保存安全文件：", "settings.writeFailed"),
        ("无法替换设置文件：", "settings.writeFailed"),
        ("系统 Prompt处于自定义模式", "settings.emptyCustomPrompt"),
        ("纯截图 Prompt处于自定义模式", "settings.emptyCustomPrompt"),
    ];
    for (prefix, code) in PREFIXES {
        if let Some(detail) = message.strip_prefix(prefix) {
            return (code, clean_detail(detail));
        }
    }

    let detail =
        (!contains_han(message) && !message.trim().is_empty()).then(|| message.to_string());
    ("unknown", detail)
}

fn clean_detail(detail: &str) -> Option<String> {
    let detail = detail.trim();
    if detail.is_empty() || contains_han(detail) {
        None
    } else {
        Some(detail.chars().take(500).collect())
    }
}

fn contains_han(value: &str) -> bool {
    value.chars().any(|character| {
        ('\u{3400}'..='\u{4DBF}').contains(&character)
            || ('\u{4E00}'..='\u{9FFF}').contains(&character)
            || ('\u{F900}'..='\u{FAFF}').contains(&character)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_errors_without_exposing_localized_rust_text() {
        let error = AppError::from_message("请先配置 API Key");
        assert_eq!(error.code(), "api.keyRequired");
        assert_eq!(error.detail(), None);

        let error = AppError::from_message("启动系统音频失败：access denied");
        assert_eq!(error.code(), "audio.startFailed");
        assert_eq!(error.detail(), Some("access denied"));
    }

    #[test]
    fn unknown_internal_localized_errors_are_not_exposed() {
        let error = AppError::from_message("无法完成某个内部操作");
        assert_eq!(error.code(), "unknown");
        assert_eq!(error.detail(), None);
    }
}
