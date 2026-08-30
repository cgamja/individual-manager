import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Facing = "left" | "right";

export type IdleKind = "look_around" | "stretch" | "shake" | "shift_feet";

export type Behavior =
  | { kind: "walk" }
  | { kind: "turn" }
  | { kind: "idle"; idle: IdleKind }
  | { kind: "sleep" }
  | { kind: "startled" }
  | { kind: "dragged" }
  | { kind: "falling" }
  | { kind: "land" };

export interface PetSnapshot {
  x: number;
  y: number;
  facing: Facing;
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

export const endPetDrag = (): Promise<void> => invoke("pet_drag_end");

export const onPetState = (cb: (snapshot: PetSnapshot) => void): Promise<UnlistenFn> =>
  listen<PetSnapshot>(EVENT_PET_STATE, (event) => cb(event.payload));
