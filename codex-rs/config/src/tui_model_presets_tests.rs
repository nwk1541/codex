use super::Tui;
use crate::config_toml::ConfigToml;
use codex_protocol::openai_models::ReasoningEffort;

#[test]
fn model_presets_parse_and_normalize_keys() {
    let config: ConfigToml = toml::from_str(
        r#"
[tui.model_presets.sol]
model = "gpt-6.1-sol"
reasoning_effort = "xhigh"
key = "Control-1"

[tui.model_presets.astra]
model = "gpt-6-astra"
reasoning_effort = "high"
key = "ctrl-2"
"#,
    )
    .unwrap();
    let tui = config.tui.unwrap();
    assert_eq!(tui.model_presets.len(), 2);
    let sol = &tui.model_presets["sol"];
    assert_eq!(sol.model, "gpt-6.1-sol");
    assert_eq!(sol.reasoning_effort, ReasoningEffort::XHigh);
    assert_eq!(sol.key.as_str(), "ctrl-1");
    assert_eq!(
        tui.model_presets["astra"].reasoning_effort,
        ReasoningEffort::High
    );
}

#[test]
fn model_presets_are_optional_in_existing_tui_config() {
    let tui: Tui = toml::from_str("animations = false").unwrap();
    assert!(tui.model_presets.is_empty());
    assert!(!toml::to_string(&tui).unwrap().contains("model_presets"));
}

#[test]
fn model_presets_reject_missing_fields_unknown_fields_and_invalid_keys() {
    for (entry, expected) in [
        ("model = 'gpt-6-astra'\nkey = 'ctrl-2'", "reasoning_effort"),
        ("reasoning_effort = 'high'\nkey = 'ctrl-2'", "model"),
        ("model = 'gpt-6-astra'\nreasoning_effort = 'high'", "key"),
        (
            "model = 'gpt-6-astra'\nreasoning_effort = 'high'\nkey = 'ctrl-2'\nextra = true",
            "unknown field",
        ),
        (
            "model = 'gpt-6-astra'\nreasoning_effort = 'high'\nkey = 'not-a-key'",
            "key",
        ),
    ] {
        let error = toml::from_str::<Tui>(&format!("[model_presets.test]\n{entry}"))
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}
