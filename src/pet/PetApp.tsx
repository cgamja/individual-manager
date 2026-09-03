import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Penguin } from "../assets/penguin";
import { shouldClickThrough } from "../assets/penguin/hit";
import { advance, newDragTrack, pushSample, type DragTrack } from "../lib/drag";
import { loadPetSettings, loadTaunts } from "../lib/settings";
import { SoundPlayer, soundsFor } from "./sound";
import {
  DRAG_THRESHOLD_PX,
  behaviorClass,
  gazeFor,
  normalizedIn,
  isFemalePet,
  shouldRestart,
  throwVelocity,
  verticalClass,
  dragPetBy,
  endPetDrag,
  getPetState,
  onPetSound,
  onPetState,
  openPetPopover,
  setPetClickThrough,
  startPetDrag,
  tauntFor,
  DEFAULT_TAUNTS,
  whackPet,
  type PetSnapshot,
  type RestartKey,
} from "../lib/pet";

/** 드래그 진행 정보. 공 창과 공유하는 부분은 `lib/drag`에 있다 — 여기서
 * 더하는 것은 **누른 지점**뿐이고, 그건 핀볼 모드에서만 쓴다. */
interface PetDragTrack extends DragTrack {
  /** 누른 지점을 펭귄 기준으로 정규화한 값(-0.5~0.5). */
  hitX: number;
  hitY: number;
  /** 포인터를 잡아 둔 요소 — 놓을 때 같은 것에서 풀어야 한다. */
  captured: Element | null;
}

/** 통과 요청을 다시 보내기까지의 최소 간격. 창이 아직 안 바뀐 채 커서가 여백을
 * 헤매는 동안 IPC가 쌓이는 것만 막는다 — 정상 경로에서는 창이 곧 통과로 바뀌어
 * 포인터 이벤트가 끊긴다. */
const CLICK_THROUGH_RESEND_MS = 200;

/** 바탕화면 펭귄의 웹뷰. 창 위치는 Rust가 소유하므로 여기서는 */
export function PetApp() {
  const [snapshot, setSnapshot] = useState<PetSnapshot | null>(null);
  const dragRef = useRef<PetDragTrack | null>(null);
  /** 코어가 Dragged로 넘어갔는지 — 그 전의 이동량은 pendingRef에 모은다. */
  const armedRef = useRef(false);
  const pendingRef = useRef({ dx: 0, dy: 0 });
  /** 진행 중인 pet_drag_start 왕복 — 놓기 정산이 이걸 기다린다. */
  const startPromiseRef = useRef<Promise<void> | null>(null);
  /** 한 번짜리 애니메이션을 되감기 위한 remount 카운터.
   *
   * **되감기는 SVG를 통째로 다시 만드는 것이라 대가가 있다** — 그 안에서 늘
   * 돌던 것(눈 깜빡임 5.5초 주기, 숨쉬기)도 함께 0으로 되돌아간다. 빠따가
   * 한 번짜리 목록에 들어오면서 **클릭할 때마다** 그렇게 되므로, 쉬지 않고
   * 클릭하는 동안에는 눈을 깜빡이지 않는다. 되감기 없이는 방망이가 아예 다시
   * 안 휘둘러지므로 지금은 이쪽을 택했다. 거슬리면 대안은 remount 대신
   * `getAnimations()`로 스윙 애니메이션만 되감는 것이다. */
  const [restartKey, setRestartKey] = useState(0);
  /** 직전 스냅샷의 동작 클래스와 빠따 횟수 — 되감기 판정의 근거
   * (`shouldRestart` 참고). 클래스만으로는 연타한 스윙을 구분할 수 없다. */
  const lastRestartRef = useRef<RestartKey | null>(null);
  /** 눈동자가 커서를 향해 밀리는 양 (SVG 좌표, R7). */
  const [gaze, setGaze] = useState({ x: 0, y: 0 });
  /** 무대의 실제 자리 — 창 전역 리스너가 포인터를 여기 기준으로 정규화한다. */
  const stageRef = useRef<HTMLDivElement | null>(null);
  /** 통과 요청을 마지막으로 보낸 시각. 재전송 묶기에만 쓴다. */
  const clickThroughAtRef = useRef(0);
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
        const key = { cls: behaviorClass(next.behavior), whackSeq: next.whack_seq };
        if (shouldRestart(lastRestartRef.current, key)) setRestartKey((k) => k + 1);
        lastRestartRef.current = key;
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
    // **루트가 아니라 눌린 도형을 잡는다** — 루트는 `pointer-events: none`이고,
    // 히트 테스트에서 빠진 요소로 캡처가 계속 오는지는 확실하지 않다.
    const captured = (e.target as Element | null) ?? e.currentTarget;
    try {
      captured.setPointerCapture(e.pointerId);
    } catch {
      // 못 걸어도 드래그는 진행한다 — 창을 벗어나면 끊길 뿐이다.
    }
    const hit = normalizedIn(e.currentTarget.getBoundingClientRect(), e.clientX, e.clientY);
    const track: PetDragTrack = {
      ...newDragTrack(e.pointerId, e.screenX, e.screenY),
      hitX: hit.nx,
      hitY: hit.ny,
      captured,
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
    // 시선은 창 전역 리스너가 본다 — 여기는 드래그만 맡는다.
    if (!track) return;
    if (track.pointerId !== e.pointerId) return;
    const moved = advance(track, e.screenX, e.screenY);
    if (!moved) return;
    const { dx, dy } = moved;
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
      track.captured?.releasePointerCapture(e.pointerId);
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

  /** 시선 추적(R7)과 클릭 통과 요청 — **창 전역에서 듣는다.** 실루엣만 포인터를
   * 받게 되면서(`base.css`) SVG에 걸면 시선이 펭귄 몸 위에서만 움직이고, 통과
   * 요청은 애초에 여백에서 나야 한다. 드래그만 캡처를 써서 SVG에 남는다.
   *
   * **커서가 창 밖으로 나갈 때는 요청하지 않는다** — 안전한 쪽으로 틀린다. */
  useEffect(() => {
    const stage = stageRef.current;
    const onMove = (e: PointerEvent) => {
      if (dragRef.current) return;
      const rect = stage?.getBoundingClientRect();
      if (!rect || rect.width === 0 || rect.height === 0) return;
      const { nx, ny } = normalizedIn(rect, e.clientX, e.clientY);

      // 무대의 실제 자리에서 치수를 뽑는다 — CSS 변수를 파싱하지 않는다.
      // 무대가 커지거나 여백이 바뀌어도 저절로 따라간다.
      const want = shouldClickThrough(e.clientX, e.clientY, {
        size: rect.width,
        padX: rect.left,
        padTop: rect.top,
      });
      // **이 리스너가 불렸다는 것 자체가 창이 클릭을 먹고 있다는 뜻이다** —
      // 통과 중이면 포인터 이벤트가 아예 안 온다. Rust는 되돌릴 때 요청까지
      // 지우므로(걸쇠), 여기서 "이미 보냈다"를 믿으면 다시는 못 켠다.
      // 그래서 보낸 기억이 아니라 **관측**으로 다시 판단한다.
      if (!want) {
        clickThroughAtRef.current = 0;
        setGaze(gazeFor(nx, ny));
        return;
      }
      // **통과가 걸리면 이 리스너가 조용해진다** — 커서가 창을 나가도
      // `pointerleave`가 안 온다. 시선을 지금 가운데로 돌려놓지 않으면 눈동자가
      // 마지막 표본(대개 ±1.6 최대치)에 얼어붙어 계속 한쪽을 노려본다 (R7).
      setGaze({ x: 0, y: 0 });
      // 켜지지 않은 채 커서가 여백을 헤매면 매 이동마다 IPC가 나가므로 묶는다.
      // 정상 경로에서는 창이 곧 통과로 바뀌어 이 리스너가 조용해진다.
      const now = performance.now();
      if (now - clickThroughAtRef.current < CLICK_THROUGH_RESEND_MS) return;
      clickThroughAtRef.current = now;
      void setPetClickThrough(true).catch(() => {
        clickThroughAtRef.current = 0;
      });
    };
    // `pointerout`은 도형 사이를 지날 때마다 터져 시선이 계속 0으로 튄다.
    // `pointerleave`는 이 요소의 subtree를 **정말 벗어날 때만** 온다.
    const onLeave = () => setGaze({ x: 0, y: 0 });
    const root = document.documentElement;
    window.addEventListener("pointermove", onMove);
    root.addEventListener("pointerleave", onLeave);
    return () => {
      window.removeEventListener("pointermove", onMove);
      root.removeEventListener("pointerleave", onLeave);
    };
  }, []);

  // 창 라벨에서 결정적으로 뽑는다 — 렌더마다 다시 계산해도 같은 값이다.
  const female = isFemalePet(getCurrentWebviewWindow().label);

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
      <div className={stageClass} ref={stageRef}>
        <Penguin
        key={restartKey}
        className={petClass}
        female={female}
        style={
          { "--gaze-x": `${gaze.x}px`, "--gaze-y": `${gaze.y}px` } as React.CSSProperties
        }
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
