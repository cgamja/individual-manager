---
title: 우클릭으로 펭귄 추가·삭제 - Plan
type: feat
date: 2026-08-31
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# 우클릭으로 펭귄 추가·삭제 — Plan

## Goal Capsule

- **목표** — 펭귄 한 마리를 전제로 짜인 상태·창·커맨드를 **N마리로 연다.** 사용자는
  우클릭한 펭귄 옆에 새 펭귄을 부르고, 우클릭한 펭귄을 지운다.
  범위: `TODO.md` "펭귄 마릿수"의 **우클릭으로 펭귄 추가·삭제** 체크박스 하나.
- **권위 순서** — `PRD > PRINCIPLE > CONVENTIONS > MOTIONS > 이 플랜`.
- **실행 프로필** — 브랜치 `feat/pet-add-remove-01`. 컬렉션 로직·라벨 파싱·시드 유도는
  `pet.rs`(Tauri 무의존)에 두고 **TDD**. 창 생성·틱은 수동 검증.
- **정지 조건**
  1. 새 창의 커맨드가 조용히 reject된다 → `capabilities`의 `pet-*` 글롭을 먼저 의심한다.
     이 레포에 이미 기록된 함정이다.
  2. 펭귄 N마리에서 20Hz 틱이 유휴 CPU를 눈에 띄게 먹는다(창마다 웹뷰가 하나씩 붙는다).

---

## Problem Frame

지금은 **펭귄 한 마리 = 전역 상태 하나**다.

| 자리 | 현재 | 문제 |
|---|---|---|
| `PetState(Mutex<Pet>)` | `Pet` 하나 | 두 마리를 담을 곳이 없다 |
| `PET_LABEL = "pet"` | 고정 라벨 | 창을 하나만 만들 수 있다 |
| 커맨드 `State<PetState>` | 대상이 암묵적 | 어느 펭귄을 때렸는지 알 수 없다 |
| 틱 스레드 | 창 하나·경계 하나 | 마리마다 다른 화면·경계를 못 본다 |
| `flush(app)` | 창 하나에 emit | 마리마다 자기 창에 보내야 한다 |

---

## Requirements

- **R1** — 펭귄을 여러 마리 띄운다. 마리마다 자기 창·자기 상태·자기 경계를 갖는다.
- **R2** — 우클릭한 펭귄 옆에서 **추가**하고, 우클릭한 펭귄을 **삭제**한다.
- **R3** — **마지막 한 마리는 삭제할 수 없다.** 전부 없애는 것은 기존 on/off의 일이고,
  두 장치가 같은 일을 다투면 안 된다 (PRD §5.5).
- **R4** — 마리마다 **다른 시드**를 갖는다. 같은 시드면 똑같이 움직여 복제처럼 보인다.
- **R5** — 마릿수는 저장되어 다시 켜도 유지된다. 저장은 `enabled`를 덮어쓰지 않는다.
- **R6** — 클릭·드래그·빠따는 **그 펭귄에게만** 간다.
- **R7** — 컬렉션 로직은 Tauri 무의존으로 남아 테스트된다 (PRINCIPLE 3·4).

---

## Key Technical Decisions

**KTD1 — 대상 펭귄은 "호출한 창의 라벨"로 정한다. 웹뷰가 id를 보내지 않는다.**
웹뷰가 `pet_whack(id)`처럼 보내면 틀린 id를 보낼 수도, 남의 펭귄을 조작할 수도 있다.
Tauri가 커맨드에 주입해 주는 `WebviewWindow`의 라벨이 곧 신원이다 — 위조할 수 없고
인자도 늘지 않는다.

**KTD2 — 라벨은 `pet-<id>`, id는 증가만 하고 재사용하지 않는다.**
지웠다 만든 자리에 같은 id를 다시 쓰면, 닫히는 중인 창과 새 창이 같은 라벨을 다퉈
`set_position`이 엉뚱한 창으로 간다.

**KTD3 — 저장하는 것은 마릿수뿐이다. 펭귄 개체를 저장하지 않는다.**
펭귄은 이름도 성장도 없다(PRD 비목표: 게임화). 다시 켜면 저장된 수만큼 새로 만들면 된다.
저장은 **읽고-고쳐-쓰기**로 한다 — `pet` 키 아래 `enabled`가 함께 살아서, 통째로
덮어쓰면 on/off 설정이 날아간다.

**KTD4 — 상한 8마리.** 창 하나가 웹뷰 하나이고 각각 수십 MB를 쓴다. 사용자가 고른
마릿수를 막지 않되, 실수로 눌러 100마리가 되는 길은 닫는다.

**KTD5 — 새 펭귄은 부른 펭귄 옆에 나타난다.**
전부 같은 자리에서 시작하면 겹쳐서 한 마리로 보이고, 무작위로 흩뿌리면 어디서 생겼는지
모른다. "얘가 하나 더 불렀다"가 눈에 보이는 편이 낫다.

**KTD6 — 우클릭 대상은 팝오버가 뜨기 전에 기록한다.**
삭제 버튼은 팝오버(=`main` 창) 안에 있는데, 팝오버는 자기가 어느 펭귄 때문에 열렸는지
모른다. `pet_open_popover`가 **마지막으로 우클릭된 id**를 남기고 팝오버가 그것을 읽는다.

---

## Implementation Units

### U1. 코어 — 펭귄 컬렉션 (`pet.rs`)

**Goal** — `Pet` 하나를 담던 자리를 id로 관리되는 컬렉션으로 바꾼다. Tauri 무의존.

**Requirements** — R1, R3, R4, R7

**Files** — `src-tauri/src/pet.rs`

**Approach**
- `pub type PetId = u32`, `pub struct Pets { pets: BTreeMap<PetId, Pet>, next_id: PetId }`.
  `BTreeMap`은 순회 순서가 안정적이라 틱마다 같은 순서로 돈다.
- `Pets::add(seed_base, now_ms, bounds, start_x) -> Option<PetId>` — 상한(8)에 걸리면 `None`.
- `Pets::remove(id) -> bool` — **마지막 한 마리면 거부**하고 `false`.
- `Pets::get_mut(id)`, `ids()`, `len()`.
- 시드는 `seed_base`와 id를 섞어 만든다 — 같은 시드가 두 번 나오지 않게.
- `Pet::new_at(seed, now_ms, bounds, x)` 추가. 기존 `new`는 `bounds.left`로 위임한다.

**Execution note** — 실패 테스트 먼저.

**Test scenarios**
- `펭귄을_추가하면_새_id를_받는다`
- `지운_id는_다시_쓰이지_않는다` — 추가·삭제·추가 후 id가 겹치지 않는다 (KTD2)
- `마지막_한_마리는_삭제되지_않는다` (R3)
- `상한을_넘겨_추가하면_거부된다` (KTD4)
- `마리마다_시드가_달라_다르게_움직인다` — 같은 시각·같은 경계로 여러 틱을 돌렸을 때
  두 마리의 스냅샷이 언젠가 갈라진다 (R4)
- `새_펭귄은_지정한_x에서_시작한다` (KTD5)

**Verification** — `cargo test` 통과, 새 테스트 6개.

### U2. 브릿지 — 창·틱·커맨드를 마리별로 (`pet_bridge.rs`)

**Goal** — 창을 id별로 만들고, 틱이 전부를 돌고, 커맨드가 호출한 창의 펭귄에 붙는다.

**Requirements** — R1, R2, R5, R6

**Dependencies** — U1

**Files** — `src-tauri/src/pet_bridge.rs`

**Approach**
- `pet_label(id) -> String` / `pet_id_from_label(&str) -> Option<PetId>` — 순수 함수, 테스트.
- `PetState(Mutex<Pets>)` + 마지막 우클릭 id (KTD6).
- 틱: 살아 있는 id를 돌며 각자의 창·경계·`last_look`을 쓴다. 경계 캐시와 `last_look`은
  **id별 맵**이 된다. 창이 사라진 id는 컬렉션에서 정리한다.
- 커맨드에 `window: WebviewWindow`를 받아 라벨에서 id를 얻는다 (KTD1).
- `pet_add` — 부른 펭귄 옆 좌표를 계산해 추가하고 창을 만든다. 마릿수를 저장한다.
- `pet_remove` — 컬렉션에서 지우고(마지막이면 거부) 창을 닫는다. 마릿수를 저장한다.
- `pet_focused` — 팝오버가 "지금 어느 펭귄 이야기인지" 읽는다.
- 마릿수 저장은 `pet` 객체를 읽어 `count`만 갈아끼운다 (KTD3).

**Test scenarios**
- `라벨에서_펭귄_id를_뽑는다` / `펫이_아닌_라벨은_id가_없다` (`main`, `pet`, `pet-x`)
- `모든_펫_커맨드가_invoke_handler에_등록되어_있다` — 기존 테스트를 새 커맨드까지 넓힌다
  (등록 누락은 런타임에서만 조용히 reject된다)

**Verification** — `cargo test` 통과.

### U3. 셸 — 시작 시 N마리, capabilities, 커맨드 등록 (`lib.rs`)

**Goal** — 저장된 마릿수만큼 창을 만들고, 새 커맨드를 등록하고, 새 라벨에 권한을 준다.

**Requirements** — R1, R5

**Dependencies** — U2

**Files** — `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`

**Approach**
- `capabilities`의 `windows`를 `["main", "pet-*"]`로. **이걸 빠뜨리면 새 창의 커맨드가
  컴파일·테스트를 모두 통과하고 런타임에서만 조용히 reject된다.**
- setup에서 저장된 마릿수(기본 1, 1~8로 조임)만큼 만든다.
- `pet_add`·`pet_remove`·`pet_focused`를 `generate_handler!`에 등록.

**Test scenarios** — `Test expectation: none` — setup은 실행 중인 앱이 필요하다.
U2의 등록 테스트가 누락을 잡는다.

**Verification** — 앱을 띄워 저장된 마릿수대로 뜨는지 본다.

### U4. 웹뷰 — 추가·삭제 버튼 (`src/`)

**Goal** — 우클릭으로 연 설정 창에서 펭귄을 부르고 지운다.

**Requirements** — R2, R3

**Dependencies** — U3

**Files** — `src/lib/pet.ts`, `src/components/SettingsCard.tsx`(또는 새 카드), `src/App.tsx`,
각 `*.test.tsx`

**Approach**
- `addPet()` / `removePet()` / `focusedPet()` 래퍼.
- 설정에 "펭귄 추가" / "이 펭귄 삭제" 두 버튼. **마지막 한 마리면 삭제 버튼을 비활성**하고
  이유를 옆에 적는다 — 눌리는데 아무 일도 없으면 고장으로 읽힌다.
- 상한에 닿으면 추가 버튼을 비활성한다.

**Test scenarios**
- `마지막_한_마리면_삭제_버튼이_비활성된다`
- `상한에_닿으면_추가_버튼이_비활성된다`
- `추가를_누르면_addPet을_호출한다`

**Verification** — `npm test`·`npm run build` 통과. 실제로 눌러 늘고 주는지 확인.

### U5. 문서

**Files** — `TODO.md`(체크), `PRD.md`(상한 8·시작 위치 반영), `MOTIONS.md`(필요 시)

---

## Scope Boundaries

**Deferred to Follow-Up Work**
- **F2 다중 화면** — 마리마다 다른 모니터에 있을 수 있는데, 경계는 여전히 창의
  `current_monitor()` 하나를 본다. 이 PR은 그 구조를 마리별로 넓히기만 하고
  좌표계 자체는 F2가 바꾼다.
- **펭귄끼리 상호작용** (부딪히기·따라다니기) — 범위 밖.
- **개체 저장** (이름·위치 기억) — 게임화라 비목표.

---

## Risks

| 리스크 | 대응 |
|---|---|
| `capabilities` 글롭 누락 → 새 창 커맨드가 조용히 reject | U3에서 먼저 고치고, 두 마리째에서 클릭이 먹는지 즉시 확인 |
| 창이 닫히는 중에 틱이 그 창을 만짐 | 틱은 `get_webview_window`가 `None`이면 그 id를 정리하고 넘어간다 |
| N마리에서 20Hz × N 블로킹 `current_monitor()` | 경계 캐시를 id별로 두고 2초 주기를 유지한다 |
| 웹뷰 메모리 | 상한 8 (KTD4) |

---

## Definition of Done

- [ ] `cargo test`·`npm test`·`npm run build` 통과
- [ ] 앱에서 추가·삭제가 동작하고, 두 마리째도 클릭·드래그가 먹는다
- [ ] 마지막 한 마리는 삭제되지 않는다
- [ ] 껐다 켜도 마릿수가 유지되고 `enabled`가 살아 있다
- [ ] `ce-code-review` 지적 반영, 문서 최신화
