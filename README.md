# Codex CLI: 모델 · Reasoning 프리셋

[openai/codex](https://github.com/openai/codex)를 기반으로, TUI에서 모델과 reasoning effort를 프리셋 단축키로 전환하는 기능을 실험하는 비공식 포크입니다.

![모델 및 reasoning effort 프리셋 전환 데모](docs/assets/model-presets-demo.gif)

단축키로 프리셋을 전환하고 `/status`로 적용된 모델과 effort를 확인하는 모습입니다.

관련 이슈: [Model + Reasoning presets/profiles #44603](https://github.com/openai/codex/issues/44603). 해당 이슈는 데스크톱 앱 대상이며, 이 포크에서는 CLI/TUI의 구현 가능성과 동작을 검증합니다. 구현과 테스트 결과를 이슈의 설계 논의에 공유하는 것이 목표입니다.

**현재 상태:** 프리셋 구현, 관련 자동 테스트, Windows 빌드와 실제 터미널의 단축키 전환 검증 완료. Ctrl+1·2·3 전환과 세션·터미널 재시작 후 기본 모델 유지를 확인했습니다.

## 프리셋 설정과 동작

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

설정은 `config.toml`에 추가한 뒤 TUI를 다시 실행합니다. 개발용 스크립트에서는 `%LOCALAPPDATA%\CodexModelPresetsDev\home\config.toml`을 사용합니다. 모델과 effort 조합은 실행 환경의 모델 카탈로그에서 지원하는 값을 사용합니다.

같은 형식으로 `[tui.model_presets.<이름>]` 항목을 추가해 프리셋을 늘릴 수 있습니다.

- 선택은 현재 대화에만 적용하며, 저장된 기본 모델과 effort는 변경하지 않습니다.
- 대화와 입력 중인 텍스트를 유지하고, 변경된 설정은 다음 턴부터 적용합니다.
- Plan(계획) 모드에서도 모델을 현재 대화에 적용하며, effort는 Plan 모드 설정에 적용합니다.
- 단축키는 설정 가능하게 두고, 기존 단축키 및 프리셋 간 충돌을 검증합니다.

일반(Default) 모드와 Plan 모드는 effort를 별도로 관리합니다. 예를 들어 일반 모드가 `medium`일 때 Plan 모드에서 `high` 프리셋을 선택하면 Plan의 effort는 `high`가 되고, 일반 모드의 `medium`은 유지됩니다. 세션에서 임시로 선택한 Ultra를 해제하는 경우에는 기존 세션 선택 경로에 따라 다른 모드의 effort도 함께 갱신될 수 있습니다.

`ultra`는 reasoning effort 값입니다. 현재 Ultra 프리셋은 단축키로 적용하지 않으며, 해당 모델이 지원하는 경우 `/model`의 고급 reasoning 선택 화면에서 직접 선택해야 합니다.

현재 프리셋의 `key`는 한 번 누르는 키 조합을 지원합니다. 프리셋에 연속 키 조합(chord)을 지정하면 거부하며, 기존 chord의 첫 키와 충돌하는 프리셋도 거부합니다. 일반 문자 입력과 AltGr 입력에 쓰이는 키는 지정할 수 없습니다.

## 알려진 제약

- 고정 단축키의 일부 별칭(`alt-v`, `ctrl-shift-d` 등)이 충돌 검사에서 누락됩니다. 프리셋으로 지정하면 기존 붙여넣기·종료 동작과 충돌하거나 화면에 따라 다르게 처리될 수 있습니다.
- Unix에서 키 이벤트 확장을 지원하지 않는 터미널의 Vim 입력 모드에서는 Alt 프리셋이 `Esc`와 문자 입력으로 처리될 수 있습니다. 예를 들어 `alt-x`가 전환 대신 작성 중인 문자를 삭제할 수 있습니다. 이 문제는 소스 경로로 확인했으며 Unix에서의 실행 검증은 하지 않았습니다.

Ctrl+숫자는 터미널에서 해당 키 이벤트를 전달해야 동작합니다. 터미널이 가로채거나 구분해서 전달하지 않는 경우 `key`를 다른 조합으로 바꿔 확인합니다.

## 구현 방식

전환 경로는 `단축키 → 프리셋 검증 → SelectSessionModel → 기존 세션 설정 갱신`으로 구성합니다.

- [TUI 설정](codex-rs/config/src/types.rs)에 프리셋 정의를 추가하고 기존 로컬 설정 로딩 경로로 전달합니다.
- 기존 키 설정 파서와 TUI 키 바인딩·충돌 검사를 재사용합니다.
- [reasoning 단축키](codex-rs/tui/src/chatwidget/reasoning_shortcuts.rs)의 입력 가드를 따릅니다. 팝업·모달, 시작 준비 상태, 부모 에이전트의 입력 소유권과 단축키 처리 후 종료 힌트 정리를 포함합니다.
- 대상 모델과 지원 effort를 확인한 뒤 [SelectSessionModel 처리](codex-rs/tui/src/app/model_defaults.rs)에 연결합니다. 모델 전환이나 기본 설정 저장 경로를 새로 만들지 않습니다.
- 미지원 조합은 알림으로 처리하며, 임의의 모델·effort로 대체하지 않습니다.

## 진행 상태

- [x] 기존 모델 선택, reasoning 단축키, 설정 및 테스트 경로 검토
- [x] 프리셋 설정 타입·로딩 추가 — TOML 파싱과 기존 설정 호환성 검증
- [x] 설정 스키마 갱신과 생성 결과 확인
- [x] 키 바인딩과 충돌 검사 연결 — 잘못된 키, 중복 키, 기존 바인딩 및 연속 키 조합(chord)의 접두사 충돌 검증
- [x] 프리셋 단축키를 세션 선택 경로에 연결 — 입력 가드와 모델·effort 검증
- [x] 회귀 테스트 추가 및 실행 — 세션 범위, 기본값 보존, Plan 모드, 작업 중 입력 유지 검증
- [x] 실제 터미널에서 Ctrl+1·2·3 전환과 세션·터미널 재시작 후 기본 모델 유지 확인

## 검증

| 대상 | 확인할 내용 |
| --- | --- |
| 설정 | 프리셋 미설정 시 기존 동작 유지, 정상 TOML 파싱, 잘못된 설정 진단 |
| 키 입력 | 키 정규화와 충돌 검사, 팝업·모달 및 입력 소유권 가드, 작성 중인 텍스트 보존 |
| 모델 선택 | 지원 조합 전환, 미지원 조합 거부, 기존 reasoning 제약 유지 |
| 세션 범위 | 현재 대화만 변경, 다른 대화와 새 대화의 기본값 유지, `config.toml` 불변 |
| 모드·시점 | Default/Plan 모드의 적용 범위, 진행 중 턴 유지, 다음 턴에 변경 반영 |
| 터미널 | 실제 Ctrl+숫자 입력 전달과 전환 결과 확인 |

[세션 선택 테스트](codex-rs/tui/src/app/tests/model_defaults_tests.rs)는 내장 app-server를 사용해 Default/Plan 모드의 현재 대화 변경, 다른 대화·새 대화의 기본값 유지, `config.toml` 불변을 검증합니다. 진행 중인 요청의 실행 모델은 기존 `SelectSessionModel`의 다음 턴 적용 계약을 따릅니다. 실제 요청을 스트리밍하는 상태에서의 전환은 이번 자동 테스트에 포함하지 않았습니다.

프리셋 자동 테스트는 Visual Studio 개발자 PowerShell(x64)의 `codex-rs` 디렉터리에서 실행합니다. Windows의 내장 서버 테스트에는 저장소 CI에서 사용하는 8 MiB 스택 설정을 적용합니다.

```powershell
$env:RUSTUP_AUTO_INSTALL = "0"
$env:RUST_MIN_STACK = "8388608"
cargo test --locked -p codex-config --lib model_presets --target x86_64-pc-windows-msvc
cargo test --locked -p codex-tui --lib model_presets --target x86_64-pc-windows-msvc -- --test-threads=1
```

2026-10-02 검증 결과: 기준 커밋 `f9bccd006`에 작업 중 변경 사항을 적용한 상태, Rust `1.95.0`, Windows x64 MSVC.

| 검증 | 결과 |
| --- | --- |
| `codex-config`의 `model_presets` 테스트 | 3개 통과 |
| `codex-tui`의 `model_presets` 테스트 | 12개 통과 |
| 기존 `keymap::` 테스트 | 132개 통과 (프리셋 키 테스트 포함) |
| 기존 `session_model_selection` 테스트 | 4개 통과 |
| 기존 `reasoning_` 테스트 | 116개 통과, 버전 표시 차이로 스냅샷 2개 실패 |
| 설정 스키마 | 생성기로 갱신, 프리셋의 필수 필드와 참조 확인 |
| `cargo build --locked -p codex-cli --bin codex --target x86_64-pc-windows-msvc` | 성공 |
| 개발 스크립트 `-NoBuild` 시작 | 두 프리셋을 넣은 임시 설정으로 로그인 화면까지 확인 |
| 실제 터미널 단축키 (사용자 수동 검증) | Ctrl+1·2와 추가 프리셋의 Ctrl+3 전환 확인 |
| 세션·터미널 재시작 (사용자 수동 검증) | 세션 종료·시작과 터미널 재시작 후 저장된 기본 모델 유지 확인 |

실패한 두 스냅샷 테스트는 `/status` 출력의 버전 문자열이 예상값 `v0.0.0`과 실제값 `v0.160.0`으로 달라 발생했습니다. 해당 비교에서 모델·effort 등 나머지 출력은 같았습니다.

Ctrl+1/2 키 이벤트와 세션 설정 갱신은 자동 테스트로 검증했습니다. 실제 터미널에서는 사용자가 Ctrl+1·2·3 전환과 세션·터미널 재시작 후 기본 모델 유지를 직접 확인했습니다.

빌드 절차는 [Installing & building](docs/install.md), 저장소 지정 툴체인은 [rust-toolchain.toml](codex-rs/rust-toolchain.toml)을 참고하세요.

## 빌드 및 동작 확인 (Windows x64)

저장소 루트에서 [개발 스크립트](scripts/dev-model-presets.ps1)로 빌드하고 실행합니다.

```powershell
# 저장소 루트에서 증분 빌드 후 실행
powershell -File .\scripts\dev-model-presets.ps1

# 기존 바이너리로 다시 실행
powershell -File .\scripts\dev-model-presets.ps1 -NoBuild
```

PowerShell 7에서는 `powershell` 대신 `pwsh`를 사용합니다. 스크립트는 `codex` 바이너리를 빌드하고 `--no-daemon`으로 실행합니다.

설정·인증·대화 기록은 `%LOCALAPPDATA%\CodexModelPresetsDev\home`, 작업 폴더는 그 옆의 `workspace`를 사용합니다.

1. 개발용 `home\config.toml`에 위 프리셋 설정을 추가하고, `-NoBuild`로 다시 실행합니다.
2. 로그인한 대화에서 `/status` → Ctrl+1 → `/status` → Ctrl+2 → `/status` 순서로 모델과 effort를 확인합니다.
3. 다른 이름의 프리셋에 `key = "ctrl-3"`을 지정하고 다시 실행해 추가 프리셋의 전환도 확인합니다.
4. 작성 중인 입력이 유지되는지, Plan 모드에서도 적용되는지 확인합니다.
5. 세션 종료·시작과 터미널 재시작 후 저장된 기본 모델로 시작하는지, `config.toml`이 변경되지 않았는지 확인합니다.

Code Mode 및 Windows 샌드박스 도구 실행을 검증하려면 해당 보조 바이너리를 추가로 빌드해야 합니다.

## 원본 프로젝트

- [OpenAI Codex](https://github.com/openai/codex) · [공식 문서](https://developers.openai.com/codex)
