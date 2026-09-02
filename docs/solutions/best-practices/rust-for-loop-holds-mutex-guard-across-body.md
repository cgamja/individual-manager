---
title: "`for x in <락>.f()`는 가드를 루프 내내 붙들어 자기 데드락을 만든다"
module: "src-tauri/src/pet_bridge (커맨드 · 20Hz 틱)"
date: 2026-09-02
problem_type: best_practice
component: tooling
severity: critical
root_cause: scope_issue
resolution_type: code_fix
applies_when:
  - "`#[tauri::command]`를 새로 쓰면서 여러 마리를 훑을 때"
  - "루프 본문이 `flush`/`flush_ball`처럼 같은 락을 다시 잡는 함수를 부를 때"
  - "`Mutex<T>`에서 꺼낸 값을 `for`의 반복자 식에 바로 쓸 때"
symptoms:
  - "설정 창의 버튼을 누르면 앱이 통째로 멈춘다 (펭귄 전부 정지)"
  - "오류도 패닉도 로그도 없다 — 그냥 멈춘다"
  - "두 러너와 타입 검사, 번들 빌드가 전부 초록이다"
tags:
  - rust
  - mutex
  - deadlock
  - tauri
  - silent-failure
  - temporary-lifetime
---

# `for x in <락>.f()`는 가드를 루프 내내 붙들어 자기 데드락을 만든다

## 증상

볼링 기능을 붙이고 설정 창의 **"볼링 한 판"을 누르면 앱이 통째로 멈췄다.**
펭귄이 전부 그 자리에 굳고 다시는 움직이지 않는다. 오류 대화상자도, 패닉도,
stderr 한 줄도 없다. 트레이 메뉴의 종료만 듣는다.

**신호가 하나도 없는 것이 이 버그의 성격이다.** `cargo test` 264개, `npm test`,
`npm run build`, `npm run tauri build`가 전부 통과한 상태로 커밋됐다.

## 원인

문제의 코드는 이것뿐이다.

```rust
// src-tauri/src/pet_bridge/commands.rs — bowling_start
for id in state.pets.lock().unwrap().ids() {
    flush(&app, id);   // flush 안에서 같은 pets 락을 다시 잡는다
}
```

`ids()`는 `Vec<PetId>`를 **소유해서** 돌려주므로 가드가 필요 없어 보인다. 그런데
Rust의 `for`는 이렇게 펼쳐진다.

```rust
match IntoIterator::into_iter(state.pets.lock().unwrap().ids()) {
    mut iter => loop { /* 본문 */ }
}
```

`match`의 **주사식(scrutinee)에서 생긴 임시값은 `match` 전체가 끝날 때까지 산다.**
`MutexGuard`가 바로 그 임시값이라, 가드는 루프 본문이 다 돌 때까지 살아 있다.
그 안에서 `flush`가 `state.pets.lock()`을 다시 부르고, **`std::sync::Mutex`는
재진입이 안 되므로** 자기 자신을 기다리며 영원히 멈춘다.

멈추는 것이 IPC 스레드 하나로 끝나지 않는다. 락을 쥔 채 멈췄으므로 20Hz 틱
스레드도 다음 `step_all`에서 같은 락을 기다리다 멈춘다 — 그래서 "버튼 하나
눌렀는데 펭귄 전부가 굳는다".

## 안 통한 것 — 정확히는, 아무것도 잡지 못했다

이 함정의 값비싼 점은 **평소에 믿는 그물이 전부 통과시킨다**는 것이다.

- **`cargo test` 264개** — 코어 테스트는 `Pets::start_bowling`을 직접 부른다.
  커맨드 본문(`bowling_start`)을 실행하는 테스트가 하나도 없다. `AppHandle` 없이
  커맨드를 부를 방법이 이 레포에 없어서, 브릿지 커맨드는 통째로 수동 스모크에
  맡겨져 있다 (핀볼도 같다).
- **커맨드 등록 대조 테스트** — `모든_펫_커맨드가_invoke_handler에_등록되어_있다`는
  소스를 **텍스트로** 훑어 등록 여부만 본다. 본문은 읽지 않는다.
- **컴파일러·clippy 기본 린트** — 경고 한 줄 없다. 타입은 완전히 맞는다.
- **`npm run build`·번들 빌드** — Rust 런타임 동작과 무관하다.

잡은 것은 **코드 리뷰**였다. 그리고 리뷰의 지적을 그대로 믿지 않고 **따로 재현
프로그램을 만들어** 확인했다 — 이 확인이 없었으면 "for 루프의 임시값 수명"이라는
설명이 맞는지 아닌지 판단할 근거가 없었다.

```rust
// 30줄짜리 독립 재현. rustc로 바로 돌아간다.
use std::sync::Mutex;
struct S { pets: Mutex<Vec<u32>> }
fn flush(s: &S, _id: u32) { let _g = s.pets.lock().unwrap(); }
fn main() {
    let s = S { pets: Mutex::new(vec![1, 2, 3]) };
    for id in s.pets.lock().unwrap().clone() {   // ← 여기서 멈춘다
        flush(&s, id);
    }
}
```

## 해결

**id를 먼저 지역 변수로 꺼내 가드를 떨군다.** `let` 문의 임시값은 그 문장이
끝날 때 사라지므로, 루프에 들어갈 때는 락이 이미 풀려 있다.

```rust
let ids = state.pets.lock().unwrap().ids();
for id in ids {
    flush(&app, id);
}
```

이 레포에는 **이미 올바른 모양이 두 군데 있었다** — `commands.rs`의
`pet_set_pinball`(클로저가 `Vec`를 반환하고 그걸 받아서 순회)과 `window.rs`의
`close_all_pet_windows`(`let ids = ...` 먼저). 그래서 지금까지 안 터졌던 것이지,
안전한 설계여서가 아니었다.

## 왜 이게 통하나

두 자리의 임시값 수명이 다르다.

| 쓰는 모양 | 가드가 사는 범위 |
|---|---|
| `let ids = <락>.ids();` | **그 `let` 문 끝까지** — 다음 줄에서는 이미 풀렸다 |
| `for id in <락>.ids() { }` | **루프 전체** — `match` 주사식의 임시값이라 본문이 끝나야 죽는다 |

`ids()`가 소유값을 돌려준다는 사실은 여기에 아무 영향이 없다. 사는 것은 반환값이
아니라 **그 값을 만드는 데 쓰인 임시 가드**다.

## 예방

- **`for x in <락 표현식>` 모양을 금지 패턴으로 본다.** 락에서 뭔가를 꺼내
  순회할 거면 무조건 `let`으로 한 번 받는다. 반환 타입이 소유값이어도 그렇다.
  같은 함정이 `if let`·`match`·`while let`의 주사식에도 있다.
- **새 커맨드를 쓸 때 "본문이 락을 다시 잡는 함수를 부르는가"를 먼저 본다.**
  이 레포에서 그런 함수는 `flush`와 `flush_ball` 둘이고, 커맨드 대부분이 마지막에
  이 둘 중 하나를 부른다. 즉 **커맨드 안에서 락을 잡은 채로 할 수 있는 일은 거의
  없다** — 상태를 고치고 곧바로 가드를 놓는 것이 이 레포의 규약이다.
- **브릿지 커맨드는 테스트가 안 잡는다는 것을 전제로 리뷰한다.** 코어(`src-tauri/src/pet/`)는
  테스트가 촘촘하지만 커맨드 본문은 어느 러너도 실행하지 않는다. 두 러너가 초록인
  것은 커맨드가 맞다는 근거가 **전혀** 아니다.
- **리뷰 지적을 재현으로 확인한다.** 임시값 수명처럼 "그럴 것 같다"로는 갈릴 수
  있는 사안은 20~30줄짜리 독립 프로그램이 가장 싸게 답을 준다.

관련: [커맨드 등록 누락은 컴파일·테스트·경고를 전부 통과한다](tauri-command-registration-silent-failure.md) —
증상이 없는 실패라는 점, 그리고 잡아 주는 것이 테스트가 아니라 대조라는 점이 같다.
