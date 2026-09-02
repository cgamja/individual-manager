import {
  DRAG_THRESHOLD_PX,
  dragBallBy,
  endBallDrag,
  onBallState,
  startBallDrag,
  throwVelocity,
  type BallSnapshot,
} from "../lib/pet";
import "./ball.css";

/**
 * 볼링 공 창의 웹뷰.
 *
 * 드래그 규약은 **펭귄 창과 같다** (KTD5): 웹뷰는 이동량(델타)과 놓는 순간의
 * 속도만 보내고, 창 위치의 소유자는 Rust 하나다. 그래서 화면 좌표 → 세계 좌표
 * 변환이 여기에도 저기에도 없다.
 */

/** 공 그림 — 원 하나에 손가락 구멍 셋. 펭귄만큼 공들이지 않는다 (A6). */
const BALL_SVG = `
<svg class="bw-ball" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <g class="bw-ball-body">
    <circle cx="32" cy="32" r="30" fill="#2b2f4a" stroke="#12142a" stroke-width="2" />
    <path d="M12 18 A30 30 0 0 1 34 5" stroke="#6f76a8" stroke-width="5"
          stroke-linecap="round" fill="none" opacity="0.55" />
    <circle cx="25" cy="24" r="4.4" fill="#0d0f1e" />
    <circle cx="38" cy="21" r="4.4" fill="#0d0f1e" />
    <circle cx="32" cy="34" r="4.4" fill="#0d0f1e" />
  </g>
</svg>`;

/** 속도 계산에 남겨 둘 궤적의 길이 — 펫 창과 같은 값이다. */
const SAMPLE_KEEP_MS = 400;

interface DragTrack {
  /** 이 드래그를 소유한 포인터. 다른 포인터의 이벤트는 무시한다. */
  pointerId: number;
  screenX: number;
  screenY: number;
  moved: number;
  /** 최근 궤적 — 놓는 순간의 속도를 재는 데 쓴다. */
  samples: { x: number; y: number; t: number }[];
}

function pushSample(track: DragTrack, x: number, y: number): void {
  const t = performance.now();
  track.samples.push({ x, y, t });
  while (track.samples.length > 2 && t - track.samples[0].t > SAMPLE_KEEP_MS) {
    track.samples.shift();
  }
}

const root = document.getElementById("ball-root");

if (root) {
  root.innerHTML = BALL_SVG;

  let drag: DragTrack | null = null;
  /** 코어가 공을 집었는지 — 그 전의 이동량은 pending에 모은다. */
  let armed = false;
  let pending = 0;
  let startPromise: Promise<boolean> | null = null;

  root.addEventListener("pointerdown", (e) => {
    const pe = e as PointerEvent;
    if (pe.button !== 0 || drag) return;
    try {
      root.setPointerCapture?.(pe.pointerId);
    } catch {
      // jsdom과 일부 상황에서는 캡처가 없다. macOS는 mouse-down이 일어난 창에
      // mouse-up까지 이벤트를 암묵적으로 캡처하므로 없어도 드래그는 이어진다.
    }
    const track: DragTrack = {
      pointerId: pe.pointerId,
      screenX: pe.screenX,
      screenY: pe.screenY,
      moved: 0,
      samples: [{ x: pe.screenX, y: pe.screenY, t: performance.now() }],
    };
    drag = track;
    armed = false;
    pending = 0;
    const started = startBallDrag();
    startPromise = started;
    void started
      .then((grabbed) => {
        if (drag !== track) return;
        if (!grabbed) {
          // 굴러가는 중이면 손이 안 닿는다 — 한 판에 한 번 굴린다.
          drag = null;
          return;
        }
        armed = true;
        if (pending !== 0) {
          const dx = pending;
          pending = 0;
          void dragBallBy(dx).catch(() => {});
        }
      })
      .catch(() => {});
  });

  root.addEventListener("pointermove", (e) => {
    const pe = e as PointerEvent;
    const track = drag;
    if (!track || track.pointerId !== pe.pointerId) return;
    const dx = pe.screenX - track.screenX;
    const dy = pe.screenY - track.screenY;
    if (dx === 0 && dy === 0) return;
    track.screenX = pe.screenX;
    track.screenY = pe.screenY;
    track.moved += Math.abs(dx) + Math.abs(dy);
    pushSample(track, pe.screenX, pe.screenY);
    if (dx === 0) return;
    if (!armed) {
      pending += dx;
      return;
    }
    void dragBallBy(dx).catch(() => {});
  });

  const release = (e: Event) => {
    const pe = e as PointerEvent;
    const track = drag;
    if (!track || track.pointerId !== pe.pointerId) return;
    pushSample(track, pe.screenX, pe.screenY);
    drag = null;
    armed = false;
    try {
      root.releasePointerCapture?.(pe.pointerId);
    } catch {
      // 캡처가 없었으면 놓을 것도 없다.
    }

    void (async () => {
      await startPromise?.catch(() => {});
      if (pending !== 0) {
        const dx = pending;
        pending = 0;
        await dragBallBy(dx).catch(() => {});
      }
      // **가로 속도만 넘긴다** — 조준 각도가 없다 (R6). 세로로만 그었으면
      // vx가 0에 가까워 공은 제자리에 남고 다시 집을 수 있다.
      const vx = track.moved < DRAG_THRESHOLD_PX ? 0 : throwVelocity(track.samples).vx;
      await endBallDrag(vx).catch(() => {});
    })();
  };

  root.addEventListener("pointerup", release);
  root.addEventListener("pointercancel", release);
  root.addEventListener("contextmenu", (e) => e.preventDefault());

  const paint = (ball: BallSnapshot) => {
    root.classList.toggle("bw-ball--rolling", ball.rolling);
    root.classList.toggle("bw-ball--held", ball.held);
  };

  // **받는 쪽을 창에 묶는다.** 전역 `listen()`은 대상을 `Any`로 등록해서
  // emit 대상과 무관하게 전부 호출된다 — 창이 여럿이면 그때 터진다
  // (`docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`).
  void onBallState(paint).catch(() => {});
}
