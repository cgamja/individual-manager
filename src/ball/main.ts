import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { advance, newDragTrack, pushSample, type DragTrack } from "../lib/drag";
import { loadPetSettings } from "../lib/settings";
import { SoundPlayer } from "../pet/sound";
import {
  DRAG_THRESHOLD_PX,
  dragBallBy,
  endBallDrag,
  onBallState,
  onPetSound,
  startBallDrag,
  throwVelocity,
  type BallSnapshot,
} from "../lib/pet";
import { BOWLING_BALL_SVG } from "../assets/props/bowling-ball";
import "./ball.css";

/**
 * 볼링 공 창의 웹뷰.
 *
 * 드래그 규약은 **펭귄 창과 같다** (KTD5): 웹뷰는 이동량(델타)과 놓는 순간의
 * 속도만 보내고, 창 위치의 소유자는 Rust 하나다. 그래서 화면 좌표 → 세계 좌표
 * 변환이 여기에도 저기에도 없다.
 */

interface BallDrag extends DragTrack {
  /** 코어가 공을 **실제로 집었는지.** 그 전의 이동량은 `pending`에 모은다. */
  armed: boolean;
  pending: number;
  /** 진행 중인 `ball_drag_start` 왕복. 놓기 정산이 이걸 기다린다. */
  started: Promise<boolean>;
}

const root = document.getElementById("ball-root");

if (root) {
  root.innerHTML = BOWLING_BALL_SVG;

  // **상태는 track 안에 산다.** 모듈 변수로 빼면 새 드래그가 앞 드래그의
  // 정산을 가로채 버퍼가 뒤섞인다.
  let drag: BallDrag | null = null;

  // 이 창의 소리 — 굴러가기 시작할 때 딱 한 발. 켜짐/꺼짐·음량은 펭귄 창과
  // **같은 설정 하나**를 따른다 (`pet://sound` 방송).
  const player = new SoundPlayer(getCurrentWebviewWindow().label);
  void loadPetSettings()
    .then((s) => {
      player.setEnabled(s.sound);
      player.setVolume(s.volume);
    })
    .catch(() => {});
  void onPetSound(({ sound, volume }) => {
    player.setEnabled(sound);
    player.setVolume(volume);
  }).catch(() => {});

  /** 집히기 전에 모아 둔 이동량을 한 번에 보낸다. */
  const flushPending = (track: BallDrag): Promise<void> => {
    if (track.pending === 0) return Promise.resolve();
    const dx = track.pending;
    track.pending = 0;
    return dragBallBy(dx).catch(() => {});
  };

  root.addEventListener("pointerdown", (e) => {
    const pe = e as PointerEvent;
    // 사용자 제스처에서만 suspended 컨텍스트를 깨울 수 있다 (WKWebView).
    player.nudge();
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

  let 굴렀나 = false;
  const paint = (ball: BallSnapshot) => {
    root.classList.toggle("bw-ball--rolling", ball.rolling);
    root.classList.toggle("bw-ball--held", ball.held);
    // 구르기 **시작하는 순간**에만 낸다. 상태를 그대로 소리에 연결하면
    // 굴러가는 내내 매 알림마다 다시 울린다.
    if (ball.rolling && !굴렀나) player.play("roll", performance.now());
    굴렀나 = ball.rolling;
  };

  // **받는 쪽을 창에 묶는다.** 전역 `listen()`은 대상을 `Any`로 등록해서
  // emit 대상과 무관하게 전부 호출된다 — 창이 여럿이면 그때 터진다
  // (`docs/solutions/best-practices/tauri-any-listener-receives-every-event.md`).
  void onBallState(paint).catch(() => {});
}
