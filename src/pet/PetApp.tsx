import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Penguin } from "./Penguin";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
  isOneShot,
  throwVelocity,
  verticalClass,
  dragPetBy,
  endPetDrag,
  getPetState,
  onPetState,
  pokePet,
  startPetDrag,
  type PetSnapshot,
} from "../lib/pet";

/** 드래그 진행 정보 — 화면 좌표 기준의 직전 지점과 누적 이동량. */
interface DragTrack {
  /** 이 드래그를 소유한 포인터. 다른 포인터의 이벤트는 무시한다. */
  pointerId: number;
  screenX: number;
  screenY: number;
  moved: number;
  /** 최근 궤적 — 놓는 순간의 속도를 재는 데 쓴다 (던지기 세기). */
  samples: { x: number; y: number; t: number }[];
}



/** 속도 계산에 남겨 둘 궤적의 길이. 개수로 자르면 포인터 보고 주기(60Hz vs 120Hz)에
 * 따라 실제로 보는 구간이 달라져 같은 손짓의 세기가 기기마다 달라진다. */
const SAMPLE_KEEP_MS = 400;

function pushSample(track: DragTrack, x: number, y: number): void {
  const t = performance.now();
  track.samples.push({ x, y, t });
  while (track.samples.length > 2 && t - track.samples[0].t > SAMPLE_KEEP_MS) {
    track.samples.shift();
  }
}

/** 눈동자가 흰자를 벗어나지 않을 만큼만 움직인다 (SVG 좌표 단위). */
const GAZE_LIMIT = 1.6;
const clampGaze = (v: number) => Math.max(-GAZE_LIMIT, Math.min(GAZE_LIMIT, v));

/**
 * 바탕화면 펭귄의 웹뷰. 창 위치는 Rust가 소유하므로 여기서는
 * (1) 현재 동작을 CSS 클래스로 바꿔 입히고, (2) 포인터 의도를 커맨드로 넘긴다.
 */
export function PetApp() {
  const [snapshot, setSnapshot] = useState<PetSnapshot | null>(null);
  const dragRef = useRef<DragTrack | null>(null);
  /** 코어가 Dragged로 넘어갔는지 — 그 전의 이동량은 pendingRef에 모은다. */
  const armedRef = useRef(false);
  const pendingRef = useRef({ dx: 0, dy: 0 });
  /** 진행 중인 pet_drag_start 왕복 — 놓기 정산이 이걸 기다린다. */
  const startPromiseRef = useRef<Promise<void> | null>(null);
  /** 한 번짜리 애니메이션을 되감기 위한 remount 카운터. */
  const [restartKey, setRestartKey] = useState(0);
  /** 눈동자가 커서를 향해 밀리는 양 (SVG 좌표, R7). */
  const [gaze, setGaze] = useState({ x: 0, y: 0 });

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    (async () => {
      // 첫 틱을 기다리지 않고 현재 상태부터 그린다
      const initial = await getPetState().catch(() => null);
      if (!cancelled && initial) setSnapshot(initial);
      unlisten = await onPetState((next) => {
        setSnapshot(next);
        if (isOneShot(behaviorClass(next.behavior))) {
          setRestartKey((k) => k + 1);
        }
      });
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    // 주 버튼만 드래그를 시작한다 — 우클릭까지 받으면 놓을 때 클릭으로 해석돼
    // 팝오버가 열린다. 이미 드래그 중이면 두 번째 포인터는 무시한다: 기준점을
    // 덮어쓰면 원래 포인터의 다음 이동량이 엉뚱한 값이 돼 펭귄이 순간이동한다.
    if (e.button !== 0 || dragRef.current) return;
    // 포인터를 캡처해야 커서가 펭귄 밖으로 나가도 드래그가 끊기지 않는다
    e.currentTarget.setPointerCapture(e.pointerId);
    const track: DragTrack = {
      pointerId: e.pointerId,
      screenX: e.screenX,
      screenY: e.screenY,
      moved: 0,
      samples: [{ x: e.screenX, y: e.screenY, t: performance.now() }],
    };
    dragRef.current = track;
    armedRef.current = false;
    pendingRef.current = { dx: 0, dy: 0 };
    const started = startPetDrag();
    startPromiseRef.current = started;
    void started.then(() => {
      // 왕복이 끝나기 전에 이미 손을 뗐다면 코어는 벌써 Falling이다 —
      // 여기서 밀린 이동량을 보내면 무시되고, 펭귄은 제자리에서 떨어지기만 한다
      if (dragRef.current !== track) return;
      armedRef.current = true;
      // 코어가 Dragged가 되기 전에 움직인 만큼을 몰아서 보낸다. 이걸 버리면
      // 누르자마자 빠르게 끈 첫 이동이 통째로 사라져 "즉시 안 따라온다"가 된다
      const pending = pendingRef.current;
      if (pending.dx !== 0 || pending.dy !== 0) {
        pendingRef.current = { dx: 0, dy: 0 };
        void dragPetBy(pending.dx, pending.dy);
      }
    });
  }, []);

  const handlePointerMove = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    const track = dragRef.current;
    if (!track) {
      // 잡고 있지 않을 때는 커서를 눈으로 좇는다 (R7). 창이 펭귄 크기라
      // 여기까지 이벤트가 온다는 건 커서가 이미 펭귄 위에 있다는 뜻이다.
      const rect = e.currentTarget.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        const nx = (e.clientX - rect.left) / rect.width - 0.5;
        const ny = (e.clientY - rect.top) / rect.height - 0.5;
        setGaze({ x: clampGaze(nx * 4), y: clampGaze(ny * 4) });
      }
      return;
    }
    if (track.pointerId !== e.pointerId) return;
    // 화면 좌표로 이동량을 잰다 — 창이 따라 움직이므로 창 기준 좌표를 쓰면
    // 이동량이 스스로를 상쇄해 펭귄이 따라오지 않는다
    const dx = e.screenX - track.screenX;
    const dy = e.screenY - track.screenY;
    if (dx === 0 && dy === 0) return;
    track.screenX = e.screenX;
    track.screenY = e.screenY;
    track.moved += Math.abs(dx) + Math.abs(dy);
    pushSample(track, e.screenX, e.screenY);
    if (!armedRef.current) {
      // 아직 드래그 시작이 왕복 중 — 이동량은 누적해 두고 준비되면 흘려보낸다
      pendingRef.current.dx += dx;
      pendingRef.current.dy += dy;
      return;
    }
    void dragPetBy(dx, dy);
  }, []);

  const handlePointerUp = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    const track = dragRef.current;
    if (!track || track.pointerId !== e.pointerId) return;
    // 놓는 시점을 샘플로 남긴다. 이게 없으면 속도 창이 "마지막으로 움직인 시각"에
    // 걸려, 세게 뿌린 뒤 손을 멈추고 떼도 옛 속도로 던져진다 (움직임이 없는
    // pointermove는 아래에서 걸러지므로 정지 구간에는 샘플이 생기지 않는다)
    pushSample(track, e.screenX, e.screenY);
    dragRef.current = null;
    armedRef.current = false;
    // 놓기 전에 캡처를 풀되, 실패해도 아래 정산은 반드시 진행한다
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // 이미 풀렸거나 캡처된 적 없음 — 무시해도 안전하다
    }

    void (async () => {
      // 시작 왕복이 아직이면 기다린다. 기다리지 않고 끝내면 커맨드가
      // start → end → by 순으로 도착해 밀린 이동량이 버려지고, 빠르게 튕겨
      // 놓은 드래그가 통째로 사라진다
      await startPromiseRef.current?.catch(() => {});
      const pending = pendingRef.current;
      if (pending.dx !== 0 || pending.dy !== 0) {
        pendingRef.current = { dx: 0, dy: 0 };
        await dragPetBy(pending.dx, pending.dy).catch(() => {});
      }
      // 거의 안 움직였으면 옮길 의도가 아니라 클릭이다 (R5)
      if (track.moved < DRAG_THRESHOLD_PX) {
        await pokePet().catch(() => {});
      } else {
        // 놓는 순간의 속도가 던지는 세기다 (R12)
        const { vx, vy } = throwVelocity(track.samples);
        await endPetDrag(vx, vy).catch(() => {});
      }
    })();
  }, []);

  const handlePointerLeave = useCallback(() => {
    // 커서가 떠나면 정면으로 돌아온다
    setGaze({ x: 0, y: 0 });
  }, []);

  // 방향(좌우 반전)은 바깥 무대가, 동작은 SVG가 맡는다. 둘 다 transform을 쓰는데
  // 한 요소에 겹치면 반전이 동작 애니메이션을 뒤집어 버린다.
  const stageClass = `pg-stage${snapshot?.facing === "left" ? " pg-stage--flip" : ""}`;
  const petClass = [
    "penguin",
    snapshot ? behaviorClass(snapshot.behavior) : "pg--walk",
    verticalClass(snapshot?.vertical ?? "level"),
  ].join(" ");

  return (
    <div className={stageClass}>
      <Penguin
        key={restartKey}
        className={petClass}
        style={
          { "--gaze-x": `${gaze.x}px`, "--gaze-y": `${gaze.y}px` } as React.CSSProperties
        }
        onPointerLeave={handlePointerLeave}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      />
    </div>
  );
}
