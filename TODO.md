# TODO

> [PRD.md](PRD.md) §10 마일스톤 기준. 각 단계가 끝날 때마다 "혼자 쓰기에 유용한 상태"를 유지한다.

## 시작 전 결정
- [x] 기술 스택 선택 (Tauri / Electron / Swift 네이티브) — PRD Q1 → **Tauri v2 + React/TS 확정** (2026-08-06)
- [ ] 토큰 보관 방식 확정 (macOS Keychain 권장) — PRD Q2
- [ ] 기존 Notion DB 스키마 확인: 상태 속성(완료/일부/미완/기타) 유무, 기존 데이터 호환 — PRD R3

## M1 — 앱 골격 + 뽀모도로
- [x] macOS 메뉴바 상주 앱 골격 (펭귄 아이콘 `penguin-icon.png`)
- [x] 카드형 팝오버 UI 기본 레이아웃
- [x] 뽀모도로 타이머 (25/5 기본, 시간 커스터마이즈)
- [x] macOS 알림 권한 + 타이머 종료 알림

## M2 — Notion TODO
- [ ] Notion Integration 연결 및 대상 Database 지정
- [ ] TODO 조회/생성/수정 카드 UI
- [ ] DB 속성 확장 (카테고리, 시간/소요시간, 세부 메모)
- [ ] 상태 4단계(완료/일부/미완/기타) 처리
- [ ] 동기화 검증 루프 (주기 검증 + 오류 시 전체 재동기화 + 쓰기 재시도 큐)

## M3 — Jira
- [ ] Jira Cloud API 토큰 연결
- [ ] 내 티켓 목록·상세 조회
- [ ] 완료(상태 전환) 처리, 댓글 작성
- [ ] 스프린트 진행상황 뷰
- [ ] 알림: 티켓 할당 / 내 티켓에 댓글 (opt-in)

## M4 — Google Calendar
- [ ] 개인 GCP 프로젝트 + OAuth 연결
- [ ] 일정 등록/수정/삭제/조회
- [ ] "일정으로 등록" 브릿지 (Notion·Jira → Calendar 먼저, 이후 Slack·Webex)
- [ ] 알림: 다가오는 일정 N분 전 (opt-in)

## M5 — Slack
- [ ] Slack App 생성·설치·스코프 승인 실검증 (PRD R2)
- [ ] 선택한 채널 메시지 조회
- [ ] 특정 채널 메시지 전송
- [ ] 알림: 지켜보는 채널 새 메시지 (opt-in)

## M6 — Webex
- [ ] OAuth Integration + refresh token 흐름 구현 (12h 토큰 만료 대응, PRD R1)
- [ ] 공지 스페이스 메시지 모아보기
- [ ] 키워드 필터/하이라이트
- [ ] 알림: 새 공지 / 키워드 매칭 (opt-in)
