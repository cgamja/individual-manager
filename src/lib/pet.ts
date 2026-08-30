import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Facing = "left" | "right";

/** 오르는 중인지 내려가는 중인지 — 몸 기울기를 고르는 데 쓴다. */
export type Vertical = "up" | "down" | "level";

export type IdleKind = "look_around" | "stretch" | "shake" | "shift_feet";

export type Behavior =
  | { kind: "walk" }
  | { kind: "turn" }
  | { kind: "idle"; idle: IdleKind }
  | { kind: "swim" }
  | { kind: "sleep" }
  | { kind: "startled" }
  | { kind: "dragged" }
  | { kind: "falling" }
  | { kind: "thrown" }
  | { kind: "land" };

export interface PetSnapshot {
  x: number;
  y: number;
  facing: Facing;
  vertical: Vertical;
  behavior: Behavior;
}

export const EVENT_PET_STATE = "pet://state";

/** 클릭과 드래그를 가르는 이동량(px). 이보다 덜 움직였으면 클릭으로 본다. */
export const DRAG_THRESHOLD_PX = 4;

/**
 * 동작 → CSS 클래스. 유휴는 종류까지 내려가야 두리번과 기지개가 구분된다.
 * CSS는 이 클래스 하나만 보고 부위 애니메이션을 고른다 (KTD2).
 */
export const behaviorClass = (behavior: Behavior): string =>
  behavior.kind === "idle"
    ? `pg--idle-${behavior.idle.replace(/_/g, "-")}`
    : `pg--${behavior.kind}`;

export const getPetState = (): Promise<PetSnapshot> => invoke("pet_get_state");

/** 펭귄 클릭 — 놀라게 하고 팝오버를 연다. */
export const pokePet = (): Promise<void> => invoke("pet_poke");

export const startPetDrag = (): Promise<void> => invoke("pet_drag_start");

/** 이동량만 보낸다 — 창 위치는 Rust가 단독으로 소유한다 (KTD4). */
export const dragPetBy = (dx: number, dy: number): Promise<void> =>
  invoke("pet_drag_by", { dx, dy });

/** 놓는 순간의 속도(논리 px/초)를 함께 넘긴다 — 세게 던지면 포물선을 그린다. */
export const endPetDrag = (vx: number, vy: number): Promise<void> =>
  invoke("pet_drag_end", { vx, vy });

/** 세로 방향 → CSS 클래스. 지상 동작에서는 항상 level이라 기울지 않는다. */
export const verticalClass = (vertical: Vertical): string => `pg-v--${vertical}`;

/**
 * 최근 포인터 궤적에서 놓는 순간의 속도를 잰다 (논리 px/초).
 * 마지막 한 구간만 보면 손이 멈춘 채 뗀 경우 0이 되고, 전체를 보면 초반의
 * 느린 구간이 섞여 약해진다 — 그래서 최근 `windowMs` 구간만 본다.
 */
export const throwVelocity = (
  samples: readonly { x: number; y: number; t: number }[],
  windowMs = 120,
): { vx: number; vy: number } => {
  if (samples.length < 2) return { vx: 0, vy: 0 };
  const last = samples[samples.length - 1];
  // 창 안에서 가장 오래된 샘플을 기준점으로 삼는다
  let first = samples[0];
  for (const s of samples) {
    if (last.t - s.t <= windowMs) {
      first = s;
      break;
    }
  }
  const dt = (last.t - first.t) / 1000;
  if (dt <= 0) return { vx: 0, vy: 0 };
  return { vx: (last.x - first.x) / dt, vy: (last.y - first.y) / dt };
};

/** 펭귄을 켜고 끈다 (R8). 끄면 창이 닫힌다. */
export const setPetEnabled = (enabled: boolean): Promise<void> =>
  invoke("pet_set_enabled", { enabled });

export const onPetState = (cb: (snapshot: PetSnapshot) => void): Promise<UnlistenFn> =>
  listen<PetSnapshot>(EVENT_PET_STATE, (event) => cb(event.payload));
