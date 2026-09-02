import { useEffect, useState } from "react";
import { TAUNT_MAX_LEN } from "../lib/pet";

interface TauntCardProps {
  lines: readonly string[];
  /** 저장은 부모가 한다 — 실패 표시도 부모가 모아서 보여준다. */
  onChange: (lines: string[]) => void;
}

/** 펭귄이 할 말 목록 편집. */
export function TauntCard({ lines, onChange }: TauntCardProps) {
  const [draft, setDraft] = useState("");
  /** 편집 중인 줄의 인덱스. 하나씩만 연다. */
  const [editing, setEditing] = useState<number | null>(null);
  const [editDraft, setEditDraft] = useState("");

  useEffect(() => {
    setEditing(null);
  }, [lines]);

  const add = () => {
    const line = draft.trim();
    if (line.length === 0) return;
    onChange([...lines, line]);
    setDraft("");
  };

  const commitEdit = (index: number) => {
    const line = editDraft.trim();
    setEditing(null);
    const next = lines.filter((_, i) => i !== index);
    if (line.length > 0) next.splice(index, 0, line);
    onChange(next);
  };

  return (
    <section className="card taunt-card" aria-label="펭귄 대사">
      <p className="settings-title">펭귄이 할 말 ({lines.length})</p>

      {lines.length === 0 && (
        <p className="settings-hint">대사가 없어요 — 펭귄이 조용해집니다</p>
      )}

      <ul className="taunt-list">
        {lines.map((line, index) => (
          <li key={`${index}-${line}`} className="taunt-item">
            {editing === index ? (
              <input
                className="taunt-edit"
                aria-label={`대사 ${index + 1} 수정`}
                value={editDraft}
                maxLength={TAUNT_MAX_LEN}
                autoFocus
                onChange={(e) => setEditDraft(e.target.value)}
                onBlur={() => commitEdit(index)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") commitEdit(index);
                  if (e.key === "Escape") setEditing(null);
                }}
              />
            ) : (
              <button
                type="button"
                className="taunt-text"
                title="눌러서 수정"
                onClick={() => {
                  setEditing(index);
                  setEditDraft(line);
                }}
              >
                {line}
              </button>
            )}
            <button
              type="button"
              className="taunt-remove"
              aria-label={`대사 ${index + 1} 삭제`}
              onClick={() => onChange(lines.filter((_, i) => i !== index))}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      <div className="taunt-add">
        <input
          aria-label="새 대사"
          placeholder="새 대사를 적어요"
          value={draft}
          maxLength={TAUNT_MAX_LEN}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") add();
          }}
        />
        <button type="button" onClick={add} disabled={draft.trim().length === 0}>
          추가
        </button>
      </div>
    </section>
  );
}
