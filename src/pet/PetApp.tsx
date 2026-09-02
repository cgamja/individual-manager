import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Penguin } from "./Penguin";
import { loadPetSettings, loadTaunts } from "../lib/settings";
import { SoundPlayer, soundsFor } from "./sound";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
  shouldRestart,
  throwVelocity,
  verticalClass,
  dragPetBy,
  endPetDrag,
  getPetState,
  onPetSound,
  onPetState,
  openPetPopover,
  startPetDrag,
  tauntFor,
  DEFAULT_TAUNTS,
  whackPet,
  type PetSnapshot,
} from "../lib/pet";

/** 드래그 진행 정보 — 화면 좌표 기준의 직전 지점과 누적 이동량. */
interface DragTrack {
  /** 이 드래그를 소유한 포인터. 다른 포인터의 이벤트는 무시한다. */
  pointerId: number;
  screenX: number;
  screenY: number;
  moved: number;
  /** **누른 지점**을 펭귄 기준으로 정규화한 값(-0.5~0.5). 핀볼 모드에서 */
  hitX: number;
  hitY: number;
  /** 최근 궤적 — 놓는 순간의 속도를 재는 데 쓴다 (던지기 세기). */
  samples: { x: number; y: number; t: number }[];
}

/** 속도 계산에 남겨 둘 궤적의 길이. 개수로 자르면 포인터 보고 주기(60Hz vs 120Hz)에 */
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

/** 바탕화면 펭귄의 웹뷰. 창 위치는 Rust가 소유하므로 여기서는 */
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
  /** 직전 스냅샷의 동작 클래스 — 되감기 판정의 근거 (`shouldRestart` 참고). */
  const lastClassRef = useRef<string | null>(null);
  /** 눈동자가 커서를 향해 밀리는 양 (SVG 좌표, R7). */
  const [gaze, setGaze] = useState({ x: 0, y: 0 });
  /** 사용자가 팝오버에서 고칠 수 있으므로 저장소가 원천이다. */
  const [taunts, setTaunts] = useState<readonly string[]>(DEFAULT_TAUNTS);
  /** 이 창의 소리 전부 — 켜짐/꺼짐·쿨다운·컨텍스트 수명을 소유한다. */
  const playerRef = useRef<SoundPlayer | null>(null);
  /** 소리 판정용 직전 스냅샷. **`useEffect`로 `snapshot` 변화를 보면 안 된다** — */
  const prevSnapRef = useRef<PetSnapshot | null>(null);
  /** 핀볼 여부 — 클릭 정산은 `useCallback([])`이라 스냅샷 상태를 못 본다. */
  const pinballRef = useRef(false);

  useEffect(() => {
    const player = new SoundPlayer(getCurrentWebviewWindow().label);
    playerRef.current = player;
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    (async () => {
      const settings = await loadPetSettings().catch(() => null);
      if (!cancelled && settings) {
        player.setEnabled(settings.sound);
        player.setVolume(settings.volume);
      }
      unlisten = await onPetSound(({ sound, volume }) => {
        player.setEnabled(sound);
        player.setVolume(volume);
      });
    })();
    return () => {
      cancelled = true;
      unlisten?.();
      player.close();
      playerRef.current = null;
    };
  }, []);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    (async () => {
      const initial = await getPetState().catch(() => null);
      if (!cancelled && initial) {
        setSnapshot(initial);
        prevSnapRef.current = initial;
      }
      const saved = await loadTaunts().catch(() => null);
      if (!cancelled && saved) setTaunts(saved);
      unlisten = await onPetState((next) => {
        setSnapshot(next);
        for (const name of soundsFor(prevSnapRef.current, next)) {
          playerRef.current?.play(name, performance.now());
        }
        prevSnapRef.current = next;
        if (next.speech) loadTaunts().then(setTaunts).catch(() => {});
        const cls = behaviorClass(next.behavior);
        if (shouldRestart(lastClassRef.current, cls)) setRestartKey((k) => k + 1);
        lastClassRef.current = cls;
      });
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    playerRef.current?.nudge();
    if (e.button !== 0) {
      if (e.button === 2 && !dragRef.current) void openPetPopover().catch(() => {});
      return;
    }
    if (dragRef.current) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const rect = e.currentTarget.getBoundingClientRect();
    const track: DragTrack = {
      pointerId: e.pointerId,
      screenX: e.screenX,
      screenY: e.screenY,
      moved: 0,
      hitX: rect.width > 0 ? (e.clientX - rect.left) / rect.width - 0.5 : 0,
      hitY: rect.height > 0 ? (e.clientY - rect.top) / rect.height - 0.5 : 0,
      samples: [{ x: e.screenX, y: e.screenY, t: performance.now() }],
    };
    dragRef.current = track;
    armedRef.current = false;
    pendingRef.current = { dx: 0, dy: 0 };
    const started = startPetDrag();
    startPromiseRef.current = started;
    void started.then(() => {
      if (dragRef.current !== track) return;
      armedRef.current = true;
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
      const rect = e.currentTarget.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        const nx = (e.clientX - rect.left) / rect.width - 0.5;
        const ny = (e.clientY - rect.top) / rect.height - 0.5;
        setGaze({ x: clampGaze(nx * 4), y: clampGaze(ny * 4) });
      }
      return;
    }
    if (track.pointerId !== e.pointerId) return;
    const dx = e.screenX - track.screenX;
    const dy = e.screenY - track.screenY;
    if (dx === 0 && dy === 0) return;
    track.screenX = e.screenX;
    track.screenY = e.screenY;
    track.moved += Math.abs(dx) + Math.abs(dy);
    pushSample(track, e.screenX, e.screenY);
    if (!armedRef.current) {
      pendingRef.current.dx += dx;
      pendingRef.current.dy += dy;
      return;
    }
    void dragPetBy(dx, dy);
  }, []);

  const handlePointerUp = useCallback((e: React.PointerEvent<SVGSVGElement>) => {
    const track = dragRef.current;
    if (!track || track.pointerId !== e.pointerId) return;
    pushSample(track, e.screenX, e.screenY);
    dragRef.current = null;
    armedRef.current = false;
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
    }

    void (async () => {
      await startPromiseRef.current?.catch(() => {});
      const pending = pendingRef.current;
      if (pending.dx !== 0 || pending.dy !== 0) {
        pendingRef.current = { dx: 0, dy: 0 };
        await dragPetBy(pending.dx, pending.dy).catch(() => {});
      }
      if (track.moved < DRAG_THRESHOLD_PX) {
        if (pinballRef.current) playerRef.current?.play("whoosh", performance.now());
        await whackPet(track.hitX, track.hitY).catch(() => {});
      } else {
        const { vx, vy } = throwVelocity(track.samples);
        await endPetDrag(vx, vy).catch(() => {});
      }
    })();
  }, []);

  useEffect(() => {
    const on = snapshot?.pinball ?? false;
    pinballRef.current = on;
    document.body.classList.toggle("pg-pinball-mode", on);
    return () => document.body.classList.remove("pg-pinball-mode");
  }, [snapshot?.pinball]);

  const handlePointerLeave = useCallback(() => {
    setGaze({ x: 0, y: 0 });
  }, []);

  const stageClass = `pg-stage${snapshot?.facing === "left" ? " pg-stage--flip" : ""}`;
  const petClass = [
    "penguin",
    snapshot ? behaviorClass(snapshot.behavior) : "pg--walk",
    verticalClass(snapshot?.vertical ?? "level"),
    snapshot?.air ? "pg-air" : "",
    snapshot?.pinball ? "pg-pinball" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const speech = snapshot?.speech ?? null;

  return (
    <>
      {speech && (
        <div className="pg-bubble" key={speech.seq} role="status">
          {tauntFor(speech.roll, taunts)}
        </div>
      )}
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
          onContextMenu={(e) => e.preventDefault()}
        />
      </div>
    </>
  );
}
