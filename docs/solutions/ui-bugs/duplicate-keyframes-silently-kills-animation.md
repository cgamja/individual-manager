---
title: "같은 이름의 @keyframes를 두 번 정의하면 앞의 애니메이션이 통째로 죽는다"
module: "src/pet/pet.css"
date: 2026-08-31
problem_type: ui_bug
component: styling
severity: high
applies_when:
  - "동작 하나에 CSS 애니메이션을 새로 붙일 때"
  - "`@keyframes` 이름을 동작 이름에서 따올 때 (`pg-<동작>`)"
  - "긴 CSS 파일에 규칙을 이어 붙일 때"
tags:
  - css
  - keyframes
  - silent-failure
  - animation
---

# 같은 이름의 `@keyframes`를 두 번 정의하면 앞의 애니메이션이 통째로 죽는다

## 증상

F3 "벽에서 굴러떨어지기"(PR #27)에서 만든 `Tumble` 그림이 화면에 **한 번도 나온 적이
없었다.** 벽에 박고 뒤로 나자빠졌다 일어나는 대신, 펭귄이 제자리에서 팽이처럼 한 바퀴
돌았다. 그 PR은 두 러너와 타입 검사, 코드 리뷰를 전부 통과했고, 다음 PR(슬라이딩)의
리뷰에서야 잡혔다.

## 원인

`pet.css`에 `@keyframes pg-tumble`이 **두 번** 있었다.

- 723줄 — 굴러떨어지기 자세 (`.pg--tumble .pg-all`이 쓰려던 것)
- 856줄 — 던져졌을 때의 360도 회전 (`.pg--thrown`이 쓰던 것, 먼저 있던 이름)

CSS에서 같은 이름의 `@keyframes`가 여러 번 나오면 **나중 정의가 이긴다.** 그것도 규칙별로
가려지는 게 아니라 그 이름에 대한 참조 전부가 나중 것을 본다. 그래서 `.pg--tumble`도
`.pg--thrown`도 똑같이 360도 회전을 재생했다.

던져짐 회전이 먼저 있었고 이름이 `pg-tumble`이었다 — "구른다"는 뜻으로 지은 이름인데,
나중에 `Tumble`이라는 **동작**이 생기면서 `pg-<동작>` 관례와 정면으로 부딪혔다.

## 안 통한 것

- **두 러너와 타입 검사** — CSS는 어느 쪽도 보지 않는다.
- **`pet.css 커버리지` 테스트** — `.pg--tumble` 선택자가 있는지만 본다. 있었다.
- **`정의된_keyframes가_모두_쓰인다`** — 이름별 등장 횟수가 1보다 큰지 본다.
  중복 정의는 등장 횟수를 **늘리므로** 오히려 더 확실히 통과한다.
- **`동작 길이 동기화`** — 길이(`1.1s`)만 대조한다. 길이는 맞았다. 그림만 달랐다.

## 해결

던져짐 쪽 이름을 `pg-thrown-spin`으로 바꿨다. 동작 이름에서 딴 `pg-tumble`은 그 동작이
가져간다.

그리고 **중복 정의 자체를 막는 가드**를 넣었다 (`pet-css.test.ts`):

```ts
const defined = [...css.matchAll(/@keyframes\s+([\w-]+)/g)].map((m) => m[1]);
const 중복 = defined.filter((n, i) => defined.indexOf(n) !== i);
expect(중복).toEqual([]);
```

## 왜 이게 통하나

이 레포의 CSS 가드는 전부 **"쓰이지 않는 것"**을 찾는 방향이었다(선택자 누락, 죽은
keyframes). 이 사고는 반대 방향이다 — 이름이 **너무 많이** 쓰였다. 등장 횟수를 세는
검사로는 원리상 잡을 수 없고, 정의 목록에서 중복을 직접 봐야 한다.

## 예방

- `@keyframes` 이름은 **그것을 쓰는 클래스에서** 딴다. 던져짐이 쓰는 회전은
  `pg-thrown-spin`이지 `pg-tumble`이 아니다. "무엇을 하는가"로 지으면 나중에 같은 이름의
  동작이 생겼을 때 부딪힌다.
- 애니메이션을 새로 붙일 때 **이름을 먼저 grep**한다. 파일이 1000줄이 넘어 눈으로는 못 본다.
- 이 부류(컴파일·테스트·경고가 전부 통과하고 런타임에서만 조용히 틀리는 것)를 만나면
  **소스를 직접 대조하는 테스트**를 만든다. 커맨드 등록 누락과 같은 처방이다 →
  `docs/solutions/best-practices/tauri-command-registration-silent-failure.md`
- **그림 자체는 렌더 스냅샷으로 못 박는다** (2026-09-03 에셋 리팩터링에서 추가).
  `src/assets/assets.test.ts`가 펭귄 둘과 소품 다섯을 렌더해 마크업을 굳혀 둔다 —
  도형·좌표·겹침 순서가 조용히 바뀌는 것은 이걸로 잡힌다. **다만 이 사고는 여전히
  스냅샷 밖이다**: 스냅샷은 마크업만 보고 CSS는 한 줄도 안 덮으므로, `@keyframes`가
  겹쳐 애니메이션이 죽어도 스냅샷은 미동도 없다. 위 두 검사가 계속 본체다.
- **위 처방을 따라 만든 소스 대조 테스트가 주석에 걸려 헛돌 수 있다** —
  그 함정과 확인 방법은 `docs/solutions/best-practices/source-text-tests-pass-on-comments.md`
