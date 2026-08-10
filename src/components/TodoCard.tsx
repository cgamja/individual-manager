import { Fragment, useState } from "react";
import type { TodoItem, TodoSnapshot } from "../lib/notion";

/** 행 만들기 폼이 제출하는 파라미터 — lib/notion createTodoRow와 동일 형태.
 * 행 생성은 미래 [TODO] 전용이라 날짜 하나만 받는다. */
export interface CreateRowFormParams {
  start: string;
}

/** 행 만들기 제출 결과 — 카드가 폼 접힘/유지·기존 행 안내를 결정하는 데 쓴다. */
export type CreateRowFormResult =
  | { state: "created" }
  | {
      state: "exists";
      page_id: string;
      title: string;
      date: string;
      /** 겹친 행의 현재 수행도 — "기존 행 열기"가 그대로 넘긴다 (KTD1). */
      performance: string | null;
    }
  | { state: "failed" };

/** 추가 시 고를 수 있는 카테고리 — 페이지 본문의 헤딩 텍스트와 일치해야 한다. */
const ADD_CATEGORIES = ["공부", "기타"];
const DEFAULT_CATEGORY = "공부";

/** 수행도 4단계 — Rust `PERFORMANCE_OPTIONS`의 표시용 사본이다 (KTD2).
 * 권위는 Rust 상수이고 어긋나도 커맨드 가드가 최종 방어선이다. 순서는 달성도 순. */
const PERFORMANCES = ["완료", "일부", "미완", "기타"];

interface TodoCardProps {
  /** null = 첫 로드 전 ("불러오는 중…"). 스냅샷이 있으면 busy 중에도 목록을 유지한다. */
  snapshot: TodoSnapshot | null;
  /** todo 커맨드 진행 중 — 추가·토글·편집·새로고침·만들기를 모두 잠근다. */
  isBusy: boolean;
  onRefresh: () => void;
  onCreatePage: () => void;
  /** resolve가 true(반영 성공)일 때만 입력을 비운다 — 실패·안내(notice) 시 유지.
   * category는 선택된 헤딩 텍스트 — 해당 헤딩 아래에 삽입된다. */
  onAdd: (text: string, category: string) => Promise<boolean>;
  /** checked는 토글 후 원하는 값이다. */
  onToggle: (blockId: string, checked: boolean) => void;
  /** resolve가 true일 때만 편집 모드를 닫는다 — 실패 시 입력을 유지해 재시도. */
  onEdit: (blockId: string, text: string) => Promise<boolean>;
  /** created일 때만 폼을 접는다 — exists는 기존 행 안내, failed는 입력 유지. */
  onCreateRow: (params: CreateRowFormParams) => Promise<CreateRowFormResult>;
  /** resolve가 true(열기 성공)일 때만 폼을 접는다.
   * performance는 exists가 실어 준 그 행의 값 — 열기 스냅샷에 그대로 실린다 (KTD1). */
  onOpenPage: (
    pageId: string,
    title: string,
    date: string,
    performance: string | null,
  ) => Promise<boolean>;
  /** 수행도 즉시 저장 (R3) — 카드는 같은 값 재클릭을 걸러서(R4) 호출하지 않는다. */
  onSetPerformance: (performance: string) => void;
}

/** 오늘 할 일 카드 — 오늘 페이지의 to_do 블록 조회·추가·토글·편집 + [TODO] 행 만들기·날짜 전환. */
export function TodoCard({
  snapshot,
  isBusy,
  onRefresh,
  onCreatePage,
  onAdd,
  onToggle,
  onEdit,
  onCreateRow,
  onOpenPage,
  onSetPerformance,
}: TodoCardProps) {
  const [addRaw, setAddRaw] = useState("");
  const [addCategory, setAddCategory] = useState(DEFAULT_CATEGORY);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editRaw, setEditRaw] = useState("");

  // 행 만들기 폼 — 입력 초안만 로컬 state로 둔다
  const [formOpen, setFormOpen] = useState(false);
  const [rowStart, setRowStart] = useState("");
  const [existsInfo, setExistsInfo] = useState<{
    page_id: string;
    title: string;
    date: string;
    performance: string | null;
  } | null>(null);

  // 날짜가 있는 스냅샷(no_page·loaded)에서만 폼·전환 UI가 의미가 있다
  const snapshotDate =
    snapshot !== null && snapshot.state !== "not_connected" ? snapshot.date : null;
  const notToday =
    snapshot !== null && snapshot.state !== "not_connected" && !snapshot.is_today;

  const handleAdd = async () => {
    if (isBusy) return;
    const text = addRaw.trim();
    // 빈 텍스트는 no-op — 삭제가 비목표라 앱에서 지울 수 없는 빈 항목을 만들지 않는다
    if (text === "") return;
    try {
      if (await onAdd(text, addCategory)) setAddRaw("");
    } catch {
      // 커맨드 reject — 입력을 유지해 재시도할 수 있게 둔다
    }
  };

  const startEdit = (item: TodoItem) => {
    setEditingId(item.id);
    setEditRaw(item.text);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditRaw("");
  };

  const commitEdit = async () => {
    if (isBusy || editingId === null) return;
    const text = editRaw.trim();
    if (text === "") return; // 빈 텍스트 저장 no-op (추가 입력과 동일 가드)
    try {
      if (await onEdit(editingId, text)) cancelEdit();
    } catch {
      // reject — 편집 모드·입력을 유지한다
    }
  };

  /** 폼을 접으며 입력을 버린다 — 재열림은 항상 초기 상태다. */
  const closeForm = () => {
    setFormOpen(false);
    setRowStart("");
    setExistsInfo(null);
  };

  const toggleForm = () => {
    if (formOpen) {
      closeForm();
      return;
    }
    // 기본 날짜는 스냅샷의 date — 프론트는 오늘을 계산하지 않는다
    setRowStart(snapshotDate ?? "");
    setFormOpen(true);
  };

  const canSubmit = rowStart !== "";

  const submitRow = async () => {
    if (isBusy || !canSubmit) return;
    setExistsInfo(null);
    try {
      const result = await onCreateRow({ start: rowStart });
      if (result.state === "created") {
        closeForm();
      } else if (result.state === "exists") {
        setExistsInfo({
          page_id: result.page_id,
          title: result.title,
          date: result.date,
          performance: result.performance,
        });
      }
      // failed — 배너는 App이 띄우고 입력은 유지한다 (R10)
    } catch {
      // reject — 입력 유지
    }
  };

  /** 수행도 선택 — 즉시 저장이고(KTD4) 같은 값 재클릭은 요청 없이 무동작이다 (R4). */
  const pickPerformance = (value: string, current: string | null) => {
    if (isBusy || value === current) return;
    onSetPerformance(value);
  };

  const openExisting = async () => {
    if (isBusy || existsInfo === null) return;
    try {
      if (
        await onOpenPage(
          existsInfo.page_id,
          existsInfo.title,
          existsInfo.date,
          existsInfo.performance,
        )
      ) {
        closeForm();
      }
    } catch {
      // reject — 안내·입력을 유지해 재시도할 수 있게 둔다
    }
  };

  return (
    <section className="card todo-card" aria-label="오늘 할 일">
      <div className="todo-header">
        <p className="settings-title">오늘 할 일</p>
        {snapshot?.state === "loaded" && (
          <span className="todo-page-title">{snapshot.title}</span>
        )}
        {notToday && (
          <>
            <span className="todo-date">{snapshotDate}</span>
            <button type="button" disabled={isBusy} onClick={onRefresh}>
              오늘로 돌아가기
            </button>
          </>
        )}
        {snapshotDate !== null && (
          <button
            type="button"
            disabled={isBusy}
            aria-expanded={formOpen}
            onClick={toggleForm}
          >
            행 만들기
          </button>
        )}
        {/* 첫 로드 실패로 스냅샷이 null이어도 새로고침으로 재시도할 수 있어야 한다 */}
        <button type="button" disabled={isBusy} onClick={onRefresh}>
          새로고침
        </button>
      </div>

      {snapshot === null ? (
        <p className="todo-status" role="status">
          불러오는 중…
        </p>
      ) : snapshot.state === "not_connected" ? (
        <p className="todo-status" role="status">
          Notion 연결이 필요해요 — 아래 Notion 연결 카드에서 토큰과 Database를
          설정해 주세요
        </p>
      ) : snapshot.state === "no_page" ? (
        <div className="todo-empty">
          <p className="todo-status" role="status">
            {snapshot.is_today ? "오늘 페이지 없음" : "이 날짜에는 페이지가 없어요"}
          </p>
          {/* 전환된 날짜에서는 '오늘 페이지 만들기'가 어긋난다 — 돌아가기만 남긴다 */}
          {snapshot.is_today && (
            <button type="button" disabled={isBusy} onClick={onCreatePage}>
              오늘 페이지 만들기
            </button>
          )}
        </div>
      ) : (
        <>
          {/* 수행도 줄 — 하루(또는 그 행이 덮는 구간) 전체의 값이라 개별 할 일보다 위에 둔다.
              360px 헤더는 이미 버튼으로 차 있어 헤더 안에는 자리가 없다 */}
          <div className="todo-perf">
            <div className="todo-perfs" role="group" aria-label="수행도">
              {PERFORMANCES.map((value) => (
                <button
                  key={value}
                  type="button"
                  className="todo-perf-pill"
                  aria-pressed={snapshot.performance === value}
                  disabled={isBusy}
                  onClick={() => pickPerformance(value, snapshot.performance)}
                >
                  {value}
                </button>
              ))}
            </div>
            {snapshot.performance === null && (
              <span className="todo-perf-empty">미지정</span>
            )}
            {/* 범위 행 — 이 값이 하루가 아니라 구간 전체에 적용된다는 사실을
                누르기 전에 보여준다 (R10). 행의 시작일은 스냅샷에 없어 끝만 적는다 */}
            {snapshot.range_end !== null && (
              <span className="todo-perf-range">{snapshot.range_end}까지 적용</span>
            )}
          </div>

          {snapshot.items.length === 0 ? (
            <p className="todo-status" role="status">
              오늘 할 일이 아직 없어요 — 아래에서 추가해 보세요
            </p>
          ) : (
            <ul className="todo-list">
              {/* 페이지 순서를 유지한 채 category가 바뀌는 지점에 섹션 라벨을 삽입한다.
                  첫 헤딩 전(category null) 항목은 라벨 없이 맨 앞 그대로 둔다 */}
              {snapshot.items.map((item, i) => {
                const prev = i > 0 ? snapshot.items[i - 1].category : null;
                const showLabel = item.category !== null && item.category !== prev;
                return (
                  <Fragment key={item.id}>
                    {showLabel && (
                      <li className="todo-section-label">{item.category}</li>
                    )}
                    <li className="todo-item">
                      {editingId === item.id ? (
                        <div className="todo-edit">
                          <input
                            type="text"
                            autoComplete="off"
                            aria-label="할 일 편집"
                            disabled={isBusy}
                            value={editRaw}
                            autoFocus
                            onChange={(e) => setEditRaw(e.target.value)}
                            // blur는 자동 커밋하지 않는다 (NotionCard의 onBlur 금지 전례)
                            onKeyDown={(e) => {
                              if (e.key === "Enter") void commitEdit();
                              if (e.key === "Escape") cancelEdit();
                            }}
                          />
                          <button
                            type="button"
                            disabled={isBusy}
                            onClick={() => void commitEdit()}
                          >
                            저장
                          </button>
                          <button type="button" disabled={isBusy} onClick={cancelEdit}>
                            취소
                          </button>
                        </div>
                      ) : (
                        <>
                          <input
                            type="checkbox"
                            aria-label={item.text}
                            checked={item.checked}
                            disabled={isBusy}
                            onChange={() => onToggle(item.id, !item.checked)}
                          />
                          <button
                            type="button"
                            className={item.checked ? "todo-text todo-done" : "todo-text"}
                            aria-label={`${item.text} 편집`}
                            disabled={isBusy}
                            onClick={() => startEdit(item)}
                          >
                            {item.text}
                          </button>
                        </>
                      )}
                    </li>
                  </Fragment>
                );
              })}
            </ul>
          )}

          <div className="todo-add">
            {/* 카테고리 세그먼트 — 선택된 헤딩 아래에 삽입된다 (기본 공부) */}
            <div className="todo-cats" role="group" aria-label="추가 카테고리">
              {ADD_CATEGORIES.map((cat) => (
                <button
                  key={cat}
                  type="button"
                  className="todo-cat"
                  aria-pressed={addCategory === cat}
                  disabled={isBusy}
                  onClick={() => setAddCategory(cat)}
                >
                  {cat}
                </button>
              ))}
            </div>
            <input
              type="text"
              autoComplete="off"
              aria-label="새 할 일"
              disabled={isBusy}
              value={addRaw}
              onChange={(e) => setAddRaw(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleAdd();
              }}
            />
            <button type="button" disabled={isBusy} onClick={() => void handleAdd()}>
              추가
            </button>
          </div>
        </>
      )}

      {formOpen && snapshotDate !== null && (
        <div
          className="row-form"
          // Escape는 폼 어디서든 닫기(입력 버림) — 편집 입력의 Escape는 위에서 stop되지 않지만
          // 폼과 목록 편집은 동시에 다른 DOM 서브트리라 겹치지 않는다
          onKeyDown={(e) => {
            if (e.key === "Escape") closeForm();
          }}
        >
          <input
            type="date"
            aria-label="날짜"
            disabled={isBusy}
            value={rowStart}
            onChange={(e) => setRowStart(e.target.value)}
          />

          {existsInfo !== null && (
            <div className="row-exists">
              <p className="row-exists-hint" role="status">
                이미 있음: {existsInfo.title}
              </p>
              <button type="button" disabled={isBusy} onClick={() => void openExisting()}>
                기존 행 열기
              </button>
            </div>
          )}

          <div className="row-actions">
            <button
              type="button"
              disabled={isBusy || !canSubmit}
              onClick={() => void submitRow()}
            >
              만들기
            </button>
            <button type="button" disabled={isBusy} onClick={closeForm}>
              취소
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
