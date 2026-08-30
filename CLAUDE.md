# Penguin — 개인 총괄 비서 macOS 메뉴바 앱

바탕화면의 펭귄이 하루 업무의 진입점이 되는 개인용 macOS 상주 앱. 사용자는 본인 1명, 배포 없음.
펭귄을 누르면 서비스 아이콘이 뜨고 고르면 Chrome으로 열린다. 조회·기록은 Claude Code에 위임하고,
앱이 직접 소유하는 것은 **펭귄과 뽀모도로 타이머**뿐이다.

> **2026-08-30 방향 전환 (PRD v2.0)** — 5개 서비스의 양방향 UI를 앱 안에 만드는 v1.0 방향을
> 접었다. 특히 "브라우저를 열게 만들면 실패"라는 옛 원칙 2는 **뒤집혔다.** 옛 원칙과 그 폐기
> 이유는 `PRINCIPLE.md`의 개정 이력에 있다 — 읽지 않으면 같은 논의를 처음부터 다시 하게 된다.

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

```text
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
- **시크릿은 코드·문서·커밋에 절대 넣지 않는다.** 토큰은 macOS Keychain에만 둔다
  (Rust `keyring` crate). Keychain 접근은 Rust에서만 하고 토큰 값을 웹뷰로 넘기지 않는다.
- 기능·범위가 바뀌면 `PRD.md`·`SERVICES.md`를 같은 PR에서 고치고, 끝난 항목은 `TODO.md`를 체크한다.

## 설계 원칙 (PRINCIPLE.md 요약)

1. **Claude Code가 연동의 실행 엔진이다.** 앱은 Notion·Jira·Calendar의 API 클라이언트를
   만들지 않는다. 모델은 작업 성격에 따라 섞는다 (판단=Opus / 일반=Sonnet / 단순 조회=Haiku).
2. **앱은 런처다.** 깊은 작업은 Chrome으로 연 원래 서비스에서 한다. 앱 안에 서비스 UI를
   다시 만들지 않는다.
3. **앱은 업무 데이터를 캐시하지 않는다.** 로컬에는 설정만 둔다 — 동기화 검증·재시도 큐
   같은 장치도 함께 불필요해졌다.
4. **업무 데이터는 원본 서비스에 남긴다.** 개정에서 유일하게 그대로인 원칙이다.

알림은 뽀모도로 종료만 남았다. 앱이 폴링을 하지 않으므로 서비스 이벤트 알림은 범위 밖이다.

**펭귄의 동작과 타이머 로직에는 AI를 쓰지 않는다** — 규칙과 시드 난수로 충분하다.

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
- **`#[tauri::command]`를 만들고 `lib.rs`의 `generate_handler!`에 등록하지 않으면
  컴파일·테스트·경고가 전부 통과하고 런타임에서만 조용히 reject된다.** 새 창의
  capabilities 라벨 누락도 같은 성격이다. → `docs/solutions/best-practices/tauri-command-registration-silent-failure.md`
- 전체 목록: `docs/solutions/best-practices/tauri-v2-macos-menubar-app-pitfalls.md`

**Tauri 플러그인을 추가할 때는 네 곳을 함께 고친다** — `src-tauri/Cargo.toml`, `package.json`,
`lib.rs`의 `.plugin(...)` 등록, 그리고 프론트에서 호출한다면 `src-tauri/capabilities/default.json`의
`permissions`. (Rust에서만 쓰는 플러그인은 capabilities 등록이 필요 없다 — positioner가 그 예다.)

## 현재 상태

M1(메뉴바 셸 + 뽀모도로 + 알림), M2(Notion TODO), M2.5(바탕화면 펭귄)가 완료·머지됐다.
방향 전환으로 **M2의 산출물은 제거 대상**이다.

다음은 **M3 — Notion 기능 제거 + 정리**다. 남길 코드가 줄어야 런처(M4)·타이머 연동(M5)
설계가 단순해진다. **M6(Claude Code 위임)은 PRD Q3(연동 방식)이 미정이라 착수하지 않는다** —
`claude` CLI를 자식 프로세스로 띄울지, 터미널을 열지, 별도 대화 UI를 둘지부터 정해야 한다.

마일스톤 항목 하나를 플랜부터 PR까지 끌고 가려면 `develop` 스킬을 쓴다.
