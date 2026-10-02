//! Resolve session model shortcuts against the existing runtime keymap.

use super::MAIN_RESERVED_BINDINGS;
use super::RuntimeKeymap;
use super::bindings::KeymapContext;
use super::bindings::runtime_action_bindings;
use super::chords::normalize_chord_binding;
use super::parse_keybinding;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use codex_config::types::Tui;
use codex_config::types::TuiModelPreset;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedModelPreset {
    pub(crate) name: String,
    pub(crate) preset: TuiModelPreset,
    pub(crate) binding: KeyBinding,
}

impl RuntimeKeymap {
    /// Resolve the complete local TUI config, including session model shortcuts.
    pub(crate) fn from_tui_config(tui: &Tui) -> Result<Self, String> {
        let mut resolved = Self::from_config(&tui.keymap)?;
        let mut seen = HashMap::new();
        for (name, preset) in &tui.model_presets {
            let path = format!("tui.model_presets.{name}.key");
            let binding = parse_keybinding(preset.key.as_str()).ok_or_else(|| {
                format!("{path}: expected a single key such as ctrl-1; chords are not supported")
            })?;
            let (code, modifiers) = binding.parts();
            if key_hint::is_plain_text_key_event(KeyEvent::new(code, modifiers))
                || matches!(code, KeyCode::Char(_)) && key_hint::is_altgr(modifiers)
            {
                return Err(format!(
                    "{path}: printable keys are reserved for text input"
                ));
            }
            #[cfg(unix)]
            if binding == key_hint::ctrl(KeyCode::Char('z')) {
                return Err(format!("{path}: ctrl-z is reserved for suspend"));
            }
            let key = normalize_chord_binding(binding).parts();
            if let Some(previous) = seen.insert(key, name) {
                return Err(format!(
                    "{path}: conflicts with tui.model_presets.{previous}.key; choose a unique key"
                ));
            }
            if let Some((action, _)) = MAIN_RESERVED_BINDINGS
                .iter()
                .find(|(_, reserved)| normalize_chord_binding(*reserved).parts() == key)
            {
                return Err(format!("{path}: key is reserved by {action}"));
            }
            if let Some(action) = runtime_action_bindings(&resolved)
                .filter(|action| action.id.context.overlaps(KeymapContext::Composer))
                .find(|action| {
                    action
                        .bindings
                        .iter()
                        .any(|other| normalize_chord_binding(*other).parts() == key)
                })
            {
                return Err(format!(
                    "{path}: conflicts with {}; unbind or remap that shortcut first",
                    action.id.config_path()
                ));
            }
            if let Some(chord) = resolved.chords.bindings.iter().find(|chord| {
                chord.action.context.overlaps(KeymapContext::Composer)
                    && normalize_chord_binding(chord.chord.prefix).parts() == key
            }) {
                return Err(format!(
                    "{path}: conflicts with the prefix of {}; choose a different key",
                    chord.action.config_path()
                ));
            }
            resolved.chat.model_presets.push(ResolvedModelPreset {
                name: name.clone(),
                preset: preset.clone(),
                binding,
            });
        }
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(key: &str) -> Tui {
        toml::from_str(&format!(
            "[model_presets.sol]\nmodel = 'gpt-6.1-sol'\nreasoning_effort = 'xhigh'\nkey = '{key}'"
        ))
        .unwrap()
    }

    #[test]
    fn model_presets_resolve_and_keep_existing_bindings() {
        let tui = config("ctrl-1");
        let runtime = RuntimeKeymap::from_tui_config(&tui).unwrap();
        assert_eq!(runtime.chat.model_presets.len(), 1);
        assert_eq!(
            runtime.chat.model_presets[0].binding,
            key_hint::ctrl(KeyCode::Char('1'))
        );
        assert_eq!(
            runtime.chat.increase_reasoning_effort,
            RuntimeKeymap::defaults().chat.increase_reasoning_effort
        );
    }

    #[test]
    fn model_presets_reject_duplicate_keys() {
        let mut tui = config("ctrl-1");
        tui.model_presets
            .insert("duplicate".into(), tui.model_presets["sol"].clone());
        let error = RuntimeKeymap::from_tui_config(&tui).unwrap_err();
        assert!(error.contains("tui.model_presets.sol.key"), "{error}");
        assert!(error.contains("tui.model_presets.duplicate.key"), "{error}");
    }

    #[test]
    fn model_presets_reject_existing_and_reserved_keys() {
        for (key, expected) in [
            ("alt-.", "increase_reasoning_effort"),
            ("ctrl-a", "move_line_start"),
            ("ctrl-5", "skip_question"),
            ("ctrl-7", "toggle_side_conversation"),
            ("ctrl-c", "interrupt_or_quit"),
            ("shift-tab", "cycle_collaboration_mode"),
        ] {
            let error = RuntimeKeymap::from_tui_config(&config(key)).unwrap_err();
            assert!(error.contains("tui.model_presets.sol.key"), "{error}");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn model_presets_reject_custom_keys_and_chord_prefixes() {
        for (preset_key, key) in [
            ("ctrl-1", "ctrl-1"),
            ("ctrl-1", "ctrl-1 ctrl-s"),
            ("ctrl-7", "ctrl-/ ctrl-s"),
        ] {
            let mut tui = config(preset_key);
            tui.keymap = toml::from_str(&format!("[global]\ncopy = '{key}'")).unwrap();
            let error = RuntimeKeymap::from_tui_config(&tui).unwrap_err();
            assert!(error.contains("tui.keymap.global.copy"), "{error}");
        }
    }

    #[test]
    fn model_presets_accept_disjoint_popup_keys_and_explicit_unbindings() {
        let mut tui = config("ctrl-1");
        tui.keymap = toml::from_str("[list]\nmove_up = 'ctrl-1'").unwrap();
        RuntimeKeymap::from_tui_config(&tui).unwrap();

        let mut tui = config("alt-.");
        tui.keymap = toml::from_str("[chat]\nincrease_reasoning_effort = []").unwrap();
        RuntimeKeymap::from_tui_config(&tui).unwrap();
    }

    #[test]
    fn model_presets_reject_printable_altgr_and_chord_keys() {
        for key in ["a", "shift-a", "ctrl-alt-a", "ctrl-1 ctrl-2"] {
            let error = RuntimeKeymap::from_tui_config(&config(key)).unwrap_err();
            assert!(error.contains("tui.model_presets.sol.key"), "{error}");
        }
    }
}
