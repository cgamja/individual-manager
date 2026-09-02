import { advance, newDragTrack, pushSample, type DragTrack } from "../lib/drag";
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

interface BallDrag extends DragTrack {
  /** 코어가 공을 **실제로 집었는지.** 그 전의 이동량은 `pending`에 모은다. */
  armed: boolean;
  pending: number;
  /** 진행 중인 `ball_drag_start` 왕복. 놓기 정산이 이걸 기다린다. */
  started: Promise<boolean>;
}

const root = document.getElementById("ball-root");

if (root) {
  root.innerHTML = BALL_SVG;

  // **상태는 track 안에 산다.** 모듈 변수로 빼면 새 드래그가 앞 드래그의
  // 정산을 가로채 버퍼가 뒤섞인다.
  let drag: BallDrag | null = null;

  /** 집히기 전에 모아 둔 이동량을 한 번에 보낸다. */
  const flushPending = (track: BallDrag): Promise<void> => {
    if (track.pending === 0) return Promise.resolve();
    const dx = track.pending;
    track.pending = 0;
    return dragBallBy(dx).catch(() => {});
  };

  root.addEventListener("pointerdown", (e) => {
    const pe = e as PointerEvent;
    if (pe.button !== 0 || drag) return;
    try {
      root.setPointerCapture?.(pe.pointerId);
    } catch {
      // jsdom과 일부 상황에서는 캡처가 없다. macOS는 mouse-down이 일어난 창에
      // mouse-up까지 이벤트를 암묵적으로 캡처하므로 없어도 드래그는 이어진다.
    }
    const track: BallDrag = {
      ...newDragTrack(pe.pointerId, pe.screenX, pe.screenY),
      armed: false,
      pending: 0,
      started: Promise.resolve(false),
    };
    drag = track;
    track.started = startBallDrag().catch(() => false);
    void track.started.then((grabbed) => {
      if (!grabbed) {
        // 굴러가는 중이면 손이 안 닿는다 — 한 판에 한 번 굴린다.
        if (drag === track) drag = null;
        return;
      }
      track.armed = true;
      // 이미 놓았으면 정산은 release가 한다. 여기서 또 보내면 두 번 간다.
      if (drag !== track) return;
      void flushPending(track);
    });
  });

  root.addEventListener("pointermove", (e) => {
    const pe = e as PointerEvent;
    const track = drag;
    if (!track || track.pointerId !== pe.pointerId) return;
    const moved = advance(track, pe.screenX, pe.screenY);
    if (!moved || moved.dx === 0) return;
    if (!track.armed) {
      track.pending += moved.dx;
      return;
    }
    void dragBallBy(moved.dx).catch(() => {});
  });

  const release = (e: Event) => {
    const pe = e as PointerEvent;
    const track = drag;
    if (!track || track.pointerId !== pe.pointerId) return;
    pushSample(track, pe.screenX, pe.screenY);
    drag = null;
    try {
      root.releasePointerCapture?.(pe.pointerId);
    } catch {
      // 캡처가 없었으면 놓을 것도 없다.
    }

    void (async () => {
      // **집기 결과를 끝까지 들고 간다.** 빠르게 튕기면 `pointerup`이
      // `ball_drag_start`의 왕복보다 먼저 온다 — 그때 결과를 안 보고 놓기를
      // 보내면, 집기가 거절됐는데도 굴러가던 공의 속도를 덮어쓴다.
      const grabbed = await track.started.catch(() => false);
      if (!grabbed) return;
      await flushPending(track);
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
