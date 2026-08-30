import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Penguin } from "./Penguin";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
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
  screenX: number;
  screenY: number;
  moved: number;
}

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

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    (async () => {
      // 첫 틱을 기다리지 않고 현재 상태부터 그린다
      const initial = await getPetState().catch(() => null);
      if (!cancelled && initial) setSnapshot(initial);
      unlisten = await onPetState((next) => setSnapshot(next));
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    // 포인터를 캡처해야 커서가 펭귄 밖으로 나가도 드래그가 끊기지 않는다
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = { screenX: e.screenX, screenY: e.screenY, moved: 0 };
    armedRef.current = false;
    pendingRef.current = { dx: 0, dy: 0 };
    void startPetDrag().then(() => {
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
    if (!track) return;
    // 화면 좌표로 이동량을 잰다 — 창이 따라 움직이므로 창 기준 좌표를 쓰면
    // 이동량이 스스로를 상쇄해 펭귄이 따라오지 않는다
    const dx = e.screenX - track.screenX;
    const dy = e.screenY - track.screenY;
    if (dx === 0 && dy === 0) return;
    track.screenX = e.screenX;
    track.screenY = e.screenY;
    track.moved += Math.abs(dx) + Math.abs(dy);
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
    if (!track) return;
    dragRef.current = null;
    armedRef.current = false;
    e.currentTarget.releasePointerCapture(e.pointerId);
    // 거의 안 움직였으면 옮길 의도가 아니라 클릭이다 (R5)
    if (track.moved < DRAG_THRESHOLD_PX) {
      void pokePet();
    } else {
      void endPetDrag();
    }
  }, []);

  // 방향(좌우 반전)은 바깥 무대가, 동작은 SVG가 맡는다. 둘 다 transform을 쓰는데
  // 한 요소에 겹치면 반전이 동작 애니메이션을 뒤집어 버린다.
  const stageClass = `pg-stage${snapshot?.facing === "left" ? " pg-stage--flip" : ""}`;
  const petClass = `penguin ${snapshot ? behaviorClass(snapshot.behavior) : "pg--walk"}`;

  return (
    <div className={stageClass}>
      <Penguin
        className={petClass}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
      />
    </div>
  );
}
