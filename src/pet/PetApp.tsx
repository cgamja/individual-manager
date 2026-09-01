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
  /** **누른 지점**을 펭귄 기준으로 정규화한 값(-0.5~0.5). 핀볼 모드에서
   * 채가 어디를 쳤는지가 된다. 뗀 지점이 아니라 누른 지점을 쓴다 — 클릭으로
   * 판정되는 범위(4px)라 값은 거의 같지만 "어디를 쳤나"의 의미는 누른 쪽에 있다. */
  hitX: number;
  hitY: number;
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
  /** 직전 스냅샷의 동작 클래스 — 되감기 판정의 근거 (`shouldRestart` 참고). */
  const lastClassRef = useRef<string | null>(null);
  /** 눈동자가 커서를 향해 밀리는 양 (SVG 좌표, R7). */
  const [gaze, setGaze] = useState({ x: 0, y: 0 });
  /** 사용자가 팝오버에서 고칠 수 있으므로 저장소가 원천이다. */
  const [taunts, setTaunts] = useState<readonly string[]>(DEFAULT_TAUNTS);
  /** 이 창의 소리 전부 — 켜짐/꺼짐·쿨다운·컨텍스트 수명을 소유한다. */
  const playerRef = useRef<SoundPlayer | null>(null);
  /**
   * 소리 판정용 직전 스냅샷. **`useEffect`로 `snapshot` 변화를 보면 안 된다** —
   * React가 렌더를 배칭하면 중간 스냅샷이 통째로 스킵돼 소리가 샌다.
   * `lastClassRef`가 되감기 판정을 같은 방식으로 하고 있다.
   */
  const prevSnapRef = useRef<PetSnapshot | null>(null);
  /** 핀볼 여부 — 클릭 정산은 `useCallback([])`이라 스냅샷 상태를 못 본다. */
  const pinballRef = useRef(false);

  useEffect(() => {
    // 컨텍스트는 미리 만든다 — suspended여도 괜찮고, 제스처마다 깨운다 (KTD4).
    // 라벨은 목소리의 시드다: 같은 펭귄은 껐다 켜도 같은 목소리다 (R10)
    const player = new SoundPlayer(getCurrentWebviewWindow().label);
    playerRef.current = player;
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    (async () => {
      // 시작 값은 저장소에서 — 설정 창의 방송은 그 뒤의 변경만 실어 나른다
      const settings = await loadPetSettings().catch(() => null);
      if (!cancelled && settings) player.setEnabled(settings.sound);
      unlisten = await onPetSound((on) => player.setEnabled(on));
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
      // 첫 틱을 기다리지 않고 현재 상태부터 그린다
      const initial = await getPetState().catch(() => null);
      if (!cancelled && initial) {
        setSnapshot(initial);
        // 첫 스냅샷은 소리 판정의 기준점으로만 쓴다 — `soundsFor`가 prev
        // 없음(null)을 무음으로 치는 것과 같은 이유로, 여기서 재생하지 않는다
        prevSnapRef.current = initial;
      }
      const saved = await loadTaunts().catch(() => null);
      if (!cancelled && saved) setTaunts(saved);
      unlisten = await onPetState((next) => {
        setSnapshot(next);
        // 소리는 여기(콜백 안)에서 판정한다 — 스냅샷을 하나도 흘리지 않는다
        for (const name of soundsFor(prevSnapRef.current, next)) {
          playerRef.current?.play(name, performance.now());
        }
        prevSnapRef.current = next;
        // 새 대사가 나올 때마다 목록을 다시 읽는다. 팝오버는 다른 웹뷰라
        // 여기서 직접 알 방법이 없고, 몇 초에 한 번이라 비용도 미미하다
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
    // 어떤 버튼이든 제스처다 — suspended 오디오 컨텍스트를 깨울 유일한
    // 기회라 버튼 분기보다 앞에서 부른다 (KTD4)
    playerRef.current?.nudge();
    // 주 버튼만 드래그를 시작한다 — 우클릭까지 받으면 놓을 때 클릭으로 해석돼
    // 팝오버가 열린다. 이미 드래그 중이면 두 번째 포인터는 무시한다: 기준점을
    // 덮어쓰면 원래 포인터의 다음 이동량이 엉뚱한 값이 돼 펭귄이 순간이동한다.
    if (e.button !== 0) {
      // 오른쪽(그 외) 버튼 — 창을 연다. 왼쪽은 빠따가 가져갔다.
      // 왼쪽을 누르고 있는 중이면 무시한다: 마우스는 모든 버튼이 같은 pointerId를
      // 쓰기 때문에, 오른쪽을 떼는 pointerup이 진행 중인 드래그를 끝내 버린다
      if (e.button === 2 && !dragRef.current) void openPetPopover().catch(() => {});
      return;
    }
    if (dragRef.current) return;
    // 포인터를 캡처해야 커서가 펭귄 밖으로 나가도 드래그가 끊기지 않는다
    e.currentTarget.setPointerCapture(e.pointerId);
    const rect = e.currentTarget.getBoundingClientRect();
    const track: DragTrack = {
      pointerId: e.pointerId,
      screenX: e.screenX,
      screenY: e.screenY,
      moved: 0,
      // 창이 0×0으로 잡히는 순간(첫 페인트 전 등)에는 중앙으로 친 것으로 본다
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
        // 핀볼의 공중 재타격은 Thrown→Thrown이라 전이 검출(soundsFor)이 못
        // 본다 — 랠리가 첫 타 이후 전부 무음이 된다 (리뷰 #1). 핀볼에서
        // 펭귄 창 클릭은 곧 채 타격이므로 스냅샷을 기다리지 않고 재생한다.
        // 지상 첫 타는 전이 검출과 겹치는데, 쿨다운(150ms)이 둘째를 거른다
        if (pinballRef.current) playerRef.current?.play("whoosh", performance.now());
        // 거의 안 움직였으면 옮길 의도가 아니라 빠따다
        await whackPet(track.hitX, track.hitY).catch(() => {});
      } else {
        // 놓는 순간의 속도가 던지는 세기다 (R12)
        const { vx, vy } = throwVelocity(track.samples);
        await endPetDrag(vx, vy).catch(() => {});
      }
    })();
  }, []);

  // **창 전체를 방망이로.** `.penguin`에만 걸면 창 여백(펭귄 둘레)에서 커서가
  // 화살표로 되돌아와, 펭귄에 다가가는 동안 방망이가 한 번 끊긴다.
  useEffect(() => {
    const on = snapshot?.pinball ?? false;
    pinballRef.current = on;
    document.body.classList.toggle("pg-pinball-mode", on);
    return () => document.body.classList.remove("pg-pinball-mode");
  }, [snapshot?.pinball]);

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
    // 그림자는 동작이 아니라 "떠 있는가"로 지운다 — 공중에서 클릭하면 지상
    // 동작(반응)을 하면서도 떠 있어서, 동작으로 판정하면 그림자가 되살아난다
    snapshot?.air ? "pg-air" : "",
    // 핀볼이면 커서가 채가 된다. **저장소를 다시 읽지 않는다** — 스냅샷으로
    // 오므로 설정을 켠 순간 반영된다 (`Look`에 들어 있는 이유가 이것이다)
    snapshot?.pinball ? "pg-pinball" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const speech = snapshot?.speech ?? null;

  return (
    <>
      {/* 말풍선 — 창 위쪽 여백에 뜬다. key로 발화마다 다시 나타나게 한다 */}
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
