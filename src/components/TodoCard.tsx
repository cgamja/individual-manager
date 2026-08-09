import { useState } from "react";
import type { TodoItem, TodoSnapshot } from "../lib/notion";

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
}

/** 오늘 할 일 카드 — 오늘 페이지의 to_do 블록 조회·추가·토글·편집. */
export function TodoCard({
  snapshot,
  isBusy,
  onRefresh,
  onCreatePage,
  onAdd,
  onToggle,
  onEdit,
}: TodoCardProps) {
  const [addRaw, setAddRaw] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editRaw, setEditRaw] = useState("");

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

  return (
    <section className="card todo-card" aria-label="오늘 할 일">
      <div className="todo-header">
        <p className="settings-title">오늘 할 일</p>
        {snapshot?.state === "loaded" && (
          <span className="todo-page-title">{snapshot.title}</span>
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
            오늘 페이지 없음
          </p>
          <button type="button" disabled={isBusy} onClick={onCreatePage}>
            오늘 페이지 만들기
          </button>
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
    </section>
  );
}
