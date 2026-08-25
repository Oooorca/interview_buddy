use super::{coding_prompt, system_prompt, AnswerLanguage, AppSettings, PromptMode, UiLanguage};

const KNOWN_SYSTEM_DEFAULTS: &[&str] = &[
    "你是会议实时 Copilot。根据上下文先理解对方意图，再给出自然、简短、可以直接说出口的中文回答；必要时补充要点和一个合适的反问。不要编造事实。",
    "你是会议实时 Copilot。根据上下文给出自然、简短、可直接说出口的中文回答；必要时补充要点和反问。不要编造事实。",
    "你是会议实时 Copilot。根据上下文给出自然、简短、可直接说出口的中文回答；必要时补充要点和反问。",
];

const KNOWN_CODING_DEFAULTS: &[&str] = &[
    "你是算法面试助手。识别截图中的完整题目，给出：1. 核心思路；2. 时间与空间复杂度；3. 可直接提交的代码；4. 容易出错的边界情况。默认使用 TypeScript；如果截图指定语言则遵从截图。回答紧凑、正确、适合手撕讲解。",
    "你是算法面试助手。识别截图题目，给出核心思路、复杂度、可提交代码和边界情况。",
    "你是算法面试助手。识别截图题目，给出python语言的核心思路、复杂度、可提交代码和边界情况。对于复杂的题目，先给出一个最直观但复杂度较高的解法及代码，再逐步优化到最优解，展现思维过程。",
];

fn legacy_prompt_mode(prompt: Option<&str>, current: &[&str], known: &[&str]) -> PromptMode {
    let value = prompt.unwrap_or_default().trim();
    if value.is_empty()
        || current.iter().any(|default| default.trim() == value)
        || known.iter().any(|default| default.trim() == value)
    {
        PromptMode::Default
    } else {
        PromptMode::Custom
    }
}

fn normalize_prompt_field(
    mode: &mut PromptMode,
    prompt: &mut Option<String>,
    label: &str,
    reject_empty_custom: bool,
) -> Result<bool, String> {
    match mode {
        PromptMode::Default | PromptMode::Disabled => Ok(prompt.take().is_some()),
        PromptMode::Custom => {
            if prompt
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                Ok(false)
            } else if reject_empty_custom {
                Err(format!("{label}处于自定义模式，但内容为空"))
            } else {
                *mode = PromptMode::Default;
                *prompt = None;
                Ok(true)
            }
        }
    }
}

pub fn normalize_prompt_settings(
    settings: &mut AppSettings,
    reject_empty_custom: bool,
) -> Result<bool, String> {
    let system_changed = normalize_prompt_field(
        &mut settings.system_prompt_mode,
        &mut settings.system_prompt,
        "系统 Prompt",
        reject_empty_custom,
    )?;
    let coding_changed = normalize_prompt_field(
        &mut settings.coding_prompt_mode,
        &mut settings.coding_prompt,
        "纯截图 Prompt",
        reject_empty_custom,
    )?;
    Ok(system_changed || coding_changed)
}

pub fn migrate_legacy_settings(source: &str, settings: &mut AppSettings) -> bool {
    let mut changed = false;
    if settings.my_transcription_language == "en" {
        settings.my_transcription_language = "en-US".into();
        changed = true;
    }
    if settings.their_transcription_language == "en" {
        settings.their_transcription_language = "en-US".into();
        changed = true;
    }
    if !source.contains("\"uiLanguage\"") {
        settings.ui_language = UiLanguage::ZhCn;
        changed = true;
    }
    if !source.contains("\"answerLanguage\"") {
        settings.answer_language = AnswerLanguage::ZhCn;
        changed = true;
    }
    if !source.contains("\"systemPromptMode\"") {
        settings.system_prompt_mode = legacy_prompt_mode(
            settings.system_prompt.as_deref(),
            &[system_prompt("zh-CN"), system_prompt("en-US")],
            KNOWN_SYSTEM_DEFAULTS,
        );
        changed = true;
    }
    if !source.contains("\"codingPromptMode\"") {
        settings.coding_prompt_mode = legacy_prompt_mode(
            settings.coding_prompt.as_deref(),
            &[coding_prompt("zh-CN"), coding_prompt("en-US")],
            KNOWN_CODING_DEFAULTS,
        );
        changed = true;
    }
    changed
}

pub fn resolved_system_prompt<'a>(
    settings: &'a AppSettings,
    answer_locale: &str,
) -> Option<&'a str> {
    match settings.system_prompt_mode {
        PromptMode::Default => Some(system_prompt(answer_locale)),
        PromptMode::Custom => settings
            .system_prompt
            .as_deref()
            .filter(|prompt| !prompt.trim().is_empty()),
        PromptMode::Disabled => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_prompts_migrate_empty_and_known_defaults_to_default_mode() {
        let mut empty = AppSettings {
            system_prompt: Some(String::new()),
            ..AppSettings::default()
        };
        assert!(migrate_legacy_settings("{}", &mut empty));
        assert_eq!(empty.system_prompt_mode, PromptMode::Default);

        let mut builtin = AppSettings {
            system_prompt: Some(system_prompt("zh-CN").into()),
            ..AppSettings::default()
        };
        migrate_legacy_settings("{}", &mut builtin);
        assert_eq!(builtin.system_prompt_mode, PromptMode::Default);

        let mut custom = AppSettings {
            system_prompt: Some("我的专用回答规则".into()),
            ..AppSettings::default()
        };
        migrate_legacy_settings("{}", &mut custom);
        assert_eq!(custom.system_prompt_mode, PromptMode::Custom);
    }

    #[test]
    fn legacy_settings_keep_the_previous_chinese_behavior() {
        let mut settings = AppSettings {
            my_transcription_language: "en".into(),
            their_transcription_language: "en".into(),
            ..AppSettings::default()
        };
        assert!(migrate_legacy_settings("{}", &mut settings));
        assert_eq!(settings.ui_language, UiLanguage::ZhCn);
        assert_eq!(settings.answer_language, AnswerLanguage::ZhCn);
        assert_eq!(settings.my_transcription_language, "en-US");
        assert_eq!(settings.their_transcription_language, "en-US");

        let source = r#"{"uiLanguage":"en-US","answerLanguage":"follow-ui","systemPromptMode":"default","codingPromptMode":"default"}"#;
        let mut current = AppSettings::default();
        assert!(!migrate_legacy_settings(source, &mut current));
        assert_eq!(current.ui_language, UiLanguage::System);
        assert_eq!(current.answer_language, AnswerLanguage::FollowUi);
    }

    #[test]
    fn prompt_modes_resolve_and_normalize_safely() {
        let mut settings = AppSettings::default();
        assert_eq!(
            resolved_system_prompt(&settings, "zh-CN"),
            Some(system_prompt("zh-CN"))
        );
        assert_ne!(system_prompt("zh-CN"), system_prompt("en-US"));

        settings.system_prompt_mode = PromptMode::Disabled;
        settings.system_prompt = Some("ignored".into());
        assert!(normalize_prompt_settings(&mut settings, true).expect("normalize"));
        assert_eq!(settings.system_prompt, None);
        assert_eq!(resolved_system_prompt(&settings, "zh-CN"), None);

        settings.system_prompt_mode = PromptMode::Custom;
        assert!(normalize_prompt_settings(&mut settings, true).is_err());
        settings.system_prompt = Some("custom".into());
        assert!(!normalize_prompt_settings(&mut settings, true).expect("custom"));
        assert_eq!(resolved_system_prompt(&settings, "en-US"), Some("custom"));
    }

    #[test]
    fn default_prompt_settings_serialize_without_copying_builtin_text() {
        let value = serde_json::to_value(AppSettings::default()).expect("serialize settings");
        assert_eq!(value["systemPromptMode"], "default");
        assert_eq!(value["codingPromptMode"], "default");
        assert!(value["systemPrompt"].is_null());
        assert!(value["codingPrompt"].is_null());
    }
}
