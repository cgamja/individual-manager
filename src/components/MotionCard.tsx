import { useState } from "react";

interface MotionCardProps {
  /** 마지막으로 우클릭된 펭귄. 없으면 누구를 시킬지 알 수 없다. */
  focused: number | null;
  /** 그 펭귄에게 낚시를 시킨다. 거절되면 사유와 함께 reject된다. */
  onFish: () => Promise<void>;
}

/**
 * 동작을 지금 시켜보는 카드.
 *
 * 얼음낚시는 저절로는 **십 분에 한 번쯤** 나온다 (`ICE_FISHING_PERMILLE`).
 * 만들어 놓고 확인할 방법이 "가만히 기다리기"뿐이면 고쳐 볼 수가 없다.
 *
 * **누를 수 없는 버튼은 비활성으로 보이고 이유를 옆에 적는다** — `PetCountCard`와
 * 같은 규칙이다. 그리고 **끝나는 조건을 미리 적는다**: 낚시 중에 던지거나 때리면
 * 그 자리에서 중단되므로(빠따·드래그가 동작을 갈아치운다), 모르고 건드리면
 * 버튼이 안 먹은 것처럼 보인다.
 */
export function MotionCard({ focused, onFish }: MotionCardProps) {
  const [error, setError] = useState<string | null>(null);
  const noTarget = focused === null;

  const fish = async () => {
    setError(null);
    try {
      await onFish();
    } catch (err) {
      setError(typeof err === "string" ? err : "낚시를 시키지 못했어요");
    }
  };

  return (
    <section className="card">
      <h2>동작 시켜보기</h2>
      <div className="pet-count-actions">
        <button type="button" onClick={() => void fish()} disabled={noTarget}>
          얼음낚시
        </button>
      </div>
      {noTarget ? (
        <p className="hint">낚시할 펭귄을 우클릭해서 열어 주세요</p>
      ) : (
        <p className="hint">
          30~60초 동안 앉아 있어요. <strong>던지거나 때리면 그 자리에서 그만둬요</strong> —
          끝까지 보려면 건드리지 말고 두세요. 헤엄치는 중에 시키면 그 높이에서
          허공에 드리워요.
        </p>
      )}
      {error && (
        <p className="hint" role="status">
          {error}
        </p>
      )}
    </section>
  );
}
