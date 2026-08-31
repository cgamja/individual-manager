interface PetCountCardProps {
  /** 지금 떠 있는 마릿수. */
  count: number;
  /** 상한 (Rust의 `MAX_PETS`). 창 하나가 웹뷰 하나라 무한히 늘릴 수 없다. */
  max: number;
  /** 마지막으로 우클릭된 펭귄. 없으면 삭제 대상을 모른다. */
  focused: number | null;
  onAdd: () => void;
  onRemove: () => void;
}

/**
 * 펭귄을 부르고 지우는 카드.
 *
 * **누를 수 없는 버튼은 비활성으로 보여야 한다** — 눌리는데 아무 일도 없으면
 * 고장으로 읽힌다. 왜 못 누르는지도 옆에 적는다.
 */
export function PetCountCard({ count, max, focused, onAdd, onRemove }: PetCountCardProps) {
  const atMax = count >= max;
  const isLast = count <= 1;
  // 트레이로 팝오버를 열면 우클릭 대상이 없다 — 그때는 지울 펭귄을 특정할 수 없다
  const noTarget = focused === null;
  const cannotRemove = isLast || noTarget;

  const removeReason = isLast
    ? "마지막 한 마리는 지울 수 없어요. 전부 없애려면 펭귄을 꺼 주세요"
    : noTarget
      ? "지울 펭귄을 우클릭해서 열어 주세요"
      : null;

  return (
    <section className="card">
      <h2>펭귄 {count}마리</h2>
      <div className="pet-count-actions">
        <button type="button" onClick={onAdd} disabled={atMax}>
          펭귄 추가
        </button>
        <button type="button" onClick={onRemove} disabled={cannotRemove}>
          이 펭귄 삭제
        </button>
      </div>
      {atMax && <p className="hint">{max}마리가 최대예요</p>}
      {removeReason && <p className="hint">{removeReason}</p>}
    </section>
  );
}
