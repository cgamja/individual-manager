import { useRef, useState } from "react";

/** 시켜볼 수 있는 동작 하나. */
export interface Motion {
  /** 버튼에 쓰는 이름. */
  name: string;
  /** 누르기 전에 알아야 할 것 — 얼마나 걸리고 무엇이 끊는지. */
  note: string;
  /** 대상 펭귄에게 시킨다. 거절되면 사유와 함께 reject된다. */
  run: () => Promise<void>;
}

interface MotionCardProps {
  /** 마지막으로 우클릭된 펭귄. 없으면 누구를 시킬지 알 수 없다. */
  focused: number | null;
  motions: readonly Motion[];
}

/** 동작을 지금 시켜보는 카드. */
export function MotionCard({ focused, motions }: MotionCardProps) {
  const [error, setError] = useState<string | null>(null);
  /** 방금 누른 동작 — 설명을 그것만 보여준다. 넷을 한꺼번에 늘어놓으면 안 읽힌다. */
  const [shown, setShown] = useState(0);
  /** 마지막으로 누른 번째. **늦게 도착한 결과는 버린다** — 빠르게 두 번 누르면 */
  const pressSeq = useRef(0);
  const noTarget = focused === null;

  const press = async (index: number) => {
    const seq = ++pressSeq.current;
    setShown(index);
    setError(null);
    try {
      await motions[index].run();
    } catch (err) {
      if (pressSeq.current !== seq) return;
      setError(typeof err === "string" ? err : "시키지 못했어요");
    }
  };

  return (
    <section className="card">
      <h2>동작 시켜보기</h2>
      <div className="pet-count-actions">
        {motions.map((motion, i) => (
          <button
            key={motion.name}
            type="button"
            onClick={() => void press(i)}
            disabled={noTarget}
          >
            {motion.name}
          </button>
        ))}
      </div>
      {noTarget ? (
        <p className="hint">시킬 펭귄을 우클릭해서 열어 주세요</p>
      ) : (
        <p className="hint">{motions[shown]?.note}</p>
      )}
      {error && (
        <p className="hint" role="status">
          {error}
        </p>
      )}
    </section>
  );
}
