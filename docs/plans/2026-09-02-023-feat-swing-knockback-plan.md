---
title: 방망이를 휘두르면 앞에 있는 마리가 날아간다 - Plan
type: feat
date: 2026-09-02
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 방망이를 휘두르면 앞에 있는 마리가 날아간다

## Goal Capsule

- **목표** — 펭귄을 왼쪽 클릭해 방망이를 휘두르면, **그 펭귄이 보는 방향**의 스윙 범위 안에
  있는 **다른 마리**가 함께 날아간다. 날아간 마리는 평소 던지기 물리(`Thrown`)를 그대로 타고
  착지 등급(`Land`/`Splat`/`Sprawl`)도 그대로 받는다. 근거: `TODO.md` "펭귄 마릿수" 절의
  체크박스 하나(2026-09-02 사용자 지시).
  **정정(구현 중)** — 처음엔 "PRD를 고치지 않는다(사거리 세부일 뿐이다)"로 적었는데
  틀렸다. `PRD.md` §5.2가 **"휘둘러도 날아가지 않는다 — 나는 건 드래그로 던졌을
  때뿐이다. 핀볼 모드에서만 예외다"**라고 못 박고 있어 정면으로 어긋난다. 코드 리뷰가
  잡았고, §5.2와 §5.4 조작 표를 같은 PR에서 고쳤다 (CONVENTIONS의 "기능·범위가 바뀌면
  PRD를 같은 PR에서 함께 수정한다").
- **권위 순서** — `PRD > PRINCIPLE > CONVENTIONS > MOTIONS > 이 플랜`. 충돌하면 상위가 이기고,
  상위와 어긋나는 구현이 필요해지면 멈추고 보고한다.
- **실행 프로필** — 브랜치 `feat/f3-swing-knockback-01`, PR 하나. TDD는 코어
  (`src-tauri/src/pet/`)의 판정과 전이에 적용하고 테스트 이름은 한국어다. 커밋은 한국어
  Angular 컨벤션, 유닛 하나가 커밋 하나.
- **정지 조건** — 아래 중 하나라도 걸리면 멈추고 보고한다.
  - 넉백을 **핀볼 모드에도** 넣어야 한다는 결론이 난다 → 마리 간 충돌은 다른 PR의 설계
    영역이라 겹친다 (KTD4).
  - 판정을 `Pets::step_all`/`step_bowling` 본문이나 `pet/motion/pinball.rs`,
    `pet_bridge/tick.rs`에서 해야만 한다는 결론이 난다 → 같은 이유로 소유 경계 밖이다.
  - `Pet::whack`의 시그니처를 바꿔야 한다 → 이음매 규칙(#44)과 어긋난다.
  - 골든 수열(`같은_시드는_같은_동작_시퀀스를_낳는다`)이 깨진다 → 사용자 입력 경로가
    `pick_next`의 난수를 소비했다는 뜻이다.
- **꼬리 작업** — `.github/TEMPLATE/PR.md`로 PR을 연다. **merge하지 않는다.** 같은 PR에
  `TODO.md`(체크박스 한 줄)와 `MOTIONS.md`(`Swing` 절)를 넣는다. `npm run tauri dev` 스모크는
  다른 에이전트와 `single-instance`가 겹치므로 돌리지 않고, PR "테스트" 절에 수동 확인
  체크리스트를 남긴다.

---

## Product Contract

### Summary

작업이 끝나면 사용자는 펭귄 둘을 나란히 붙여 놓고 뒤쪽 마리를 때릴 수 있다. 맞은 마리가
방망이를 휘두르면 **그 앞에 서 있던 마리가 방망이에 걸려 포물선을 그리며 날아가** 벽에 튕기고
철푸덕 널브러진다. 지금은 같은 상황에서 방망이가 옆 마리를 그대로 통과한다.

### Problem Frame

빠따는 이 앱에서 가장 자주 쓰는 조작인데 **맞은 한 마리만** 움직인다. 마릿수를 8까지 늘릴 수
있게 해 놓고 여러 마리를 붙여 놓을수록 "휘두른 방망이가 옆 마리를 통과한다"가 눈에 띈다 —
마릿수가 늘수록 어색함도 같이 는다. 볼링(#45)이 "여러 마리가 하나의 사건에 참여한다"를
열었지만 그건 버튼으로 여는 특별한 판이고, **평소에 늘 쓰는 조작**은 아직 한 마리만 본다.

### Requirements

| ID | 요구사항 |
|---|---|
| R1 | 스윙이 실제로 나왔을 때만 넉백이 난다. 연타로 빽빽거리거나 핀볼 채로 친 클릭은 넉백을 만들지 않는다 |
| R2 | 넉백 대상은 **휘두른 마리가 보는 방향**에 있는 다른 마리다. 등 뒤는 안 맞는다 |
| R3 | 스윙에는 사거리가 있다. 몸통 가운데끼리 일정 거리 안이고 위아래로도 일정 폭 안이어야 맞는다 |
| R4 | 맞은 이웃은 `Thrown`이 되어 앞쪽 비스듬히 위로 날아간다. 속도는 세계 폭에 비례한다 |
| R5 | 맞은 이웃은 착지 등급을 그대로 탄다 — 세게 떨어지면 철푸덕·널브러짐이 나도 된다 |
| R6 | **연쇄가 없다.** 날아간 이웃이 또 다른 마리를 치지 않는다 |
| R7 | 들려 있는 마리(`Dragged`)와 볼링 판에 선 마리는 넉백 대상이 아니다 |
| R8 | 맞은 이웃은 방망이를 휘두르지 않는다 — `whack_seq`가 늘지 않는다 |
| R9 | 넉백은 사용자 입력 경로다. 코어의 난수를 소비하지 않아 골든 수열이 그대로다 |

### Acceptance Examples

- **AE1** — 두 마리를 나란히 놓고 왼쪽 마리가 **오른쪽을 보는 상태**에서 왼쪽 마리를 클릭한다.
  왼쪽 마리는 방망이를 휘두르고, 오른쪽 마리는 오른쪽 위로 날아가 벽에 튕겼다가 떨어진다.
- **AE2** — 같은 배치에서 **오른쪽 마리**를 클릭한다(오른쪽 마리는 오른쪽을 본다). 오른쪽
  마리만 휘두르고 **왼쪽 마리는 그대로 있다** — 등 뒤이기 때문이다.
- **AE3** — 두 마리를 화면 양 끝에 떨어뜨려 놓고 한쪽을 클릭한다. 아무도 날아가지 않는다.
- **AE4** — 세 마리를 A—B—C로 붙여 놓고 A를 클릭한다(A는 오른쪽을 본다). B만 날아가고
  **C는 그대로다** — 날아가는 B가 C를 치지 않는다 (R6).
- **AE5** — 핀볼 모드를 켜고 두 마리를 붙여 놓고 한쪽을 클릭한다. 클릭한 마리만 채에 맞아
  날아간다 — 핀볼에서는 넉백이 없다.

### Scope Boundaries

**비목표**

- **핀볼 모드의 마리 간 충돌.** 별도 항목이고 양방향 충돌(상대속도 + 운동량 배분)을 설계한다.
  이 PR은 평소 모드의 **단방향** 넉백만 넣는다.
- **연쇄.** 위와 같은 이유다 (R6).
- **웹뷰 변경.** 날아가는 이웃은 이미 있는 `Thrown` 그림을 그대로 쓴다. 새 CSS·새
  `@keyframes`가 없다.

**Deferred to Follow-Up Work**

- **맞은 이웃의 "퍽" 효과음.** `whack_seq`에 붙어 있는데 이웃은 그 값을 올리지 않는다
  (KTD7). 핀볼 채 타격이 이미 같은 이유로 무음이라 일관적이다 — 소리가 아쉬우면 그때
  `Thrown` 진입에 붙는 별도 소리를 논의한다.

---

## Planning Contract

### Key Technical Decisions

- **KTD1 — 판정 자리는 커맨드 경로(`Pets::whack`)다. 틱이 아니다.**
  넉백은 **사용자 클릭 한 번의 결과**라 그 순간에 한 번만 판정하면 된다. 틱(`step_all`)에
  얹으면 (a) 20Hz로 매번 전 마리를 훑고, (b) "지금 스윙 중인가"를 시간으로 다시 판정해야
  하며, (c) `step_all` 본문은 지금 다른 PR(핀볼 마리 간 충돌)이 쓰고 있다. 커맨드 경로는
  그 셋을 전부 피한다.

- **KTD2 — `Pet::whack`의 시그니처를 바꾸지 않는다.**
  `Pet`은 자기 자신만 본다. 다른 마리를 보는 판정은 `Pets`에 새 메서드로 얹는다 — #44에서
  세운 이음매 규칙이고 `Pets::step_bowling`이 그 선례다. `whack`에 이웃 목록을 인자로
  넘기기 시작하면 모든 모션 함수와 테스트가 따라 바뀐다.

- **KTD3 — 넉백 조건은 "때린 마리가 `Behavior::Swing`이 됐는가" 하나다.**
  `whack`은 상황에 따라 셋으로 갈린다: 스윙, 빽빽거리기(연타 20회 또는 빽빽거리는 중의
  재타), 핀볼 `flip`. **넉백은 방망이가 만드는 것**이므로 스윙일 때만 나야 한다. 진입 뒤
  동작을 한 번 보면 세 갈래가 한 조건으로 갈린다 — 조건을 세 개로 쪼개 쓰면 나중에 네
  번째 갈래가 생겼을 때 조용히 틀린다.

- **KTD4 — 핀볼 모드에는 넣지 않는다.**
  핀볼에서 왼쪽 클릭은 방망이가 아니라 **커서가 든 채**이고(`MOTIONS.md` 핀볼 절), 맞은
  마리는 휘두르지 않는다. 휘두르지 않는 타격에 "방망이 사거리"를 붙일 근거가 없다. 게다가
  핀볼에서 마리끼리 부딪히는 판정은 다른 PR이 양방향으로 설계 중이라, 여기서 단방향
  넉백을 먼저 넣으면 둘 중 하나를 나중에 버려야 한다. KTD3의 한 조건이 이걸 자동으로
  보장한다 — `flip`은 `Swing`이 아니다.

- **KTD5 — 넉백 속도는 세계 폭 비례이고 던지기 상한보다 느리다.**
  `SWING_KNOCK_WORLDS_PER_SEC = 1.0`, `assert!(SWING_KNOCK_WORLDS_PER_SEC <
  THROW_MAX_WORLDS_PER_SEC)`. 세기 순서를 **볼링 핀(1.5) > 손으로 던지기(1.4) > 방망이에
  스친 이웃(1.0) > 핀볼 채(0.8)** 로 둔다. 손으로 조준해 던진 것이 옆에서 스친 것보다
  세야 "때렸다"와 "던졌다"의 그림이 뒤집히지 않는다 — 던지기 상한을 올린 커밋
  `afd03e7`이 볼링 핀을 함께 올린 것과 같은 이유이고, 같은 방식으로 `assert!`가 그 순서를
  붙든다. 고정 px/s로 두지 않는 이유도 그 커밋에 있다: 화면 폭과 무관한 고정값은 화면이
  넓어지면 "안 날아간 것"처럼 보인다.

- **KTD6 — 착지 등급을 그대로 태운다.**
  `TODO.md`의 지시다. 저절로 나는 철푸덕(헤엄 끝 자유낙하)은 사용자가 만들지 않은
  결과라 줄였지만, 넉백은 사용자가 방금 클릭해서 만든 결과다. 철푸덕이 나오는 편이
  "내가 날렸다"는 인과가 선명하다.

- **KTD7 — 맞은 이웃의 `whack_seq`는 올리지 않는다.**
  `whack_seq`는 웹뷰에서 **방망이를 한 번 휘두르는 신호**이고(`Snapshot` 주석), "퍽"
  효과음도 거기 붙어 있다(`MOTIONS.md` 효과음 표). 이웃은 맞는 쪽이지 휘두르는 쪽이
  아니므로 올리면 방망이가 두 개 보인다. 대신 소리가 안 나는데, 핀볼 채 타격이 이미 같은
  이유로 무음이라 규칙이 갈라지지 않는다 (Deferred 참고).

- **KTD8 — 들려 있는 마리와 볼링 판에 선 마리는 제외한다.**
  `Dragged`는 사용자의 손이 잡고 있다 — 손과 방망이가 다투면 손이 이겨야 잡은 게 튀어
  나가지 않는다. 볼링 핀은 **판이 소유한 상태**라(`Pets::step_bowling`) 방망이로 쓰러뜨리면
  공이 할 일이 없어진다. 둘 다 `Pets`가 이미 볼 수 있는 상태라 새 필드가 필요 없다.

- **KTD9 — 브릿지는 영향받은 id 목록을 받아 각각 `flush`한다.**
  `pet_whack`은 지금 한 마리만 flush한다. 이웃이 날아가면 그 창도 즉시 옮겨야 한다 —
  다음 틱까지 기다리면 맞은 순간과 날아가는 순간이 벌어져 보인다. **목록은 반드시 `let`으로
  먼저 받는다**: `for id in pets.lock().unwrap().whack(..)`은 `MutexGuard`를 루프 내내
  붙들고 `flush`가 같은 락을 다시 잡아 즉시 자기 데드락이다
  (`docs/solutions/best-practices/rust-for-loop-holds-mutex-guard-across-body.md` —
  볼링에서 실제로 밟았고 두 러너·타입 검사·번들 빌드가 전부 통과했다).

- **KTD10 — 난수를 소비하지 않는다.**
  사거리 판정도 방향도 전부 결정적이다. `pick_next`의 추첨 사다리를 건드리지 않으므로
  골든 수열 재기준화가 없다 (R9).

### 사거리 상수

`tuning.rs`에 **"── 스윙 넉백 ──" 절을 새로 만든다**(기존 절은 건드리지 않는다).

| 상수 | 값 | 근거 |
|---|---|---|
| `SWING_REACH` | 200.0 | 몸통 가운데끼리의 거리. `PET_SIZE`(140)보다 커야 **어깨를 맞댄 이웃이 반드시 닿는다** — `const assert!`로 묶는다 |
| `SWING_REACH_V` | 100.0 | 위아래 폭. `PET_SIZE`보다 작아야 한 층 위를 헤엄치는 마리가 안 맞는다 |
| `SWING_KNOCK_WORLDS_PER_SEC` | 1.0 | KTD5 |
| `SWING_KNOCK_LIFT` | 0.5 | 앞으로 1일 때 위로 얼마인가. 0이면 바닥을 기고 1에 가까우면 제자리에서 솟는다 |

---

## Implementation Units

### U1. 스윙 넉백 상수와 `Pet::swing_knocked`

**Goal** — 맞은 이웃 한 마리가 앞쪽 비스듬히 위로 날아가는 진입점을 만든다.
**Requirements** — R4, R5, R8.
**Dependencies** — 없음.
**Files** — `src-tauri/src/pet/tuning.rs`, `src-tauri/src/pet/motion/react.rs`,
`src-tauri/src/pet/motion/react_tests.rs`.
**Approach** — `tuning.rs`의 "── 착지 ──" 절 다음에 "── 스윙 넉백 ──" 절을 새로 만들고 위 표의
상수 넷과 `const assert!`를 넣는다. `react.rs`에 `Pet::swing_knocked(now_ms, forward,
world_width)`를 추가한다 — 방향은 `(forward.signum(), -SWING_KNOCK_LIFT)`를 정규화하고,
속도는 `(world_width * SWING_KNOCK_WORLDS_PER_SEC).max(THROW_MIN_SPEED)`, 마지막에
`enter(Behavior::Thrown, now_ms)`. `whack_seq`는 건드리지 않는다.
**Patterns to follow** — `Pet::bowling_knocked`(`motion/bowling.rs`)와 `Pet::flip`
(`motion/pinball.rs`)이 같은 꼴이다: 단위벡터 × 세계 비례 속도 → `facing` 갱신 → `Thrown`.
**Execution note** — 상수 관계(`SWING_REACH > PET_SIZE`)는 `const assert!`가 컴파일 시각에
잡으므로 테스트로 다시 쓰지 않는다.
**Test scenarios**
- `방망이에_맞으면_앞으로_날아간다` — 오른쪽으로 넉백하면 `Thrown`이고 `vx > 0`, 몇 틱 뒤
  x가 늘어 있다.
- `방망이에_맞으면_살짝_떠오른다` — 넉백 직후 위로 뜬다(초기 y보다 작아지는 틱이 있다).
- `맞은_쪽은_방망이를_휘두르지_않는다` — `swing_knocked` 뒤 `whack_seq`가 그대로다.
**Verification** — `cargo test`.

### U2. `Pets::whack` — 스윙 사거리 안의 이웃을 함께 날린다

**Goal** — 여러 마리를 가로지르는 판정을 `Pets`에 얹는다.
**Requirements** — R1, R2, R3, R6, R7, R9.
**Dependencies** — U1.
**Files** — `src-tauri/src/pet/mod.rs`, `src-tauri/src/pet/motion/react_tests.rs`.
**Approach** — `Pets`에 `whack(id, now_ms, world, nx, ny) -> Vec<PetId>`를 추가한다.
**`step_all`·`step_bowling` 본문은 건드리지 않고** `ball_drag_end` 근처에 둔다. 순서는
(1) 대상 마리에 `Pet::whack`을 그대로 넘긴다, (2) 대상이 `Behavior::Swing`이 **아니면**
대상 id만 담아 돌려준다(KTD3 — 빽빽거리기·핀볼이 여기서 걸린다), (3) 맞다면 대상의
중심·`facing`을 읽어 두고 나머지 마리를 id 순으로 훑는다, (4) `Dragged`·볼링 참여
마리는 건너뛴다, (5) 앞쪽 거리 `0..=SWING_REACH`, 위아래 `|dy| <= SWING_REACH_V`면
`swing_knocked`를 부르고 id를 담는다. 스윙한 마리의 위치·방향은 **루프 전에 한 번 읽는다** —
루프 안에서 다시 읽으면 대상 마리를 가변으로 빌리는 중이라 빌림 검사에 걸린다. 날아간
이웃은 다시 훑지 않는다 (R6).
**Patterns to follow** — `Pets::step_bowling`이 `BTreeMap`을 두 번 훑으며 마리 간 판정을
하는 같은 꼴이다(단, 그 본문은 이 PR에서 읽기만 한다).
**Test scenarios**
- `앞에_있는_마리도_같이_날아간다` — 오른쪽을 보는 A 앞에 B. A를 때리면 A는 `Swing`,
  B는 `Thrown`.
- `등_뒤의_마리는_안_날아간다` — 같은 배치에서 B를 때리면(둘 다 오른쪽을 봄) A는 그대로다.
- `사거리_밖의_마리는_안_날아간다` — `SWING_REACH`보다 멀리 놓으면 `Thrown`이 안 된다.
- `한_층_위의_마리는_안_맞는다` — 세로로 `SWING_REACH_V`보다 떨어뜨리면 안 맞는다.
- `날아간_마리가_또_다른_마리를_치지_않는다` — A—B—C 배치에서 A를 때리면 B만 `Thrown`.
- `들고_있는_마리는_안_날아간다` — 앞 마리가 `Dragged`면 그대로 `Dragged`다.
- `핀볼_모드에서는_이웃이_안_날아간다` — 전 마리 `set_pinball(true)` 뒤 때리면 맞은 마리만
  `Thrown`이다.
- `연타로_빽빽거리면_이웃이_안_날아간다` — 스무 번 연달아 때려 `Squawk`로 넘어간 클릭에서는
  이웃이 그대로다.
- `넉백은_난수를_쓰지_않는다` — 이웃이 있는 경우와 없는 경우에 때린 마리의 이후 동작
  시퀀스가 같다.
**Verification** — `cargo test`, 그리고 기존 골든 테스트가 그대로 통과한다.

### U3. 브릿지 배선과 문서

**Goal** — 클릭이 실제로 이웃을 날리고, 날아간 창이 즉시 따라간다.
**Requirements** — R4, 꼬리 작업.
**Files** — `src-tauri/src/pet_bridge/commands.rs`, `TODO.md`, `MOTIONS.md`.
**Approach** — `pet_whack`이 `pets.whack(id, ...)`을 부르고 돌려받은 `Vec<PetId>`를
**`let`으로 먼저 받은 뒤** 각각 `flush`한다 (KTD9). `MOTIONS.md`의 `Swing` 절에 사거리
한 줄을 넣고, `TODO.md`의 체크박스를 체크한다.
**Execution note** — 새 `#[tauri::command]`를 만들지 않으므로 `generate_handler!` 등록
문제는 없다. 커맨드 시그니처도 그대로다.
**Test expectation: none** — 브릿지 배선은 Tauri 런타임이 필요해 단위 테스트로 잡히지
않는다. PR의 수동 확인 체크리스트가 이 유닛의 검증이다.
**Verification** — `cargo test`, `npm test`, `npm run build`, 그리고 PR의 수동 체크리스트.

---

## Verification Contract

| 게이트 | 명령 |
|---|---|
| Rust 단위 테스트 | `cd src-tauri && cargo test` |
| 프론트 단위 테스트 | `npm test` |
| 타입 검사 | `npm run build` |
| 코드 리뷰 | `ce-code-review` (PR 전 필수) |

**개발 스모크(`npm run tauri dev`)는 돌리지 않는다** — `tauri-plugin-single-instance` 때문에
다른 worktree의 에이전트와 동시에 띄울 수 없다 (2026-09-02 사용자 지시). 대신 PR "테스트"
절에 수동 확인 체크리스트를 구체적으로 적는다.

## Definition of Done

- R1~R9가 테스트나 수동 체크리스트로 확인된다.
- 두 러너와 타입 검사가 통과한다.
- `ce-code-review` 지적이 반영되거나, 반영하지 않은 이유가 PR "비고"에 있다.
- `TODO.md` 체크박스가 체크되고 `MOTIONS.md`의 `Swing` 절이 사거리를 말한다.
- 소유 경계 밖 파일(`pinball.rs`, `step_all`/`step_bowling` 본문, `tuning.rs`의 핀볼 절,
  `pet_bridge/tick.rs`)이 diff에 없다.

## Sources & Research

- `TODO.md` — "방망이를 휘두르면 앞에 있는 마리가 날아간다" 항목 본문(설계 제약의 원천).
- 커밋 `815d009` — 마리별 step 루프를 `Pets`로 끌어올린 이음매. `Pet`의 시그니처를 건드리지
  않고 계층을 얹는 규칙의 출처.
- 커밋 `26cb33d` — 볼링. `bowling_knocked`가 "맞으면 `Thrown`" 패턴의 선례다.
- 커밋 `afd03e7` — 던지기 상한 1.4. 세기 순서를 `const assert!`로 묶는 방식의 선례.
- `docs/solutions/best-practices/rust-for-loop-holds-mutex-guard-across-body.md` — KTD9.
- `MOTIONS.md` — `Swing` 행, 핀볼 절, 효과음 표(`whack_seq`에 붙은 "퍽").
