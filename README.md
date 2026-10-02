# Codex CLI: 모델 · Reasoning 프리셋

[openai/codex](https://github.com/openai/codex)를 기반으로, TUI에서 모델과 reasoning effort를 프리셋 단축키로 전환하는 기능을 실험하는 비공식 포크입니다.

관련 이슈: [Model + Reasoning presets/profiles #44603](https://github.com/openai/codex/issues/44603). 해당 이슈는 데스크톱 앱 대상이며, 이 포크에서는 CLI/TUI의 구현 가능성과 동작을 검증합니다. 구현과 테스트 결과를 이슈의 설계 논의에 공유하는 것이 목표입니다.

**현재 상태:** 기존 코드 경로 검토 완료. 프리셋 기능 구현과 실행 검증은 예정입니다.

## 목표 동작

모델과 reasoning effort를 이름 있는 프리셋으로 정의하고, 현재 대화에서 단축키로 함께 전환합니다.

```toml
[tui.model_presets.sol]
model = "gpt-6.1-sol"
reasoning_effort = "xhigh"
key = "ctrl-1"

[tui.model_presets.astra]
model = "gpt-6-astra"
reasoning_effort = "high"
key = "ctrl-2"
```

위 설정은 구현 예정인 형식입니다. 모델과 effort 조합은 실행 환경의 모델 카탈로그에서 지원하는 값을 사용합니다.

- 선택은 현재 대화에만 적용하며, 저장된 기본 모델과 effort는 변경하지 않습니다.
- 대화와 입력 중인 텍스트를 유지하고, 변경된 설정은 다음 턴부터 적용합니다.
- Plan 모드에서는 기존 세션 모델 선택 경로의 모델·effort 적용 범위를 따릅니다.
- 단축키는 설정 가능하게 두고, 기존 단축키 및 프리셋 간 충돌을 검증합니다.

## 구현 방향

전환 경로는 `단축키 → 프리셋 검증 → SelectSessionModel → 기존 세션 설정 갱신`으로 구성합니다.

- [TUI 설정](codex-rs/config/src/types.rs)에 프리셋 정의를 추가하고 기존 로컬 설정 로딩 경로로 전달합니다.
- 기존 키 설정 파서와 TUI 키 바인딩·충돌 검사를 재사용합니다.
- [reasoning 단축키](codex-rs/tui/src/chatwidget/reasoning_shortcuts.rs)의 입력 가드를 따릅니다. 팝업·모달, 시작 준비 상태, 부모 에이전트의 입력 소유권과 단축키 처리 후 종료 힌트 정리를 포함합니다.
- 대상 모델과 지원 effort를 확인한 뒤 [SelectSessionModel 처리](codex-rs/tui/src/app/model_defaults.rs)에 연결합니다. 모델 전환이나 기본 설정 저장 경로를 새로 만들지 않습니다.
- 미지원 조합은 알림으로 처리하며, 임의의 모델·effort로 대체하지 않습니다. Ultra 등 기존 고급 reasoning 선택 제약도 유지합니다.

## 구현 계획

- [x] 기존 모델 선택, reasoning 단축키, 설정 및 테스트 경로 검토
- [ ] 프리셋 설정 타입·로딩·스키마 추가 — TOML 파싱과 기존 설정 호환성 검증
- [ ] 키 바인딩과 충돌 검사 연결 — 잘못된 키, 중복 키, 기존 바인딩 및 연속 키 조합(chord)의 접두사 충돌 검증
- [ ] 프리셋 단축키를 세션 선택 경로에 연결 — 입력 가드와 모델·effort 검증
- [ ] 회귀 테스트 추가 및 실행 — 세션 범위, 기본값 보존, Plan 모드, 전환 시점 검증
- [ ] 실제 터미널에서 단축키 전환 확인 — 실행 환경과 재현 절차 기록
- [ ] 구현 요약과 검증 결과를 관련 이슈에 공유

## 검증 범위

| 대상 | 확인할 내용 |
| --- | --- |
| 설정 | 프리셋 미설정 시 기존 동작 유지, 정상 TOML 파싱, 잘못된 설정 진단 |
| 키 입력 | 키 정규화와 충돌 검사, 팝업·모달 및 입력 소유권 가드, 작성 중인 텍스트 보존 |
| 모델 선택 | 지원 조합 전환, 미지원 조합 거부, 기존 reasoning 제약 유지 |
| 세션 범위 | 현재 대화만 변경, 다른 대화와 새 대화의 기본값 유지, `config.toml` 불변 |
| 모드·시점 | Default/Plan 모드의 적용 범위, 진행 중 턴 유지, 다음 턴에 변경 반영 |
| 터미널 | 실제 Ctrl+숫자 입력 전달과 전환 결과 확인 |

[기존 세션 선택 테스트](codex-rs/tui/src/app/tests/model_defaults_tests.rs)와 [턴 설정 테스트](codex-rs/core/tests/suite/step_settings.rs)를 기준으로 회귀 범위를 잡습니다. 실행 후에는 사용한 커밋, Rust 버전, OS·터미널, 테스트 명령과 결과를 기록합니다.

빌드 절차는 [Installing & building](docs/install.md), 저장소 지정 툴체인은 [rust-toolchain.toml](codex-rs/rust-toolchain.toml)을 참고하세요.

## 원본 프로젝트 · 라이선스

- [OpenAI Codex](https://github.com/openai/codex) · [공식 문서](https://developers.openai.com/codex)
- [원본 기여 지침](docs/contributing.md)
- [Apache-2.0 License](LICENSE) · [NOTICE](NOTICE)
