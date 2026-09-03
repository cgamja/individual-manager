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
src/
  assets/             **그림과 색은 전부 여기 있다** — 로직 파일에 SVG를 두지 않는다
    palette.ts        모든 색. 부위별 이름 하나씩
    penguin/          index(조립·겹침 순서) · body · hula(훌라 차림) · gear(방망이·낚시)
                      hit(클릭 판정 상자 — Rust와 같은 수를 들고 있다)
    props/            소품 — bat(커서 방망이) · bowling-ball · beach-ball · court
                      **React를 쓰지 않는다** (바닐라 창이 쓴다)
    assets.test.ts    그림 렌더 스냅샷 + props의 React 무의존 검사
  pet/                펭귄 창 웹뷰 — PetApp.tsx, sound·synth
    css/              동작별 스타일 (base·ground·air·react·pinball·drag·
                      fishing·freakout·rest·speech·bowling·volleyball)
                      — index.css가 묶는다
  pinball/            핀볼 판 창 — 화면을 덮는 투명 창, 커서만 방망이
  ball/               볼링 공 창 — 집어서 굴린다
  volley/             비치발리볼 코트·비치볼 창 — 그림뿐이고 클릭을 통과시킨다
  components/         설정 창 카드 UI
  lib/                Rust invoke·이벤트 래퍼 (pet, settings)
src-tauri/src/
  pet/                펭귄 코어 — Tauri 무의존 순수 상태머신
    tuning.rs         속도·길이·확률 상수 — 값을 바꾸려면 여기만
    behavior.rs       동작 목록(Behavior) + 국면 enum
    world.rs          펭귄이 다닐 영역
    motion/           동작 하나가 파일 하나 (ground·air·react·drag·
                      pinball·fishing·freakout·bowling·volleyball)
    mod.rs            Pet·Pets·step 디스패치·pick_next·enter·clamp·난수
  pet_bridge/         Tauri 연결 — settings·window·pinball·ball_window·
                      volleyball·bounds·tick·popover·commands
  lib.rs              setup: 트레이 생성, Accessory 정책, 플러그인 등록, 창 이벤트
docs/plans/           마일스톤 항목별 구현 플랜
docs/solutions/       재발 방지용 학습 기록 — 셸을 건드리기 전에 읽는다
```

**테스트는 구현과 다른 파일이다** — `#[path = "*_tests.rs"]`로 붙인다. Rust 단위 테스트는
비공개 항목을 봐야 해서 같은 crate 안에 있어야 하고 `tests/` 통합 테스트로는 안 된다.
프론트는 `*.test.ts(x)`가 같은 폴더에 있다.

**모션 하나는 일곱 자리에 흩어져 있다** — `behavior.rs`의 모듈 문서에 목록이 있다.
새 모션을 얹을 때 CSS(`pg--*`)와 `pet-css.test.ts`의 `ALL_BEHAVIORS`를 빠뜨려도
Rust는 아무 말도 하지 않는다.

스택은 Tauri v2 + React 19 + TypeScript + Vite 7 (PRD Q1 확정). Rust는 단일 crate `penguin`.

## 반드시 지키는 규칙 (CONVENTIONS.md 요약)

- **`main`에 직접 커밋하지 않는다.** 브랜치는 `타입/기능-설명-번호` (예: `feat/f3-ice-fishing-01`).
- **커밋은 한국어 Angular 컨벤션**, `타입: 제목` 50자 이내, 기능 단위로 묶는다.
- **주석은 WHAT을 간단히.** 결정 과정·실패 이력은 주석이 아니라 `MOTIONS.md`·`TODO.md`·
  `docs/solutions/*`에 두고 한 줄로 가리킨다 (2026-09-02·09-03 사용자 지시, 두 번 받았다).
- **TDD** — 핵심 로직(`pet/motion/*`의 상태 전이·경계 판정)은 실패 테스트 먼저.
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
   설정(펭귄 on/off·대사 목록·소리 on/off·음량·테마·핀볼) 말고는 아무것도 저장하지 않는다.

## 이 코드베이스의 함정

`docs/solutions/`에 기록된 것들 — 메뉴바 셸(`lib.rs`)·브릿지(`pet_bridge/`)를 수정하기
전에 해당 문서를 읽는다.

- **`app.hide()`를 호출하지 않는다.** macOS 26(Tahoe)에서 트레이 아이콘까지 사라진다.
  창 숨김은 `window.hide()`로 충분하다. → `docs/solutions/ui-bugs/macos-tahoe-app-hide-removes-tray-icon.md`
- **숨겨진 웹뷰의 JS 타이머는 ~5분 뒤 멈춘다.** 주기 작업의 진실 원천은 타임스탬프이고
  권위 있는 틱은 Rust 스레드가 소유한다. `setInterval` 감산 방식 금지.
- **트레이는 `setup()`에서 동기 생성**해야 마우스 이벤트를 받는다. `on_tray_icon_event`에서는
  `positioner::on_tray_event`를 항상 먼저 호출한다.
- **`current_monitor()`는 이벤트 루프를 왕복하는 블로킹 호출이다.** 20Hz 틱에서 매번
  부르지 않는다 — 현재는 주기적으로 캐시한다. **읽기에 실패했을 때 낡은 캐시를 붙들면
  안 된다**: 창이 어떤 화면에도 안 걸치면(= 모니터를 뽑으면) `None`이 오는데, 그때
  갱신을 건너뛰면 펭귄이 사라진 좌표로 영원히 clamp되어 다시는 안 보인다. 못 읽으면
  주 모니터로 떨어진다 (`world_to_cache`).
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
- **창 순서는 클릭할 때마다 바뀐다 — 순서가 아니라 레벨로 잡는다.** 핀볼 판은 펭귄
  창보다 아래여야 클릭이 펭귄에게 가는데, 만들 때 한 번 올려 두는 방식
  (`set_always_on_top` 껐다 켜기, `orderFrontRegardless`)은 **전부 실패했다** —
  판을 한 번 클릭하면 도로 올라온다. `ns_window()`로 판의 레벨을 펭귄(3)보다
  낮은 2로 내린다. 증상은 "펭귄이 안 날아간다" 하나뿐이고 테스트·로그가 전부
  깨끗해서 원인이 안 보인다. →
  `docs/solutions/ui-bugs/macos-window-order-is-not-stable-level-is.md`
- **`for x in <락>.f()`는 가드를 루프 내내 붙든다 — 본문이 같은 락을 다시 잡으면
  자기 데드락이다.** 반환값이 소유 `Vec`이어도 그렇다: 사는 것은 반환값이 아니라
  임시 `MutexGuard`이고, `for`의 반복자 식은 `match` 주사식이라 본문이 끝나야 죽는다.
  커맨드 대부분이 마지막에 `flush`/`flush_ball`을 부르는데 그 둘이 락을 다시 잡으므로,
  **락에서 꺼낸 것을 순회할 때는 반드시 `let`으로 먼저 받는다.** 증상은 "버튼을
  누르면 앱이 통째로 멈춘다" 하나뿐이고 두 러너·타입 검사·번들 빌드가 전부 통과한다.
  → `docs/solutions/best-practices/rust-for-loop-holds-mutex-guard-across-body.md`
- **`ns_window()` 아래로 내려간 순간부터는 반드시 메인 스레드다.** AppKit 객체를
  20Hz 틱 스레드에서 만지면 **앱이 흔적 없이 죽는다** — 패닉도, `RunEvent::Exit`도,
  로그 한 줄도 안 남고 프로세스가 증발한다("판을 잘 열었다"는 로그 **직후에** 사라진다).
  KTD5의 *"`set_position`은 어느 스레드에서 불러도 안전하다"*는 **Tauri API에 한한**
  이야기이고, `ns_window()`로 꺼낸 포인터를 직접 만지는 것은 그 디스패치를 건너뛴다.
  같은 함수라도 **호출 자리가 커맨드냐 틱이냐로 갈린다** — 핀볼 판(`sink_pinball_below_pets`)은
  커맨드에서 불려 멀쩡했고, 그걸 베껴 온 코트(`sink_court_below_pets`)는 틱에서 불려
  죽었다. 진단의 첫 수는 `.run(|_, event|)`로 `RunEvent`를 전부 찍어 보는 것이다 —
  종료 이벤트가 **안 뜨는 것**이 "정상 종료가 아니다"의 증거다. →
  `docs/solutions/best-practices/appkit-from-tick-thread-kills-the-app.md`
- **`set_ignore_cursor_events`는 비동기다 — 호출 직후에 읽으면 `false`다.** `Ok(())`를
  즉시 주지만 적용은 이벤트 루프를 왕복한 뒤다(`set_position`과 같은 성질). 직후에
  읽어 확인하면 "이 API는 `always_on_top` 창에서 안 먹는다"는 **오답**이 나온다 —
  실제로는 2초 뒤에 읽으면 `true`고 창은 처음부터 정상이었다. 창을 `visible(false)`로
  만들고 → 플래그를 걸고 → `show()` 하면 간극이 가장 좁아지지만 **한 프레임은 남을
  수 있고 CSS로는 못 메운다** — `pointer-events: none`은 웹뷰가 반응하지 않게 할 뿐
  네이티브 창은 클릭을 그대로 먹는다. 클릭 통과와 창 레벨은 서로를 대신하지 못하므로
  **둘 다** 건다. →
  `docs/solutions/best-practices/tauri-ignore-cursor-events-is-async.md`
- **macOS의 클릭 통과는 창 단위다 — "이 부분만 통과"가 없다.** 흉내 내려면 커서를
  따라 껐다 켜야 하고, 그러면 **통과 중에는 웹뷰에 이벤트가 안 와서 스스로 되돌릴 수
  없다** — 웹뷰가 요청하고 Rust 틱만 되돌린다. 되돌리는 눈이 하나뿐이라 **그 눈이
  멀면 영영 못 누른다.** `pointer-events`와 창 통과는 **다른 층**이고
  (`opacity: 0`은 히트 테스트를 **안** 막는다), 둘의 경계가 어긋나면 "웹뷰는
  반응하는데 창은 통과시키는" 갈래가 생긴다. →
  `docs/solutions/best-practices/macos-click-through-is-per-window.md`
- **그림은 두 러너·타입 검사·리뷰를 전부 통과하면서 바뀔 수 있다.** 그래서
  `src/assets/assets.test.ts`가 **렌더 스냅샷**으로 못 박아 뒀다 — 펭귄 둘(암·수)과
  소품 다섯. **`-u`로 덮는 것은 "그림을 바꾸겠다"는 선언**이지 통과시키는 방법이
  아니다. **다만 스냅샷은 마크업만 본다** — CSS는 한 줄도 안 덮으므로 중복
  `@keyframes`로 애니메이션이 죽는 것은 여전히 스냅샷 밖이다.
- **소스 텍스트를 읽는 검사는 주석에 걸려 헛돈다.** 호출을 주석 처리해도 이름이
  남아 통과한다. **새로 쓴 소스 대조 검사는 반드시 돌연변이로 한 번 빨갛게 만들어
  본다** — 에셋 리팩터링에서 이 방법으로만 일곱 개가 헛돈다는 게 드러났다. →
  `docs/solutions/best-practices/source-text-tests-pass-on-comments.md`
- **CSS `cursor`는 키워드를 목록 맨 끝에만 허용한다.** `cursor: var(--x, grab), grab`은
  `--x`가 없을 때 `grab, grab`이 되는데 **무효라 선언이 통째로 버려진다** — 대체값이
  아무것도 막지 못하는데 보기에는 방어처럼 읽힌다. 값 전체(`url(…) 10 30, grab`)를
  프로퍼티에 담고 `cursor: var(--x, grab)`으로 받는다.
- **화면을 넘나드는 좌표는 배율부터 의심한다.** 창 하나로 여러 화면을 덮으면 그 창은
  배율 하나만 쓰므로 배율이 다른 화면에서 어긋난다. 화면마다 창을 따로 만든다.
- **사용자를 막는 기능에는 나가는 문이 둘 있어야 한다.** 핀볼 판은 화면 전체의 클릭을
  먹으므로 되돌리는 길이 우리 코드에만 있으면 그 코드가 망가졌을 때 맥을 못 쓴다.
  트레이가 두 번째 문인 근거는 macOS의 창 레벨(메뉴바 24 > `always_on_top` 3)이다.
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

**F3의 모션은 전부 들어갔다** — 굴러떨어지기·얼음낚시·슬라이딩·빽빽거리기·발작.
빈도 재조정, **핀볼 모드**(PRD §5.8), **효과음**(Web Audio 직접 합성, 일곱에서만 —
`MOTIONS.md` 효과음 절)까지 들어가 `MOTIONS.md`의 "넣을 동작" 목록은 비었고,
**`PRD.md` §9에 미정 오픈 퀘스천이 하나도 없다** (Q9가 마지막이었다).

마일스톤 항목 하나를 플랜부터 PR까지 끌고 가려면 `develop` 스킬을 쓴다.
