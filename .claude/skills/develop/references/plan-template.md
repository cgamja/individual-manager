# 플랜 문서 형식

경로: `docs/plans/YYYY-MM-DD-NNN-<타입>-<슬러그>-plan.md`
(`NNN`은 기존 플랜 중 최대 번호 +1, `<타입>`은 커밋 타입과 동일하게 `feat`/`fix`/`refactor` 등)

기준 예시는 `docs/plans/2026-08-06-001-feat-m1-menubar-pomodoro-plan.md`다. 실제로 구현을 끌고
가는 데 쓰인 문서이므로 분량·서술 밀도의 감을 잡으려면 그걸 먼저 본다.

각 섹션이 왜 있는지를 알고 쓰는 게 중요하다. 형식을 채우는 게 목적이 아니라, **구현 중에 판단이
필요한 순간을 미리 없애는 것**이 목적이다.

---

## Front-matter

```yaml
---
title: <제목> - Plan
type: feat            # 커밋 타입과 동일
date: YYYY-MM-DD
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---
```

## Goal Capsule

구현자가 이 한 덩어리만 읽어도 방향을 잃지 않게 하는 요약. 다섯 줄:

- **목표** — 무엇을 어디까지 만드는지. 근거가 되는 PRD 조항을 명시한다.
- **권위 순서** — `PRD > PRINCIPLE > CONVENTIONS > 이 플랜`. 충돌 시 상위가 이기고 플랜과
  어긋나면 멈춘다는 것을 못박는다.
- **실행 프로필** — 브랜치 이름, TDD 적용 범위, 커밋 컨벤션.
- **정지 조건** — 어떤 상황에서 구현을 멈추고 보고할지. 구체적으로 쓴다. 이걸 미리 정해두면
  막혔을 때 "조금만 더 해보자"로 시간을 태우지 않는다.
- **꼬리 작업** — PR 템플릿 사용, merge는 사용자, 같은 PR에 포함할 문서 갱신.

## Product Contract

**Summary** — 이 작업이 끝나면 사용자에게 무엇이 가능해지는지. 기능 목록이 아니라 사용 장면으로.

**Problem Frame** — 왜 지금 이걸 하는지, 이게 없으면 뒤에 뭐가 막히는지.

**Requirements** — `R1`, `R2`... 로 번호를 매긴 검증 가능한 요구사항. 구현 방법이 아니라
관찰 가능한 동작으로 쓴다. 뒤의 Implementation Unit이 이 번호를 참조한다.

**Acceptance Examples** — `AE1`... Given/When/Then. 수동 검증 시나리오가 되므로 실제로 재현할 수
있게 구체적으로 쓴다 ("약 15:00(±수 초)"처럼 판정 기준까지).

**Scope Boundaries** — 비목표(PRD §4 근거)와 **Deferred to Follow-Up Work**를 나눠 적는다.
Deferred는 PR "비고"와 `TODO.md`로도 옮겨야 잊히지 않는다.

## Planning Contract

**Key Technical Decisions** — `KTD1`... 이 플랜의 가장 중요한 부분이다. 각 항목에
**무엇을 정했는지 + 왜 그게 아니면 안 되는지**를 쓴다. 특히 리서치로 알아낸 제약(알려진 버그,
플랫폼 동작, API 한계)은 이슈 번호나 출처와 함께 남긴다. 나중에 "이거 왜 이렇게 했지?" 하고
되돌리다 같은 함정을 다시 밟는 걸 막아준다.

**Assumptions** — 확인 없이 채택한 추정. 틀리면 구현 전에 알려달라고 명시한다. 사용자가 플랜을
검토할 때 가장 값싸게 교정할 수 있는 지점이다.

**High-Level Technical Design** — mermaid 다이어그램. 모듈 경계와 데이터 흐름을 보인다.
상태를 다루면 `stateDiagram-v2`도 함께.

## Implementation Units

`U1`, `U2`... 각 유닛이 커밋 하나에 대응한다. 유닛마다:

- **Goal** — 이 유닛이 끝났을 때 동작하는 것
- **Requirements** — 커버하는 R·KTD 번호
- **Dependencies** — 선행 유닛
- **Files** — 만들거나 고칠 파일 목록
- **Approach** — 구현 방침. 함정 회피책은 여기에 구체적으로
- **Test scenarios** — 쓸 테스트를 한국어 이름 감각으로 나열. 테스트하지 않는 유닛은
  `Test expectation: none — <이유>`라고 명시한다. 빠뜨린 것과 의도적으로 뺀 것을 구분하기 위해서다
- **Verification** — 통과 판정 방법 (명령 또는 수동 시나리오)

유닛 순서는 의존성 순이면서, 가능하면 **외부 의존성이 없는 순수 로직을 먼저** 둔다. 그래야
API 연결이 막혀도 테스트 가능한 코어가 먼저 완성된다.

## Verification Contract

게이트 표: 무엇을 / 어떤 명령으로 / 어느 유닛에 적용하는지.
`cargo test`와 `npm test` 둘 다 들어가야 한다. 알림·번들 관련이면 `npm run tauri build`도.

## Definition of Done

체크 가능한 완료 조건. 보통:

- R1~Rn 충족, AE1~AEn 재현 확인
- 두 테스트 러너 전체 통과, 핵심 로직은 테스트가 먼저 작성된 커밋 이력
- 컨벤션에 맞는 브랜치·커밋, PR 템플릿으로 오픈 (merge는 사용자)
- 관련 문서 갱신이 같은 PR에 포함
- 실험하다 버린 코드·미사용 잔재가 diff에 없음
