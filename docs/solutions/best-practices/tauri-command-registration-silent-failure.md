---
title: "Tauri 커맨드 등록 누락은 컴파일·테스트·경고를 전부 통과한다"
module: "src-tauri 브릿지 모듈 (invoke_handler)"
date: 2026-08-30
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "새 `#[tauri::command]`를 추가하고 `lib.rs`의 `generate_handler!`에 등록할 때"
  - "브릿지 모듈(`*_bridge.rs`)을 새로 만들 때"
  - "스크립트로 소스를 일괄 수정할 때 (문자열 치환)"
tags:
  - tauri
  - invoke
  - command
  - silent-failure
  - tooling
---

# Tauri 커맨드 등록 누락은 컴파일·테스트·경고를 전부 통과한다

## Context

M2.5(바탕화면 펭귄)에서 `pet_poke`·`pet_drag_*`·`pet_set_enabled` 등 6개 커맨드를
`pet_bridge.rs`에 만들고 `lib.rs`의 `invoke_handler`에 등록했다고 생각했는데, 실제로는
**하나도 등록되지 않은 채** 커밋·번들 빌드까지 통과했다. 코드 리뷰가 잡지 않았다면
"펭귄은 걸어다니는데 클릭도 드래그도 설정 토글도 아무 반응이 없는" 상태로 머지될 뻔했다.

## Guidance

1. **등록 누락은 어떤 자동 게이트에도 걸리지 않는다.** 커맨드 함수는 `pub`이라
   `dead_code` 경고가 뜨지 않고, `cargo check`·`cargo test`·`npm test`·`tauri build`가
   전부 통과한다. 런타임에서만 `Command <name> not found`로 reject된다.

2. **프론트가 실패를 삼키면 증상마저 사라진다.** 이 레포의 관용구인
   `void invoke(...)` / `.catch(() => null)`은 정상 경로에서는 옳지만, 등록 누락과
   만나면 콘솔 밖으로는 아무것도 드러나지 않는다. "기능이 조용히 없는" 상태가 된다.

3. **소스 일괄 수정에 `python str.replace`를 쓰지 않는다.** 이번 사고의 직접 원인이다.
   `str.replace`는 **매치가 없어도 예외 없이 원본을 그대로 돌려준다.** 들여쓰기 한 칸,
   줄바꿈 하나만 달라도 조용히 아무 일도 일어나지 않고, 스크립트는 `ok`를 출력한다.
   → 편집은 실패가 드러나는 도구(에이전트의 `Edit`)를 쓰고, 부득이 스크립트를 쓴다면
   **치환 전에 `assert old in s`로 매치를 강제한다.**

4. **등록을 테스트로 고정한다.** 소스를 직접 대조하는 값싼 테스트 하나면 이 부류가
   다시 새지 않는다 (`pet_bridge.rs`의 `모든_펫_커맨드가_invoke_handler에_등록되어_있다`).
   브릿지에서 `#[tauri::command]` 뒤의 `pub fn 이름`을 모아 `lib.rs` 본문과 대조한다.

5. **capabilities도 같은 성격의 조용한 실패다.** 새 창을 만들면
   `capabilities/default.json`의 `windows`에 라벨을 추가해야 `core:default`가 적용된다.
   빠뜨리면 이벤트가 오지 않는데 오류도 나지 않는다.

## Why This Matters

Tauri에서 "브릿지를 추가한다"는 작업은 **한 곳에서 정의하고 다른 곳에서 등록하는**
구조라 두 파일이 어긋날 수 있는데, 그 어긋남을 언어도 빌드도 검사해 주지 않는다.
M3~M6에서 Jira·Calendar·Slack·Webex 브릿지를 같은 모양으로 계속 추가할 예정이라
재발 확률이 높다. 그리고 증상이 "런타임에서 조용히 아무 일도 안 일어남"이라
디버깅 진입점조차 없다.

## When to Apply

- 새 `#[tauri::command]`를 하나라도 추가할 때 (등록 + 테스트 갱신을 같은 커밋에)
- 새 브릿지 모듈이나 새 창을 추가할 때 (capabilities 포함)
- 소스를 스크립트로 고칠 때 — 언제나

## Examples

등록을 지키는 테스트 (`src-tauri/src/pet_bridge.rs`):

```rust
#[test]
fn 모든_펫_커맨드가_invoke_handler에_등록되어_있다() {
    let bridge = include_str!("pet_bridge.rs");
    let lib = include_str!("lib.rs");
    // `#[tauri::command]` 다음 줄의 `pub fn 이름(`을 모아 lib.rs와 대조한다
    for name in commands_in(bridge) {
        assert!(
            lib.contains(&format!("pet_bridge::{name},")),
            "`{name}`이 lib.rs의 invoke_handler 목록에 없다"
        );
    }
}
```

스크립트 치환을 쓸 수밖에 없을 때:

```python
old = '...'
assert old in s, f'매치 실패: {label}'   # 이 줄이 없으면 조용히 아무 일도 안 한다
s = s.replace(old, new, 1)
```
