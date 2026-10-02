use super::*;
use codex_config::types::Tui;

fn install_presets(chat: &mut ChatWidget) {
    let tui: Tui = toml::from_str(
        r#"
[model_presets.sol]
model = "gpt-6.1-sol"
reasoning_effort = "xhigh"
key = "ctrl-1"
[model_presets.astra]
model = "gpt-6-astra"
reasoning_effort = "high"
key = "ctrl-2"
"#,
    )
    .unwrap();
    let keymap = RuntimeKeymap::from_tui_config(&tui).unwrap();
    chat.local_settings.tui = tui.clone();
    chat.apply_keymap_update(tui.keymap, &keymap);
    let mut model = chat.model_catalog.models[0].clone();
    model.model = "gpt-6.1-sol".into();
    model.show_in_picker = true;
    model.supported_reasoning_efforts = [ReasoningEffortConfig::High, ReasoningEffortConfig::XHigh]
        .into_iter()
        .map(|effort| ReasoningEffortPreset {
            effort,
            description: String::new(),
        })
        .collect();
    let mut astra = model.clone();
    astra.model = "gpt-6-astra".into();
    Arc::make_mut(&mut chat.model_catalog).models = vec![model, astra];
}

fn selections(events: &[AppEvent]) -> Vec<(String, Option<ReasoningEffortConfig>)> {
    events
        .iter()
        .filter_map(|event| match event {
            AppEvent::SelectSessionModel { model, effort } => Some((model.clone(), effort.clone())),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn model_presets_dispatch_session_selection_and_preserve_running_input() {
    for plan_mode in [false, true] {
        let (mut chat, mut rx, _ops) = make_chatwidget_manual(Some("gpt-5.5")).await;
        install_presets(&mut chat);
        chat.thread_id = Some(ThreadId::new());
        chat.set_feature_enabled(Feature::CollaborationModes, true);
        if plan_mode {
            let mask = collaboration_modes::plan_mask(chat.model_catalog.as_ref()).unwrap();
            chat.set_collaboration_mask(mask);
        }
        chat.bottom_pane
            .set_composer_text("draft stays here".into(), Vec::new(), Vec::new());
        chat.on_task_started();
        chat.arm_quit_shortcut(key_hint::ctrl(KeyCode::Char('c')));
        while rx.try_recv().is_ok() {}

        for digit in ['1', '2'] {
            chat.handle_key_event(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::CONTROL));
        }
        let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert_eq!(
            selections(&events),
            vec![
                ("gpt-6.1-sol".into(), Some(ReasoningEffortConfig::XHigh)),
                ("gpt-6-astra".into(), Some(ReasoningEffortConfig::High)),
            ]
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AppEvent::PersistModelSelection { .. }))
        );
        assert_eq!(chat.bottom_pane.composer_text(), "draft stays here");
        assert!(chat.bottom_pane.is_task_running());
        assert!(!chat.bottom_pane.quit_shortcut_hint_visible());
        assert!(chat.quit_shortcut_expires_at.is_none());
        assert!(chat.quit_shortcut_key.is_none());
    }
}

#[tokio::test]
async fn model_presets_obey_startup_parent_ownership_and_popup_guards() {
    for guard in ["startup", "parent", "popup"] {
        let (mut chat, mut rx, _ops) = make_chatwidget_manual(Some("gpt-5.5")).await;
        install_presets(&mut chat);
        if guard != "startup" {
            chat.thread_id = Some(ThreadId::new());
        }
        if guard == "parent" {
            chat.blocks_direct_input = true;
        }
        if guard == "popup" {
            chat.open_model_popup();
        }
        while rx.try_recv().is_ok() {}
        chat.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL));
        let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(selections(&events).is_empty(), "guard: {guard}");
        if guard != "popup" {
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, AppEvent::InsertHistoryCell(_))),
                "guard: {guard}"
            );
        }
    }
}

#[tokio::test]
async fn model_presets_reject_unavailable_models_unsupported_effort_and_ultra() {
    for invalid in ["model", "hidden", "effort", "ultra"] {
        let (mut chat, mut rx, _ops) = make_chatwidget_manual(Some("gpt-5.5")).await;
        install_presets(&mut chat);
        chat.thread_id = Some(ThreadId::new());
        match invalid {
            "model" => chat
                .chat_keymap
                .model_presets
                .iter_mut()
                .for_each(|entry| entry.preset.model = "unknown-model".into()),
            "hidden" => Arc::make_mut(&mut chat.model_catalog)
                .models
                .iter_mut()
                .for_each(|model| model.show_in_picker = false),
            "effort" => {
                chat.chat_keymap.model_presets.iter_mut().for_each(|entry| {
                    entry.preset.reasoning_effort = ReasoningEffortConfig::Minimal
                })
            }
            "ultra" => {
                chat.chat_keymap
                    .model_presets
                    .iter_mut()
                    .for_each(|entry| entry.preset.reasoning_effort = ReasoningEffortConfig::Ultra);
                Arc::make_mut(&mut chat.model_catalog).models[0]
                    .supported_reasoning_efforts
                    .push(ReasoningEffortPreset {
                        effort: ReasoningEffortConfig::Ultra,
                        description: String::new(),
                    });
            }
            _ => unreachable!(),
        }
        while rx.try_recv().is_ok() {}
        chat.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL));
        let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(selections(&events).is_empty(), "invalid: {invalid}");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AppEvent::InsertHistoryCell(_))),
            "invalid: {invalid}"
        );
        assert_eq!(chat.current_model(), "gpt-5.5");
    }
}

#[tokio::test]
async fn model_presets_ignore_key_release_and_keep_shortcuts_when_reconfigured() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(Some("gpt-5.5")).await;
    install_presets(&mut chat);
    chat.thread_id = Some(ThreadId::new());
    let mut tui = chat.local_settings.tui.clone();
    tui.keymap = toml::from_str("[global]\ncopy = 'ctrl-9'").unwrap();
    let runtime = RuntimeKeymap::from_tui_config(&tui).unwrap();
    chat.apply_keymap_update(tui.keymap, &runtime);
    while rx.try_recv().is_ok() {}
    chat.handle_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('1'),
        KeyModifiers::CONTROL,
        KeyEventKind::Release,
    ));
    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert!(selections(&events).is_empty());
    chat.handle_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('1'),
        KeyModifiers::CONTROL,
        KeyEventKind::Repeat,
    ));
    let events = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    assert_eq!(
        selections(&events),
        vec![("gpt-6.1-sol".into(), Some(ReasoningEffortConfig::XHigh))]
    );
}
