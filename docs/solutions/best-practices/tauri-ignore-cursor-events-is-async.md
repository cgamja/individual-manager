---
title: "`set_ignore_cursor_events`는 비동기다 — 호출 직후에 읽으면 `false`다"
module: "src-tauri/src/pet_bridge (그림만 그리는 창)"
date: 2026-09-02
problem_type: best_practice
component: ui
severity: high
root_cause: async_timing
resolution_type: code_fix
applies_when:
  - "클릭을 통과시켜야 하는 창(오버레이·장식·HUD)을 새로 만들 때"
  - "`WebviewWindow`의 세터를 걸고 곧바로 적용됐는지 확인하려 할 때"
  - "`always_on_top` 창에서 `ignoresMouseEvents`가 안 먹는 것처럼 보일 때"
symptoms:
  - "`set_ignore_cursor_events(true)`가 `Ok(())`인데 직후에 읽으면 `false`다"
  - "그래서 '이 API는 always_on_top 창에서 안 먹는다'는 결론에 도달한다"
  - "실제로는 2초쯤 뒤에 읽으면 `true`다 — 창은 처음부터 정상이었다"
  - "창이 뜬 직후 한 프레임쯤 오버레이가 클릭을 먹는다"
tags:
  - tauri
  - macos
  - window
  - async
  - silent-failure
  - ignore-cursor-events
---

# `set_ignore_cursor_events`는 비동기다 — 호출 직후에 읽으면 `false`다

## 증상

비치발리볼의 코트 창(모래사장 + 네트)은 **그림뿐이라 클릭을 통과시켜야 한다.**
핀볼 판이 클릭을 먹는 근거는 "커서가 어디서나 채가 된다"였고 그래서 나가는 문이
둘 필요했는데, 코트에는 그 근거가 없다 — 먹으면 "방해하지 않는다"(PRINCIPLE 5)를
근거 없이 어긴다.

이 레포에 `set_ignore_cursor_events` 선례가 없어 스파이크로 확인했다. 화면 전체를
덮는 **투명 + `always_on_top`** 창을 만들고 플래그를 건 **직후에** `NSWindow`의
`ignoresMouseEvents`를 읽었더니 `false`였다. 반환값은 `Ok(())`고 stderr도 깨끗하다.

**결론이 "이 API는 이 조합에서 안 먹는다"로 나왔고, 그건 오답이었다.**

## 안 통한 시도

- **호출 직후에 읽어서 확인했다 — 이게 오답의 원인이다.** `set_ignore_cursor_events(true)`
  바로 다음 줄에서 `ns_window()`의 `ignoresMouseEvents`를 읽으면 `false`가 나온다.
  반환값도 `Ok`, 로그도 깨끗해서 "걸리지 않았다"고 읽힌다. **이 레포가 문서화해 온
  다른 조용한 실패들과 정확히 같은 모양이라** 더 그럴듯해 보였다.
- **`always_on_top`이 플래그를 덮어쓰는 줄 알고 순서를 바꿔 봤다.** 플래그를 건 뒤
  `set_always_on_top(true)`를 다시 걸어도 `ignoresMouseEvents`는 그대로 `true`다.
  범인이 아니었다.
- **AppKit으로 직접 `setIgnoresMouseEvents:`를 걸어 봤다.** 결과가 같다 —
  Tauri 쪽 래퍼의 문제가 아니라는 뜻이지만, 그것만으로는 원인이 안 보인다.

## 원인

**이 세터는 비동기다.** `Ok(())`를 즉시 돌려주지만 실제 적용은 이벤트 루프를
한 번 왕복한 뒤에 일어난다. `set_position`이 어느 스레드에서 불러도 안전한 것과
정확히 같은 성질이다 — tauri-runtime-wry가 메인 스레드가 아니면 이벤트 루프로
넘기고, tao의 macOS 구현이 다시 메인 스레드로 디스패치한다.

같은 창에서 시간을 두고 잰 값이다.

```text
set_ignore_cursor_events(true) -> Ok(())
즉시:                ignoresMouseEvents=false   level=5
2초 뒤:              ignoresMouseEvents=true    level=5
다시 호출 후:         ignoresMouseEvents=true    level=5
AppKit 직접 설정 후:  ignoresMouseEvents=true    level=5
```

레벨은 내내 안 변한다 — `always_on_top`이 플래그를 되돌리지 않는다는 뜻이다.
**API는 처음부터 정상이었고, 틀린 것은 확인 방법이었다.**

## 해결

**두 가지다.**

**1. 적용됐는지 읽어서 확인하는 코드를 호출 직후에 두지 않는다.** 확인이 필요하면
사람이 실제로 클릭해 보는 것이 유일하게 믿을 만한 검증이다. 자동 검증을 넣고
싶으면 이벤트 루프를 한 번 이상 보낸 뒤에 읽어야 하는데, 그 지연이 얼마인지는
보장되지 않으므로 **그런 검증은 아예 안 쓰는 편이 낫다.**

**2. 창을 안 보이게 만들고 → 플래그를 걸고 → 보인다.**

```rust
let window = WebviewWindowBuilder::new(app, label, url)
    // ...
    .visible(false)   // 먼저 안 보이게 만든다
    .build()?;
window.set_ignore_cursor_events(true)?;   // 플래그를 걸고
window.show()?;                            // 그다음 보인다
```

둘 다 같은 이벤트 루프를 **순서대로** 지나므로 이게 "클릭을 먹는 창이 화면에
떠 있는 구간"을 가장 좁힌다.

**그래도 한 프레임쯤은 남을 수 있다.** 세터가 비동기인 이상 완전히 없앨 수는
없고, 그 한 프레임을 감수하는 근거는 이렇다: 코트가 뜨는 순간 포인터는 방금
버튼을 누른 **설정 창 위**에 있지 코트 위가 아니고, 코트는 20초 뒤 사라진다.
이 한 프레임을 없애자고 창 생성을 지연시키면 "버튼을 눌렀는데 아무 일도 안
일어나는" 구간이 대신 생긴다 — 그쪽이 더 나쁘다.

**CSS `pointer-events: none`은 이 구간을 못 메운다 — 이중 방어가 아니다.**
그건 웹뷰가 클릭에 **반응하지 않게** 할 뿐이고, 네이티브 창은 여전히 클릭을
**먹는다**(아래 앱으로 안 내려간다). 클릭 통과의 메커니즘은 아래 절대로
`NSWindow.ignoresMouseEvents` 하나뿐이라, 세터가 적용되기 전 한 프레임 동안은
CSS가 있어도 클릭이 그대로 먹힌다. 그래도 CSS를 두는 이유는 다른 데 있다 —
커서 모양·텍스트 선택·hover 같은 **웹뷰 자체의 반응**을 없앤다.

## 왜 이게 통하는가

macOS에서 클릭 통과는 `NSWindow.ignoresMouseEvents`가 곧 메커니즘이다. 그 프로퍼티가
`true`이면 그 창은 히트 테스트에서 통째로 빠지고, 클릭은 그 아래 창(다른 앱이든
바탕화면이든)으로 그대로 내려간다. `always_on_top`은 **레벨**만 정하지 히트 테스트에
관여하지 않으므로 둘은 서로 간섭하지 않는다.

## 예방책

- **창 관련 세터를 걸고 곧바로 읽어서 확인하지 않는다.** `set_position`·
  `set_size`·`set_ignore_cursor_events`가 전부 같은 성질이다.
- **클릭 통과와 창 레벨은 서로를 대신하지 못한다.** 코트는 펭귄보다 아래여야 하고
  (`ns_window()`로 레벨을 내린다), 그것과 **별개로** 클릭을 통과시켜야 한다 —
  레벨만 내리면 한 번 클릭했을 때 도로 올라온다
  (`docs/solutions/ui-bugs/macos-window-order-is-not-stable-level-is.md`).
- **그림만 그리는 창의 플래그는 한 함수에 몰아 둔다** (`pet_bridge/volleyball.rs`의
  `그림_창`). 창마다 따로 쓰면 한쪽만 클릭을 먹게 되는 갈래가 생긴다.
- 이 결함은 **단위 테스트로 안 잡힌다** — 창 생성은 Tauri 런타임 표면이다.
  스모크에서 **코트 위에서 다른 앱을 클릭해** 확인한다.
