import { useState } from "react";
import type { TodoItem, TodoSnapshot } from "../lib/notion";

/** 행 만들기 폼이 제출하는 파라미터 — lib/notion createTodoRow와 동일 형태. */
export interface CreateRowFormParams {
  title: string;
  start: string;
  end?: string;
  icon?: string;
  performance?: string;
}

/** 행 만들기 제출 결과 — 카드가 폼 접힘/유지·기존 행 안내를 결정하는 데 쓴다. */
export type CreateRowFormResult =
  | { state: "created" }
  | { state: "exists"; page_id: string; title: string; date: string }
  | { state: "failed" };

const TODO_TITLE = "[TODO]";
const TITLE_CHIPS = [TODO_TITLE, "휴일", "MT"];
const PERFORMANCE_OPTIONS = ["완료", "일부", "미완", "기타"];
const DEFAULT_PERFORMANCE = "기타";

// tsconfig lib(ES2020)에는 Intl.Segmenter 타입이 없다 — 런타임(WKWebView·
// jsdom 모두 지원)에는 존재하므로 필요한 형태만 로컬로 좁혀 쓴다
const GraphemeSegmenter = (
  Intl as unknown as {
    Segmenter: new (
      locale: string,
      options: { granularity: "grapheme" },
    ) => { segment(input: string): Iterable<unknown> };
  }
).Segmenter;

const graphemeSegmenter = new GraphemeSegmenter("ko", { granularity: "grapheme" });

/** 그래프임(사용자 지각 문자) 개수 — 결합 이모지(👍🏽)도 1로 센다. */
function graphemeCount(value: string): number {
  let count = 0;
  for (const _ of graphemeSegmenter.segment(value)) {
    count += 1;
  }
  return count;
}

interface TodoCardProps {
  /** null = 첫 로드 전 ("불러오는 중…"). 스냅샷이 있으면 busy 중에도 목록을 유지한다. */
  snapshot: TodoSnapshot | null;
  /** todo 커맨드 진행 중 — 추가·토글·편집·새로고침·만들기를 모두 잠근다. */
  isBusy: boolean;
  onRefresh: () => void;
  onCreatePage: () => void;
  /** resolve가 true(반영 성공)일 때만 입력을 비운다 — 실패·안내(notice) 시 유지. */
  onAdd: (text: string) => Promise<boolean>;
  /** checked는 토글 후 원하는 값이다. */
  onToggle: (blockId: string, checked: boolean) => void;
  /** resolve가 true일 때만 편집 모드를 닫는다 — 실패 시 입력을 유지해 재시도. */
  onEdit: (blockId: string, text: string) => Promise<boolean>;
  /** created일 때만 폼을 접는다 — exists는 기존 행 안내, failed는 입력 유지. */
  onCreateRow: (params: CreateRowFormParams) => Promise<CreateRowFormResult>;
  /** resolve가 true(열기 성공)일 때만 폼을 접는다. */
  onOpenPage: (pageId: string, title: string, date: string) => Promise<boolean>;
}

/** 오늘 할 일 카드 — 오늘 페이지의 to_do 블록 조회·추가·토글·편집 + 행 만들기·날짜 전환. */
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
}: TodoCardProps) {
  const [addRaw, setAddRaw] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editRaw, setEditRaw] = useState("");

  // 행 만들기 폼 — 입력 초안만 로컬 state로 둔다
  const [formOpen, setFormOpen] = useState(false);
  const [rowTitle, setRowTitle] = useState("");
  const [rowIcon, setRowIcon] = useState("");
  const [rowStart, setRowStart] = useState("");
  const [rowRange, setRowRange] = useState(false);
  const [rowEnd, setRowEnd] = useState("");
  const [rowPerf, setRowPerf] = useState(DEFAULT_PERFORMANCE);
  const [existsInfo, setExistsInfo] = useState<{
    page_id: string;
    title: string;
    date: string;
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
      if (await onAdd(text)) setAddRaw("");
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

  /** 폼을 접으며 입력을 전부 버린다 — 재열림은 항상 초기 상태다. */
  const closeForm = () => {
    setFormOpen(false);
    setRowTitle("");
    setRowIcon("");
    setRowStart("");
    setRowRange(false);
    setRowEnd("");
    setRowPerf(DEFAULT_PERFORMANCE);
    setExistsInfo(null);
  };

  const toggleForm = () => {
    if (formOpen) {
      closeForm();
      return;
    }
    // 기본 날짜는 스냅샷의 date — 프론트는 오늘을 계산하지 않는다
    setRowStart(snapshotDate ?? "");
    setRowEnd(snapshotDate ?? "");
    setFormOpen(true);
  };

  // 분류·전송 모두 트림된 제목 기준 — 추가·편집 핸들러의 trim 전례와 일치
  const trimmedTitle = rowTitle.trim();
  // 분류 기준은 "현재 제목 입력값"(트림 후) — 클릭한 칩이 아니다
  const isTodoTitle = trimmedTitle === TODO_TITLE;
  const iconInvalid =
    !isTodoTitle && rowIcon !== "" && graphemeCount(rowIcon) !== 1;
  const dateInvalid =
    !isTodoTitle && rowRange && rowEnd !== "" && rowEnd < rowStart;
  const canSubmit =
    trimmedTitle !== "" && rowStart !== "" && !iconInvalid && !dateInvalid;

  const submitRow = async () => {
    if (isBusy || !canSubmit) return;
    const params: CreateRowFormParams = { title: trimmedTitle, start: rowStart };
    if (!isTodoTitle) {
      // [TODO]에는 end·icon·performance를 절대 실지 않는다 (아이콘은 백엔드가 복사)
      if (rowRange && rowEnd !== "") params.end = rowEnd;
      if (rowIcon !== "") params.icon = rowIcon;
      params.performance = rowPerf;
    }
    setExistsInfo(null);
    try {
      const result = await onCreateRow(params);
      if (result.state === "created") {
        closeForm();
      } else if (result.state === "exists") {
        setExistsInfo({
          page_id: result.page_id,
          title: result.title,
          date: result.date,
        });
      }
      // failed — 배너는 App이 띄우고 입력은 유지한다 (R10)
    } catch {
      // reject — 입력 유지
    }
  };

  const openExisting = async () => {
    if (isBusy || existsInfo === null) return;
    try {
      if (await onOpenPage(existsInfo.page_id, existsInfo.title, existsInfo.date)) {
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
          {snapshot.items.length === 0 ? (
            <p className="todo-status" role="status">
              오늘 할 일이 아직 없어요 — 아래에서 추가해 보세요
            </p>
          ) : (
            <ul className="todo-list">
              {snapshot.items.map((item) => (
                <li key={item.id} className="todo-item">
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
                      <button type="button" disabled={isBusy} onClick={() => void commitEdit()}>
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
              ))}
            </ul>
          )}

          <div className="todo-add">
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
          <div className="row-chips">
            {TITLE_CHIPS.map((chip) => (
              <button
                key={chip}
                type="button"
                className="row-chip"
                disabled={isBusy}
                onClick={() => setRowTitle(chip)}
              >
                {chip}
              </button>
            ))}
          </div>

          <input
            type="text"
            autoComplete="off"
            aria-label="행 제목"
            placeholder="제목"
            disabled={isBusy}
            value={rowTitle}
            onChange={(e) => setRowTitle(e.target.value)}
          />

          {!isTodoTitle && (
            <input
              type="text"
              autoComplete="off"
              aria-label="아이콘"
              placeholder="아이콘 (이모지 1자, 비워도 됨)"
              disabled={isBusy}
              value={rowIcon}
              onChange={(e) => setRowIcon(e.target.value)}
            />
          )}

          <div className="row-dates">
            <input
              type="date"
              aria-label="날짜"
              disabled={isBusy}
              value={rowStart}
              onChange={(e) => setRowStart(e.target.value)}
            />
            {!isTodoTitle && (
              <label className="row-range">
                <input
                  type="checkbox"
                  aria-label="범위"
                  disabled={isBusy}
                  checked={rowRange}
                  onChange={(e) => setRowRange(e.target.checked)}
                />
                범위
              </label>
            )}
            {!isTodoTitle && rowRange && (
              <input
                type="date"
                aria-label="끝 날짜"
                disabled={isBusy}
                value={rowEnd}
                onChange={(e) => setRowEnd(e.target.value)}
              />
            )}
          </div>

          {!isTodoTitle && (
            <select
              aria-label="수행도"
              disabled={isBusy}
              value={rowPerf}
              onChange={(e) => setRowPerf(e.target.value)}
            >
              {PERFORMANCE_OPTIONS.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          )}

          {iconInvalid && (
            <p className="row-hint" role="status">
              아이콘은 이모지 1자만 넣을 수 있어요
            </p>
          )}
          {dateInvalid && (
            <p className="row-hint" role="status">
              끝 날짜는 시작 이후여야 해요
            </p>
          )}

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
              행 추가
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
