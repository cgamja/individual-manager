# Penguin — 개인 총괄 비서 macOS 메뉴바 앱

브라우저를 열지 않고 메뉴바의 펭귄 하나로 하루 업무 흐름(할 일 확인 → 집중 → 기록 → 소통)을 끝내는
개인용 macOS 상주 앱. 사용자는 본인 1명, 배포 없음. Jira·Notion·Google Calendar·Slack·Webex를
팝오버 UI 안에서 양방향으로 다루고, 뽀모도로 타이머를 내장한다.

## 문서 권위 순서

`PRD.md` > `PRINCIPLE.md` > `CONVENTIONS.md` > `docs/plans/*` > 코드 주석

충돌하면 상위 문서가 이긴다. 상위 문서와 어긋나는 구현이 필요해지면 멈추고 보고한다.
`TODO.md`가 진행 상황의 단일 원천이고, `SERVICES.md`는 연동 서비스별 상세다.

## 명령어

| 목적 | 명령 |
|---|---|
| 개발 실행 | `npm run tauri dev` |
| 프론트 테스트 | `npm test` (= `vitest run`) |
| Rust 테스트 | `cd src-tauri && cargo test` |
| 번들 빌드 | `npm run tauri build` |

**테스트 러너가 둘이다.** PR 전에 `npm test`와 `cargo test`를 **모두** 통과시켜야 한다.
한쪽만 돌리고 "전체 통과"로 보고하지 않는다.

## 구조

```
src/                  React 19 + TS 프론트 (팝오버 웹뷰)
  components/         카드 UI (TimerCard, SettingsCard) + *.test.tsx
  lib/                Rust invoke·이벤트 래퍼 (timer, settings, notification) + *.test.ts
src-tauri/src/
  pomodoro.rs         타이머 상태머신 — Tauri 무의존 순수 모듈 + 인라인 테스트
  timer_bridge.rs     commands · 1Hz 틱 스레드 · 트레이 타이틀 · 알림 발송
  lib.rs              setup: 트레이 생성, Accessory 정책, 플러그인 등록, 창 이벤트
docs/plans/           마일스톤 항목별 구현 플랜
docs/solutions/       재발 방지용 학습 기록 — 셸을 건드리기 전에 읽는다
```

스택은 Tauri v2 + React 19 + TypeScript + Vite 7 (PRD Q1 확정). Rust는 단일 crate `penguin`.

## 반드시 지키는 규칙 (CONVENTIONS.md 요약)

- **`main`에 직접 커밋하지 않는다.** 브랜치는 `타입/기능-설명-번호` (예: `feat/m2-notion-todo-01`).
- **커밋은 한국어 Angular 컨벤션**, `타입: 제목` 50자 이내, 기능 단위로 묶는다.
- **TDD** — 핵심 로직은 실패 테스트 먼저. **테스트 이름은 한국어**
  (예: `동기화_검증_실패시_전체_재동기화를_수행한다`). 외부 API는 mock/fixture로만 테스트하고
  실제 호출하지 않는다. UI 테스트는 선택.
- **PR 하나는 `TODO.md` 체크박스 하나**를 넘지 않는다. `.github/TEMPLATE/PR.md` 템플릿을 쓴다.
- **merge는 사용자가 직접 한다.** 에이전트는 절대 merge하지 않는다.
- **시크릿은 코드·문서·커밋에 절대 넣지 않는다.** 토큰은 macOS Keychain 또는 `.gitignore`된
  로컬 설정 파일에만 둔다.
- 기능·범위가 바뀌면 `PRD.md`·`SERVICES.md`를 같은 PR에서 고치고, 끝난 항목은 `TODO.md`를 체크한다.

## 설계 원칙 (PRINCIPLE.md 요약)

1. **AI를 쓰지 않는다.** 요약·캐치·판단은 사용자 수동 조작 + 키워드 규칙으로 해결한다.
2. **브라우저를 열게 만들면 실패다.** 조회·조작 모두 앱 안에서 끝나야 한다.
3. **동기화는 신뢰 가능해야 한다.** 주기 검증(로컬 캐시 ↔ 원격) + 오류 시 전체 재동기화 +
   쓰기 실패는 재시도 큐. 외부 서비스에 쓰는 기능은 이 셋을 반드시 설계에 포함한다.
4. **업무 데이터는 원본 서비스에 남긴다.** 로컬에는 설정과 캐시만 둔다.

알림은 전부 opt-in이고 기본은 pull이다 (PRD §5.7).

## 이 코드베이스의 함정

`docs/solutions/`에 기록된 것들 — 메뉴바 셸(`lib.rs`)·브릿지(`timer_bridge.rs`)를 수정하거나
폴링·알림을 추가하기 전에 해당 문서를 읽는다.

- **`app.hide()`를 호출하지 않는다.** macOS 26(Tahoe)에서 트레이 아이콘까지 사라진다.
  창 숨김은 `window.hide()`로 충분하다. → `docs/solutions/ui-bugs/macos-tahoe-app-hide-removes-tray-icon.md`
- **숨겨진 웹뷰의 JS 타이머는 ~5분 뒤 멈춘다.** 주기 작업의 진실 원천은 타임스탬프이고
  권위 있는 틱은 Rust 스레드가 소유한다. `setInterval` 감산 방식 금지.
- **트레이는 `setup()`에서 동기 생성**해야 마우스 이벤트를 받는다. `on_tray_icon_event`에서는
  `positioner::on_tray_event`를 항상 먼저 호출한다.
- **dev 모드에서 macOS 알림은 오지 않는 것이 정상**이다. 알림 검증은 `npm run tauri build`로
  만든 `.app`에서만 한다.
- 전체 목록: `docs/solutions/best-practices/tauri-v2-macos-menubar-app-pitfalls.md`

**Tauri 플러그인을 추가할 때는 네 곳을 함께 고친다** — `src-tauri/Cargo.toml`, `package.json`,
`lib.rs`의 `.plugin(...)` 등록, 그리고 프론트에서 호출한다면 `src-tauri/capabilities/default.json`의
`permissions`. (Rust에서만 쓰는 플러그인은 capabilities 등록이 필요 없다 — positioner가 그 예다.)

## 현재 상태

M1(메뉴바 셸 + 뽀모도로 + 알림) 완료·머지됨. 다음은 **M2 — Notion TODO 통합**이며,
`TODO.md`의 "시작 전 결정" 두 건(토큰 보관 방식, 기존 Notion DB 스키마 확인)이 선행 조건이다.

마일스톤 항목 하나를 플랜부터 PR까지 끌고 가려면 `develop` 스킬을 쓴다.
