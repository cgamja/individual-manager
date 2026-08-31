---
title: 던지기 속도를 세계 폭에 비례시킨다 - Plan
type: fix
date: 2026-08-31
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 던지기 속도를 세계 폭에 비례시킨다 — Plan

## Goal Capsule

- **목표** — 던지기 속도 상한이 화면 크기와 무관한 고정값이라 단일 화면에서 펭귄이 너무
  빨리 날아가는 문제를 고친다. 상한을 **세계 폭에 비례**시켜, 좁은 화면에서는 얌전하고
  화면이 넓어지면(확장 모니터) 같은 손짓이 더 멀리 가게 한다.
  범위: `TODO.md`의 "바로 고칠 것" 체크박스 **하나**.
- **권위 순서** — `PRD > PRINCIPLE > CONVENTIONS > MOTIONS > 이 플랜`. 충돌하면 상위가 이긴다.
- **실행 프로필** — 브랜치 `fix/throw-speed-world-scaled-01`. 코어(`pet.rs`)는 **TDD 필수**
  (실패 테스트 먼저, 이름은 한국어). 체감은 `npm run tauri dev`로 수동 확인.
  Implementation Unit 하나 = 커밋 하나.
- **정지 조건** — 아래면 멈추고 보고한다.
  1. 상한을 낮췄더니 "던진다"는 감각 자체가 사라진다 (드래그 던지기가 낙하와 구분되지 않는다).
  2. 세계 폭을 코어에 넘기는 일이 F2(다중 화면 좌표계, PRD Q7)의 설계 결정을 앞당겨
     요구한다 — 그건 이 PR의 범위가 아니다.

---

## Problem Frame

`src-tauri/src/pet.rs`의 던지기 속도 한계가 **절대 px/s 고정값**이다.

| 상수 | 값 | 1440px 화면에서의 의미 |
|---|---|---|
| `THROW_MIN_SPEED` | 260 px/s | 화면 폭의 0.18배/초 — 던짐/떨어뜨림을 가르는 문턱 |
| `THROW_MAX_SPEED` | 2600 px/s | 화면 폭의 **1.8배/초** — 화면을 **0.55초**에 가로지른다 |

`clamp_throw(vx, vy)`가 이 상한으로 방향만 유지한 채 속도를 자르는데, 상한 자체가 화면을
모른다. 트랙패드로 세게 튕기면 2000~4000px/s가 쉽게 나오므로 실제로는 **거의 항상 상한에
걸리고**, 그 상한이 단일 화면 기준으로 너무 높다. 확장 모니터를 붙여 세계가 넓어져도 상한은
그대로라 반대로 답답해진다.

`drag_end`는 지금 `Bounds`를 받지 않는다 — 그래서 코어가 세계 폭을 알 수 없는 것이
구조적 원인이다.

---

## Requirements

- **R1** — 던지기 속도 상한이 세계 폭에 비례한다. 좁은 화면에서는 낮고, 넓으면 높다.
- **R2** — 던짐/떨어뜨림을 가르는 **최소 속도는 화면 크기와 무관하게 유지**한다.
  그 문턱은 "사용자가 튕겼는가"라는 **손의 의도**에 대한 것이지 화면에 대한 것이 아니다.
- **R3** — 세계 폭을 읽지 못하거나(모니터 조회 실패) 비정상적으로 좁아도 던지기가
  망가지지 않는다.
- **R4** — 코어(`pet.rs`)는 Tauri 무의존 순수 모듈로 남는다 (PRINCIPLE 3·4).
- **R5** — F2(다중 화면)에서 세계가 넓어지면 이 상한이 **자동으로 따라온다.**
  F2가 `Bounds`를 무엇으로 바꾸든 "세계 폭"이라는 입력 하나만 유지되면 된다.

---

## Key Technical Decisions

**KTD1 — 상한만 비례시키고, 최소 속도는 고정한다.**
둘 다 비례시키면 같은 손짓이 넓은 화면에서는 "떨어뜨림", 좁은 화면에서는 "던짐"으로
갈린다. 문턱은 제스처의 성격이고 상한은 세계의 성격이다 (R2).
→ `THROW_MIN_SPEED`는 260 px/s 그대로. `THROW_MAX_SPEED`를 `THROW_MAX_WORLDS_PER_SEC`
비율로 대체한다.

**KTD2 — 비율은 `0.9` (초당 세계 폭의 0.9배)로 시작한다.**
1440px 화면에서 1296px/s ≈ 현재의 절반. 화면을 가로지르는 데 1.1초로, "빠르게 날아가되
눈으로 따라갈 수 있는" 구간이다. **취향 상수**이므로 수동 확인 후 조정할 수 있게
한 곳에만 둔다.

**KTD3 — 세계 폭이 유효하지 않으면 기준 폭(1440)으로 대체한다.**
`bounds_or_flat()`은 모니터를 못 읽으면 폭 0인 납작한 경계를 준다. 비례식에 그대로
넣으면 상한이 0이 되어 **모든 던지기가 낙하로 바뀐다** — 조용히 기능이 죽는 부류의
버그다. 폭이 0 이하면 기준 폭을 쓰고, 계산된 상한이 `THROW_MIN_SPEED`보다 낮아지면
최소 속도까지 끌어올린다 (R3).

**KTD4 — 브릿지가 경계를 넘긴다. 코어는 여전히 Tauri를 모른다.**
`pet_drag_end`에서 이미 있는 `bounds_or_flat(&app)`을 호출해 `drag_end`에 넘긴다
(`pet_whack`이 같은 방식을 쓴다 — 새 패턴이 아니다). 코어는 `Bounds`라는 자기 타입만 본다 (R4).

**범위 밖** — 중력(`GRAVITY`), 튕김 감쇠(`BOUNCE_DAMPING`), 걷기·헤엄 속도는 건드리지
않는다. 던지기 체감이 잡힌 뒤에도 남는 문제가 있으면 그때 별도 항목으로 만든다.

---

## Implementation Units

### U1. 코어 — 세계 폭에 비례하는 던지기 상한

**Goal** — `clamp_throw`가 세계 폭을 받아 상한을 계산하고, `drag_end`가 그 폭을 넘긴다.

**Requirements** — R1, R2, R3, R4

**Dependencies** — 없음

**Files**
- `src-tauri/src/pet.rs` (구현 + 인라인 테스트)

**Approach**
- `THROW_MAX_SPEED` 상수를 지우고 `THROW_MAX_WORLDS_PER_SEC: f64 = 0.9`와
  `FALLBACK_WORLD_WIDTH: f64 = 1440.0`을 넣는다.
- 상한 계산을 순수 함수 하나로 분리한다 — 입력은 세계 폭, 출력은 상한 속도.
  폭이 0 이하면 기준 폭을 쓰고, 결과가 `THROW_MIN_SPEED`보다 작으면 최소 속도로 올린다.
- `clamp_throw(vx, vy, world_width)` — 방향 유지, 속도만 상한으로 자르는 기존 동작 유지.
- `drag_end(now_ms, vx, vy, bounds)` — `bounds`에서 폭(`right - left`)을 꺼내 넘긴다.
  `whack(now_ms, _bounds)`가 이미 `Bounds`를 받는 전례가 있다.

**Execution note** — 실패 테스트 먼저. 기존 던지기 테스트 6개가 `drag_end` 시그니처
변경으로 깨지므로, 먼저 컴파일이 통과하도록 호출부를 고친 뒤 새 테스트를 Red로 만든다.

**Patterns to follow** — `whack(&mut self, now_ms, _bounds)`의 경계 주입 방식,
같은 파일의 기존 인라인 `#[cfg(test)] mod tests`.

**Test scenarios**
- `좁은_화면에서는_던지기_상한이_더_낮다` — 같은 과속 입력(예: 10_000px/s)을 폭 1440과
  폭 2880의 경계로 각각 던져, 앞쪽 결과 속도가 뒤쪽의 절반인지 본다.
- `상한_이하의_던지기는_속도가_그대로다` — 폭 1440에서 (400, -300) → 잘리지 않는다.
- `상한은_방향을_유지한_채_속도만_줄인다` — 과속 입력의 vx:vy 비가 보존된다.
- `화면_폭을_읽지_못하면_기본_폭으로_상한을_잡는다` — 폭 0인 납작한 경계로 세게 던져도
  `Falling`이 아니라 `Thrown`으로 들어간다 (KTD3의 조용한 죽음 방지).
- `세계가_너무_좁아도_던지기_문턱_아래로_내려가지_않는다` — 폭 100에서 세게 던져도
  상한이 `THROW_MIN_SPEED` 이상이라 던지기로 성립한다.
- `살짝_놓으면_던지지_않고_제자리에서_떨어진다` (기존) — 최소 속도가 고정임을 지키는
  회귀 가드. 화면 폭을 바꿔도 결과가 같아야 한다.
- `던지기_속도는_상한을_넘지_않는다` (기존) — 세계 폭 기준 상한으로 단언을 고친다.
- `세게_던지면_포물선을_그린다` (기존) — 시그니처만 맞추고 의미는 유지.

**Verification** — `cd src-tauri && cargo test` 통과. 새 테스트 5개가 추가되고
기존 던지기 테스트가 전부 살아 있다.

### U2. 브릿지 — 놓는 시점의 경계를 코어에 넘긴다

**Goal** — `pet_drag_end` 커맨드가 현재 이동 영역을 `drag_end`에 전달한다.

**Requirements** — R1, R4

**Dependencies** — U1

**Files**
- `src-tauri/src/pet_bridge.rs`

**Approach** — `pet_whack`과 동일하게 `bounds_or_flat(&app)`을 호출해 `drag_end`에 넘긴다.
커맨드 시그니처(웹뷰가 보내는 `vx`, `vy`)는 바뀌지 않으므로 `src/lib/pet.ts`와
`capabilities`는 손대지 않는다 — 프론트 변경 없음.

**Test scenarios** — `Test expectation: none` — Tauri `AppHandle`이 필요한 얇은 위임이라
단위 테스트로 잡히지 않는다. 동작 확인은 아래 수동 검증이 맡는다.

**Verification** — `npm run tauri dev`로 띄워 단일 화면에서 세게 던졌을 때
화면을 가로지르는 데 1초 안팎이 걸리고, 벽에서 튕긴 뒤 얌전히 착지한다.

### U3. 문서 최신화

**Goal** — 끝난 항목을 닫고 결정을 문서에 남긴다.

**Requirements** — CONVENTIONS "문서 최신화"

**Dependencies** — U1, U2

**Files**
- `TODO.md` — "바로 고칠 것" 체크박스를 체크한다
- `MOTIONS.md` — "던지기 속도는 세계 크기를 따라야 한다" 문단을 **확정된 동작 설명**으로 고친다
- `PRD.md` — §5.1의 "TODO 바로 고칠 것" 참조를 지운다 (이제 구현된 동작이다)

**Test scenarios** — `Test expectation: none — 문서 전용`

**Verification** — `TODO.md`에 미체크로 남은 "바로 고칠 것" 항목이 없다.

---

## Scope Boundaries

**이 PR 안**
- 던지기 속도 상한의 세계 폭 비례화, 그에 딸린 경계 주입, 테스트, 문서.

**Deferred to Follow-Up Work**
- **F2 — 다중 화면 좌표계 (PRD Q7).** 이 PR은 `Bounds` 하나를 그대로 쓴다. 세계 폭이라는
  입력만 유지되면 F2가 `Bounds`를 무엇으로 바꾸든 상한은 자동으로 따라온다 (R5).
- **중력·튕김 감쇠 재조정.** 던지기 체감을 보고도 남는 문제가 있으면 그때 항목을 만든다.
- **던지기 세기 설정 노출.** 설정은 세 가지(펭귄 on/off·대사·소리)로 고정이다 (PRD §7).

---

## Risks

| 리스크 | 대응 |
|---|---|
| 0.9라는 비율이 여전히 빠르거나 이번엔 너무 느리다 | 취향 상수를 한 곳에만 두어 수동 확인 후 한 줄로 조정한다. U2의 수동 검증이 이걸 잡는 자리다 |
| `drag_end` 시그니처 변경이 기존 테스트 6곳을 깬다 | 예상된 변경이다. 호출부를 먼저 맞춘 뒤 새 테스트를 Red로 만든다 |
| 폭 0인 납작한 경계에서 던지기가 조용히 죽는다 | KTD3의 대체 폭 + 전용 테스트(`화면_폭을_읽지_못하면_...`)로 막는다 |

---

## Definition of Done

- [ ] `cargo test` 통과 (새 테스트 5개 포함)
- [ ] `npm test` 통과 (프론트 변경 없음 — 회귀 확인용)
- [ ] `npm run tauri dev`로 단일 화면에서 던지기 체감 확인
- [ ] `ce-code-review` 지적 반영
- [ ] `TODO.md`·`MOTIONS.md`·`PRD.md` 최신화
