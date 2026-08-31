import { useState } from "react";

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

/**
 * 동작을 지금 시켜보는 카드.
 *
 * 얼음낚시는 십 분에 한 번, 슬라이딩은 삼십 초에 한 번쯤 저절로 나온다.
 * 확인할 방법이 "가만히 기다리기"뿐이면 고쳐 볼 수가 없다.
 *
 * **누를 수 없는 버튼은 비활성으로 보이고 이유를 옆에 적는다** — `PetCountCard`와
 * 같은 규칙이다. 설명은 **동작마다 다르다**: 끝나는 조건이 서로 달라서
 * (낚시는 30~60초를 앉아 있고, 슬라이딩은 2.4초에 끝난다) 한 문장으로 뭉뚱그리면
 * 둘 중 하나는 거짓말이 된다.
 */
export function MotionCard({ focused, motions }: MotionCardProps) {
  const [error, setError] = useState<string | null>(null);
  /** 방금 누른 동작 — 설명을 그것만 보여준다. 넷을 한꺼번에 늘어놓으면 안 읽힌다. */
  const [shown, setShown] = useState(0);
  const noTarget = focused === null;

  const press = async (index: number) => {
    setShown(index);
    setError(null);
    try {
      await motions[index].run();
    } catch (err) {
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
