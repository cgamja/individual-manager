---
title: "macOS 26(Tahoe)에서 app.hide() 호출 시 메뉴바 트레이 아이콘까지 사라짐"
module: "src-tauri 메뉴바 셸"
date: 2026-08-06
problem_type: ui_bug
component: tooling
severity: high
symptoms:
  - "팝오버를 닫는(blur) 순간 메뉴바 트레이 아이콘이 사라짐"
  - "앱 프로세스는 계속 실행 중 — 크래시 리포트·시스템 로그 모두 깨끗함"
  - "앱을 재실행하면 아이콘이 돌아오지만, 팝오버를 다시 닫으면 또 사라짐"
root_cause: wrong_api
resolution_type: code_fix
related_components:
  - "tray-icon"
  - "popover-window"
tags:
  - tauri
  - macos-tahoe
  - tray-icon
  - menubar
  - nsapp-hide
  - popover
  - accessory-policy
---

# macOS 26(Tahoe)에서 app.hide() 호출 시 메뉴바 트레이 아이콘까지 사라짐

## Problem

Tauri v2 메뉴바 상주 앱(Penguin)에서 팝오버 창을 숨길 때 `app.hide()`를 함께 호출하면, macOS 26(Tahoe, 26.5.2에서 확인)에서 메뉴바의 트레이 아이콘 자체가 사라진다. 앱은 계속 실행 중이지만 사용자가 앱에 접근할 방법이 없어진다.

## Symptoms

- 첫 실행 시 메뉴바에 트레이 아이콘이 정상 표시되고 클릭하면 팝오버도 열림
- 바깥을 클릭해 팝오버가 닫히는 순간(blur → hide 경로) 트레이 아이콘이 함께 사라짐
- `pgrep`으로 확인하면 프로세스는 살아 있음 — 크래시가 아님
- `~/Library/Logs/DiagnosticReports`에 크래시 리포트 없음, `log show`에도 관련 오류 없음

## What Didn't Work

- **크래시 의심**: 프로세스 생존 확인으로 배제 (앱은 멀쩡히 돌고 있었음)
- **Tahoe 트레이 회귀 버그 의심 (tauri#13770)**: macOS 26에서 트레이 아이콘이 안 뜨는 알려진 회귀가 있었으나, 잠긴 버전이 tauri 2.11.5 / tray-icon 0.24.2로 이미 수정본이었고, 증상도 "처음부터 안 뜸"이 아니라 "닫는 순간 사라짐"이라 불일치
- **시스템 로그 조사**: `log show --predicate`로 15분치를 뒤져도 단서 없음 — OS가 오류로 취급하지 않는 정상 동작이었기 때문

## Solution

팝오버를 숨길 때 창만 숨기고 `app.hide()`는 호출하지 않는다.

수정 전 (`src-tauri/src/lib.rs`):

```rust
fn hide_popover(app: &AppHandle, window: &WebviewWindow) {
    let _ = window.hide();
    let _ = app.hide(); // ← Tahoe에서 트레이 아이콘까지 함께 사라짐
}
```

수정 후 (`src-tauri/src/lib.rs:33`):

```rust
fn hide_popover(_app: &AppHandle, window: &WebviewWindow) {
    // app.hide()는 쓰지 않는다 — macOS 26(Tahoe)에서 상태바 아이템 창까지 함께 숨겨져
    // 메뉴바 펭귄 아이콘이 사라진다. blur로 닫힐 때는 포커스가 이미 다른 앱으로
    // 넘어간 뒤라 창만 숨겨도 충분하다.
    let _ = window.hide();
}
```

표시 경로의 `app.show()` + `window.set_focus()`는 그대로 유지한다 — Accessory 정책에서 키보드 포커스를 잡는 데 여전히 필요하다.

## Why This Works

`NSApp.hide()`는 앱이 소유한 모든 창을 숨기는데, macOS 26(Tahoe)에서는 `NSStatusItem`(트레이 아이콘)이 쓰는 상태바 창까지 앱 소유 창으로 취급되어 함께 숨겨지는 것으로 관찰됐다(이 세션에서 macOS 26.5.2 + tauri 2.11.5로 재현·확인). 널리 알려진 Tauri 메뉴바 가이드들은 `app.hide()`로 이전 앱에 포커스를 돌려주는 패턴을 권하지만, 그 가이드들은 Tahoe 이전 macOS 기준이다.

`app.hide()`를 빼도 UX 손실이 거의 없는 이유: 팝오버가 blur로 닫히는 경우는 사용자가 이미 다른 곳을 클릭한 뒤라서 포커스가 그쪽으로 넘어가 있다. 포커스 반환을 위해 앱 전체를 숨길 필요가 없다.

## Prevention

- **macOS 메뉴바 상주 앱(트레이 + Accessory 정책)에서는 `app.hide()`를 호출하지 않는다.** 창 숨김은 `window.hide()`로 충분하다.
- 트레이 아이콘이 "사라졌는데 프로세스는 살아있는" 증상이 보이면, 크래시·회귀 버그보다 먼저 **앱 전체 숨김 계열 API 호출 여부**를 의심한다.
- 메뉴바 앱의 수동 검증 체크리스트에 "팝오버를 닫은 뒤에도 트레이 아이콘이 남아 있는가"를 포함한다 — 이 동작은 단위 테스트로 잡히지 않는 OS 통합 표면이다.
