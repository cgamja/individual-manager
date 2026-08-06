import { formatMmss, nextPhase, type Phase, type TimerSnapshot } from "../lib/timer";

const PHASE_LABEL: Record<Phase, string> = {
  focus: "집중",
  break: "휴식",
};

interface TimerCardProps {
  snapshot: TimerSnapshot;
  onStart: (phase: Phase) => void;
  onPause: () => void;
  onResume: () => void;
  onReset: () => void;
}

/** 뽀모도로 타이머 카드 — 상태별 표시와 조작 버튼. */
export function TimerCard({ snapshot, onStart, onPause, onResume, onReset }: TimerCardProps) {
  const remaining =
    snapshot.state === "running" || snapshot.state === "paused"
      ? snapshot.remaining_ms
      : 0;

  return (
    <section className="card timer-card" aria-label="뽀모도로 타이머">
      {snapshot.state === "idle" && (
        <>
          <p className="timer-phase">뽀모도로</p>
          <p className="timer-time">--:--</p>
          <div className="timer-actions">
            <button type="button" onClick={() => onStart("focus")}>
              집중 시작
            </button>
          </div>
        </>
      )}

      {(snapshot.state === "running" || snapshot.state === "paused") && (
        <>
          <p className="timer-phase">
            {PHASE_LABEL[snapshot.phase]}
            {snapshot.state === "paused" && " · 일시정지"}
          </p>
          <p className="timer-time">{formatMmss(remaining)}</p>
          <div className="timer-actions">
            {snapshot.state === "running" ? (
              <button type="button" onClick={onPause}>
                일시정지
              </button>
            ) : (
              <button type="button" onClick={onResume}>
                재개
              </button>
            )}
            <button type="button" onClick={onReset}>
              리셋
            </button>
          </div>
        </>
      )}

      {snapshot.state === "finished" && (
        <>
          <p className="timer-phase">{PHASE_LABEL[snapshot.phase]} 세션 종료</p>
          <p className="timer-time">00:00</p>
          <div className="timer-actions">
            <button type="button" onClick={() => onStart(nextPhase(snapshot.phase))}>
              {PHASE_LABEL[nextPhase(snapshot.phase)]} 시작
            </button>
            <button type="button" onClick={onReset}>
              리셋
            </button>
          </div>
        </>
      )}
    </section>
  );
}
