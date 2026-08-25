use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct PromptDefaults {
    system: String,
    coding: String,
}

static ZH_CN_DEFAULTS: LazyLock<PromptDefaults> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../resources/default-prompts/zh-CN.json"
    ))
    .expect("resources/default-prompts/zh-CN.json must be valid")
});

static EN_US_DEFAULTS: LazyLock<PromptDefaults> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../resources/default-prompts/en-US.json"
    ))
    .expect("resources/default-prompts/en-US.json must be valid")
});

fn defaults(locale: &str) -> &'static PromptDefaults {
    if locale == "en-US" {
        &EN_US_DEFAULTS
    } else {
        &ZH_CN_DEFAULTS
    }
}

pub fn system_prompt(locale: &str) -> &'static str {
    &defaults(locale).system
}

pub fn coding_prompt(locale: &str) -> &'static str {
    &defaults(locale).coding
}
