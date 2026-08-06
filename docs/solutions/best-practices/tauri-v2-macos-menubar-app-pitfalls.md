---
title: "Tauri v2 macOS 메뉴바 상주 앱의 함정과 검증된 대응"
module: "src-tauri 메뉴바 셸 · 타이머 코어"
date: 2026-08-06
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - "Tauri v2로 macOS 메뉴바 상주 앱(트레이 + 팝오버)을 만들 때"
  - "숨겨진 웹뷰 상태에서도 타이머·주기 작업이 정확해야 할 때"
  - "macOS 알림을 붙이거나 dev 모드에서 알림을 테스트할 때"
  - "M2+ 마일스톤에서 이 셸 위에 새 기능을 얹을 때"
tags:
  - tauri
  - macos
  - menubar
  - tray-icon
  - wkwebview
  - notification
  - positioner
---

# Tauri v2 macOS 메뉴바 상주 앱의 함정과 검증된 대응

## Context

M1(메뉴바 앱 골격 + 뽀모도로)을 Tauri v2 + React/TS로 구현하면서 리서치로 예측하고 구현으로 검증한 함정들. 대부분 컴파일·단위 테스트로는 잡히지 않는 OS 통합 표면이라, 모르면 런타임에서 원인 불명 증상으로 나타난다. M2+에서 이 셸을 확장할 때 다시 밟지 않도록 기록한다.

## Guidance

1. **숨겨진 웹뷰의 JS 타이머는 믿지 않는다.** WKWebView는 숨겨진 웹뷰의 타이머를 ~5분 뒤 정지시킨다(wry#1246, tauri#5250 — 리서치 출처). 대응: 시간의 진실 원천은 타임스탬프(`end_time - now`)로 두고, 권위 있는 1Hz 틱은 Rust 스레드가 소유한다(`src-tauri/src/timer_bridge.rs`의 `spawn_tick_thread`). 보조로 창 설정에 `"backgroundThrottling": "disabled"`(macOS 14+)를 건다. 감산(`setInterval`로 1초씩 빼기) 방식은 금지.

2. **트레이는 `setup()`에서 동기로 생성한다.** 비동기 컨텍스트에서 만든 트레이는 마우스 이벤트를 받지 못하는 알려진 버그가 있다(tauri#11462 — 리서치 출처). `src-tauri/src/lib.rs`의 setup 클로저에서 `TrayIconBuilder`를 바로 호출한다. `on_tray_icon_event` 안에서는 `tauri_plugin_positioner::on_tray_event`를 **항상 먼저** 호출해야 `TrayCenter` 좌표가 계산된다 — 빠뜨리면 조용히 실패한다.

3. **blur 숨김과 트레이 클릭 토글은 경쟁한다.** 팝오버가 열린 채 트레이를 클릭하면 blur 숨김이 먼저 처리되고 클릭 이벤트가 뒤에 도착해 "닫기"가 "다시 열기"가 된다. 대응: blur 숨김 시각을 기록하고 ~300ms 이내의 트레이 클릭은 닫기 의도로 간주해 재표시하지 않는다(`src-tauri/src/lib.rs`의 `TOGGLE_RACE_WINDOW_MS`).

4. **투명 창은 `macOSPrivateApi` 없이는 흰 배경이 된다.** `transparent: true`만으로는 부족하고 `tauri.conf.json`의 `app.macOSPrivateApi: true` + Cargo `tauri` 의존성의 `macos-private-api` feature가 전제 조건이다.

5. **`LSUIElement`는 tauri.conf.json 키가 아니다.** Dock 숨김의 번들 설정은 `src-tauri/Info.plist` 파일로 두면 빌드 시 병합된다. 런타임 쪽은 setup에서 `set_activation_policy(ActivationPolicy::Accessory)` — 이건 dev 모드에서도 동작한다. Accessory 상태에서 팝오버에 키보드 포커스를 주려면 `app.show()` → `window.set_focus()` 순서가 필요하다. 단, **숨길 때 `app.hide()`는 호출하지 않는다** — macOS 26에서 트레이 아이콘까지 사라진다. 상세: [macOS 26(Tahoe) app.hide() 트레이 아이콘 소실](../ui-bugs/macos-tahoe-app-hide-removes-tray-icon.md).

6. **macOS 알림은 dev 모드에서 오지 않는 것이 정상이다.** `tauri dev` 바이너리는 번들 ID가 등록된 앱이 아니라 UNUserNotificationCenter가 거부한다(plugins-workspace#2143 — 리서치 출처). 알림 검증은 `tauri build`로 만든 `.app`(ad-hoc 서명으로 충분)에서만 한다. 알림은 웹뷰가 숨겨져 있어도 나가야 하므로 Rust 쪽에서 발송한다(`timer_bridge.rs`의 `notify_finished`).

7. **1Hz 틱과 사용자 커맨드는 만료 시점에서 경쟁한다.** 벽시계 만료와 다음 틱 사이(최대 1초)에 pause가 들어오면 "Paused 00:00"으로 고착되고 종료 이벤트·알림이 유실된다. 대응: 상태를 바꾸는 커맨드(pause/resume)는 먼저 `poll(now)`로 만료를 정산하고, 정산 경로(`settle_finished`)를 틱 스레드와 공유한다(`timer_bridge.rs`).

8. **트레이 조작은 메인 스레드로 위임한다.** 틱 스레드에서 `set_title`을 직접 부르지 않고 `run_on_main_thread`로 감싼다(`timer_bridge.rs`의 `refresh_tray`).

## Why This Matters

이 함정들은 전부 "코드는 맞아 보이는데 런타임에서 조용히 어긋나는" 부류다 — 타이머가 5분 뒤 멈추고, 트레이가 클릭에 반응하지 않고, 알림이 안 오고, 아이콘이 사라진다. 하나하나가 디버깅 세션 하나 분량이며, 단위 테스트가 잡아주지 못한다. M2~M6에서 폴링·알림·새 카드가 이 셸 위에 계속 얹히므로 재발 가능성이 높다.

## When to Apply

- 이 레포에서 메뉴바 셸(`src-tauri/src/lib.rs`)이나 브릿지(`timer_bridge.rs`)를 수정할 때
- M2+에서 폴링 루프·알림 이벤트를 추가할 때 (1·6·7·8번이 특히 해당)
- 다른 프로젝트에서 Tauri v2 메뉴바 앱을 새로 시작할 때 (전 항목)

## Examples

만료 정산 공유 경로 (7번 항목, `src-tauri/src/timer_bridge.rs`):

```rust
// 커맨드: 만료를 먼저 정산하고, 정산되지 않았을 때만 pause한다
let finished = {
    let mut pomodoro = state.0.lock().unwrap();
    let finished = pomodoro.poll(now);
    if finished.is_none() {
        pomodoro.pause(now);
    }
    finished
};
settle_finished(&app, finished); // 틱 스레드와 같은 emit + notify 경로
```

수동 검증 체크리스트 (OS 통합 표면 — 자동화 불가 항목):

- 트레이 클릭 토글 / 바깥 클릭 숨김 / **닫은 뒤 트레이 아이콘 유지**
- 팝오버 닫고 5분 이상 방치 후 남은 시간 정확성
- 번들 앱에서 세션 종료 알림 도착
