# 지켜야 할 컨벤션

> 관련 문서: [PRD.md](PRD.md), [TODO.md](TODO.md), PR 템플릿: [.github/TEMPLATE/PR.md](.github/TEMPLATE/PR.md)

### 커밋 컨벤션 — Angular Convention
- 타입: `feat`, `fix`, `refactor`, `chore`, `docs` 사용 (필요 시 `test` 추가 가능)
- 언어: 한국어
- 제목: 50자 이내, `타입: 제목` 형식 (ex. `feat: 뽀모도로 타이머 종료 알림 추가`)
- 본문: 한 줄 72자 이내. 무엇을/왜 바꿨는지 위주로 작성
- 커밋은 기능 단위 별로 나누기. 너무 짤막하지 않게 (파일 하나 오타 수정 수준으로 쪼개지 말고, 하나의 의미 있는 변경을 하나의 커밋으로)

### TDD 적용 및 테스트 생성
- 기능 구현 전 실패하는 테스트를 먼저 작성한다 (Red → Green → Refactor)
- 테스트 우선순위:
  1. **핵심 로직** (동기화 검증·재시도 큐, 상태 4단계 전환, 키워드 필터, 타이머): 반드시 단위 테스트 작성
  2. **외부 API 연동** (Jira/Notion/Calendar/Slack/Webex): 실제 API를 호출하지 않고 mock/fixture로 테스트. 응답 파싱과 에러 처리(토큰 만료, rate limit, 네트워크 실패) 케이스를 포함
  3. **UI**: 필수 아님. 수동 확인으로 대체 가능
- 테스트 이름은 한국어로 동작을 설명 (ex. `동기화_검증_실패시_전체_재동기화를_수행한다`)
- PR을 올리기 전 전체 테스트가 통과해야 한다

### 브랜치 전략
- 각 기능을 만들 때 branch를 파서 작업한다
- 브랜치 이름: `타입/기능-설명-번호` (ex. `feat/project-setting-06`)
- 타입은 커밋 타입과 동일하게 사용 (`feat`, `fix`, `refactor`, `chore`, `docs`)
- `main`에 직접 커밋하지 않는다

### PR 생성 및 처리
- 내가 요구했을 때, [.github/TEMPLATE/PR.md](.github/TEMPLATE/PR.md) 템플릿에 맞춰 PR을 연다
- **Merge는 내가 한다** (에이전트/자동화가 merge하지 않는다)
- PR 하나는 하나의 마일스톤 항목([TODO.md](TODO.md) 체크박스) 단위를 넘지 않게 유지

### 문서 최신화
- 기능·범위가 바뀌면 [PRD.md](PRD.md)·[SERVICES.md](SERVICES.md)를 같은 PR에서 함께 수정한다
- 작업이 끝나면 [TODO.md](TODO.md)의 해당 체크박스를 체크한다
- 새로운 원칙이 생기면 [PRINCIPLE.md](PRINCIPLE.md)에 추가한다

### 시크릿 관리
- API 토큰·OAuth 자격증명은 코드·문서·커밋에 절대 포함하지 않는다
- 토큰은 macOS Keychain(또는 `.gitignore`된 로컬 설정 파일)에만 보관한다
