//! Apply configured model and reasoning shortcuts through session model selection.

use super::ChatWidget;
use super::PARENT_OWNED_INPUT_MESSAGE;
use crate::app_event::AppEvent;
use codex_protocol::openai_models::ReasoningEffort;
use crossterm::event::KeyEvent;

impl ChatWidget {
    pub(super) fn handle_model_preset_shortcut(&mut self, key_event: KeyEvent) -> bool {
        let Some(shortcut) = self
            .chat_keymap
            .model_presets
            .iter()
            .find(|shortcut| shortcut.binding.is_press(key_event))
            .cloned()
        else {
            return false;
        };
        if !self.bottom_pane.no_modal_or_popup_active() {
            return false;
        }
        if self.blocks_direct_input {
            self.add_error_message(PARENT_OWNED_INPUT_MESSAGE.to_string());
            return true;
        }
        if !self.is_session_configured() {
            self.add_info_message(
                "Model presets are disabled until startup completes.".to_string(),
                /*hint*/ None,
            );
            return true;
        }
        let preset = shortcut.preset;
        let Some(model) = self
            .model_catalog
            .try_list_models()
            .unwrap_or_default()
            .into_iter()
            .find(|model| model.model == preset.model && model.show_in_picker)
        else {
            self.add_error_message(format!(
                "Model preset '{}' is unavailable: {} is not in the available model catalog.",
                shortcut.name, preset.model
            ));
            return true;
        };
        if !model
            .supported_reasoning_efforts
            .iter()
            .any(|option| option.effort == preset.reasoning_effort)
            && !(model.supported_reasoning_efforts.is_empty()
                && model.default_reasoning_effort == preset.reasoning_effort)
        {
            self.add_error_message(format!(
                "Model preset '{}' is unavailable: {} does not support reasoning effort {}.",
                shortcut.name, preset.model, preset.reasoning_effort
            ));
            return true;
        }
        if preset.reasoning_effort == ReasoningEffort::Ultra {
            self.add_info_message(
                format!(
                    "Ultra requires explicit selection under /model → {} → More reasoning…",
                    preset.model
                ),
                /*hint*/ None,
            );
            return true;
        }
        self.app_event_tx.send(AppEvent::SelectSessionModel {
            model: preset.model,
            effort: Some(preset.reasoning_effort),
        });
        true
    }
}
