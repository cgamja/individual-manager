# Penguin — 바탕화면 펭귄 macOS 상주 앱

바탕화면에 펭귄 한 마리가 사는 개인용 macOS 상주 앱. 사용자는 본인 1명, 배포 없음.
펭귄은 걷고·헤엄치고·자고·낚시하고·미끄러지고, 때리면 방망이를 휘두르며 싸가지 없게 군다.
**이 앱은 아무 일도 하지 않는다** — 유일한 성공 기준은 "보고 있으면 웃긴가"다.

> **2026-08-31 방향 전환 (PRD v3.0)** — 업무 기능을 **전부** 뺐다. 뽀모도로 타이머, 서비스
> 런처, Claude Code 위임, Notion·Jira·Calendar 연동이 모두 범위 밖이다. v1.0(앱 안에 5개
> 서비스 UI)과 v2.0(런처 + Claude Code 위임)을 왜 차례로 접었는지는 `PRINCIPLE.md`의
> 개정 이력에 있다 — **읽지 않으면 같은 논의를 처음부터 다시 하게 된다.**

## 문서 권위 순서

`PRD.md` > `PRINCIPLE.md` > `CONVENTIONS.md` > `MOTIONS.md` > `docs/plans/*` > 코드 주석

충돌하면 상위 문서가 이긴다. 상위 문서와 어긋나는 구현이 필요해지면 멈추고 보고한다.
`TODO.md`가 진행 상황의 단일 원천이고, `MOTIONS.md`는 펭귄 동작별 명세다
(v2.0까지 있던 `SERVICES.md`를 대체했다).

## 명령어

| 목적 | 명령 |
|---|---|
| 개발 실행 | `npm run tauri dev` |
| 프론트 테스트 | `npm test` (= `vitest run`) |
| Rust 테스트 | `cd src-tauri && cargo test` |
| 번들 빌드 | `npm run tauri build` |

**테스트 러너가 둘이다.** PR 전에 `npm test`와 `cargo test`를 **모두** 통과시켜야 한다.
한쪽만 돌리고 "전체 통과"로 보고하지 않는다.

**`npm test`는 타입 검사를 하지 않는다.** vitest는 트랜스파일만 해서 타입 오류가 있어도
초록으로 통과한다. 타입은 `npm run build`(= `tsc && vite build`)가 잡는다 — 프론트를
고쳤으면 이것도 돌린다. 안 그러면 번들 빌드에서야 터진다.

## 구조

```text
src/                  React 19 + TS 프론트
  pet/                펭귄 창 웹뷰 — Penguin.tsx(SVG·CSS 모션), PetApp.tsx, pet.css
  components/         설정 창 카드 UI + *.test.tsx
  lib/                Rust invoke·이벤트 래퍼 (pet, settings) + *.test.ts
src-tauri/src/
  pet.rs              펭귄 상태머신 — Tauri 무의존 순수 모듈 + 인라인 테스트
  pet_bridge.rs       commands · 20Hz 틱 스레드 · 창 위치 · 화면 경계
  lib.rs              setup: 트레이 생성, Accessory 정책, 플러그인 등록, 창 이벤트
docs/plans/           마일스톤 항목별 구현 플랜
docs/solutions/       재발 방지용 학습 기록 — 셸을 건드리기 전에 읽는다
```

스택은 Tauri v2 + React 19 + TypeScript + Vite 7 (PRD Q1 확정). Rust는 단일 crate `penguin`.

## 반드시 지키는 규칙 (CONVENTIONS.md 요약)

- **`main`에 직접 커밋하지 않는다.** 브랜치는 `타입/기능-설명-번호` (예: `feat/f3-ice-fishing-01`).
- **커밋은 한국어 Angular 컨벤션**, `타입: 제목` 50자 이내, 기능 단위로 묶는다.
- **TDD** — 핵심 로직(`pet.rs`의 상태 전이·경계 판정)은 실패 테스트 먼저.
  **테스트 이름은 한국어** (예: `공중에서_클릭하면_제자리에서_반응한다`). UI 테스트는 선택.
- **PR 하나는 `TODO.md` 체크박스 하나**를 넘지 않는다. `.github/TEMPLATE/PR.md` 템플릿을 쓴다.
- **에이전트가 merge해도 된다** (2026-08-30 사용자 지시). 단 두 러너를 모두 통과하고
  코드 리뷰 지적을 반영한 뒤에 한다. 되돌리기 어려운 다른 작업(force push, 브랜치 삭제,
  이력 재작성)은 여전히 사용자 확인을 받는다.
- 기능·범위가 바뀌면 `PRD.md`·`MOTIONS.md`를 같은 PR에서 고치고, 끝난 항목은 `TODO.md`를 체크한다.

## 설계 원칙 (PRINCIPLE.md 요약)

1. **쓸모를 목적으로 삼지 않는다.** "업무에 도움이 되나"로 기능을 정당화하지 않는다 —
   그 논리가 v1.0과 v2.0을 만들었고 둘 다 접혔다. 재미의 반대말은 **예측 가능함**이다.
2. **앱은 하나, 마릿수는 사용자가 정한다.** 두 개가 뜨는 것(버그)과 골라서 늘리는 것(기능)은
   다르다. **세계는 펭귄이 떠 있는 화면 하나**이고 모니터 경계는 벽이다 — 경계 넘기는
   2026-08-31에 범위 밖으로 뺐다 (PRINCIPLE 개정 이력 v3.2).
3. **동작은 규칙과 시드 난수로 만든다 — AI를 쓰지 않는다.** 같은 시드에 같은 결과가
   나와야 테스트할 수 있다. (앱 런타임 이야기이고, 개발에 Claude Code를 쓰는 것과 무관하다.)
4. **상태의 주인을 나눈다** — Rust 코어는 "무슨 동작·어디에", 웹뷰는 "어떻게 보이는지".
5. **방해하지 않는다** — 포커스를 뺏지 않고, **소리는 기본 꺼짐**, 알림은 보내지 않고,
   설정 세 가지(펭귄 on/off·대사 목록·소리 on/off) 말고는 아무것도 저장하지 않는다.

## 이 코드베이스의 함정

`docs/solutions/`에 기록된 것들 — 메뉴바 셸(`lib.rs`)·브릿지(`pet_bridge.rs`)를 수정하기
전에 해당 문서를 읽는다.

- **`app.hide()`를 호출하지 않는다.** macOS 26(Tahoe)에서 트레이 아이콘까지 사라진다.
  창 숨김은 `window.hide()`로 충분하다. → `docs/solutions/ui-bugs/macos-tahoe-app-hide-removes-tray-icon.md`
- **숨겨진 웹뷰의 JS 타이머는 ~5분 뒤 멈춘다.** 주기 작업의 진실 원천은 타임스탬프이고
  권위 있는 틱은 Rust 스레드가 소유한다. `setInterval` 감산 방식 금지.
- **트레이는 `setup()`에서 동기 생성**해야 마우스 이벤트를 받는다. `on_tray_icon_event`에서는
  `positioner::on_tray_event`를 항상 먼저 호출한다.
- **`current_monitor()`는 이벤트 루프를 왕복하는 블로킹 호출이다.** 20Hz 틱에서 매번
  부르지 않는다 — 현재는 주기적으로 캐시한다.
- **창별 이벤트는 `emit_to`만으로 안 된다.** 전역 `listen()`은 대상을 `Any`로 등록하고,
  Tauri는 `Any` 리스너를 **emit 대상과 무관하게 전부** 호출한다. 받는 쪽도
  `getCurrentWebviewWindow().listen()`으로 창에 묶어야 한다. 창이 하나일 때는 드러나지
  않다가 여러 창으로 늘리는 순간 터진다. →
  `docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`
- **`#[tauri::command]`를 만들고 `lib.rs`의 `generate_handler!`에 등록하지 않으면
  컴파일·테스트·경고가 전부 통과하고 런타임에서만 조용히 reject된다.** 새 창의
  capabilities 라벨 누락도 같은 성격이다. → `docs/solutions/best-practices/tauri-command-registration-silent-failure.md`
- **같은 이름의 `@keyframes`를 두 번 정의하면 앞의 애니메이션이 통째로 죽는다.** 나중
  정의가 그 이름에 대한 참조 **전부**를 가져간다. 굴러떨어지기 그림이 한 PR 내내 죽어
  있었고 두 러너·타입 검사·리뷰가 전부 통과했다. 이름은 **쓰는 클래스에서** 딴다
  (`pg-thrown-spin`). → `docs/solutions/ui-bugs/duplicate-keyframes-silently-kills-animation.md`
- 전체 목록: `docs/solutions/best-practices/tauri-v2-macos-menubar-app-pitfalls.md`

**Tauri 플러그인을 넣거나 뺄 때는 네 곳을 함께 고친다** — `src-tauri/Cargo.toml`,
`package.json`, `lib.rs`의 `.plugin(...)` 등록, 그리고 프론트에서 호출한다면
`src-tauri/capabilities/default.json`의 `permissions`. (Rust에서만 쓰는 플러그인은
capabilities 등록이 필요 없다 — positioner가 그 예다.)

## 현재 상태

M1(메뉴바 셸 + 뽀모도로 + 알림), M2(Notion TODO), M2.5(바탕화면 펭귄), M3(Notion 제거)가
머지됐다. **v3.0 방향 전환으로 뽀모도로·알림은 제거 대상**이고, M4(런처)는 만들어 놓고
머지하지 않은 채 폐기했다(`feat/m4-launcher-fan-01`).

**F1 — 걷어내기**(뽀모도로·알림 제거)와 **F3의 얼음낚시·슬라이딩·굴러떨어지기**가 머지됐다.
**F2 — 세계 넓히기는 2026-08-31에 폐기했다** — 모니터가 하나뿐이라 볼 일이 없다.
좌표계 교체(`World`/`Screen`)만 머지된 채 남아 있고 **프로덕션에서는 화면 하나만 담는다.**

남은 것은 **F3의 빽빽거리기·발작·핀볼 모드**와 빈도 재조정이다.

마일스톤 항목 하나를 플랜부터 PR까지 끌고 가려면 `develop` 스킬을 쓴다.
