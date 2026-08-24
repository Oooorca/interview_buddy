use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct PromptDefaults {
    system: String,
    coding: String,
}

static DEFAULTS: LazyLock<PromptDefaults> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../resources/default-prompts.json"))
        .expect("resources/default-prompts.json must be valid")
});

pub fn system_prompt() -> &'static str {
    &DEFAULTS.system
}

pub fn coding_prompt() -> &'static str {
    &DEFAULTS.coding
}
