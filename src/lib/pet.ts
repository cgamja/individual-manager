import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Facing = "left" | "right";

/** 오르는 중인지 내려가는 중인지 — 몸 기울기를 고르는 데 쓴다. */
export type Vertical = "up" | "down" | "level";

export type IdleKind = "look_around" | "stretch" | "shake" | "shift_feet";

/** 클릭했을 때의 반응 — 놀라지 않고 싸가지 없게 군다. */
export type SassyKind =
  | "turn_away"
  | "head_flick"
  | "wing_flick"
  | "eye_roll"
  | "butt_wiggle";

export type Behavior =
  | { kind: "walk" }
  | { kind: "turn" }
  | { kind: "idle"; idle: IdleKind }
  | { kind: "swim" }
  | { kind: "sleep" }
  | { kind: "sassy"; sassy: SassyKind }
  | { kind: "wind_up" }
  | { kind: "swing" }
  | { kind: "dragged" }
  | { kind: "falling" }
  | { kind: "thrown" }
  | { kind: "land" };

/** 지금 떠 있는 말풍선. 문구는 코어가 아니라 여기가 갖는다 — 대사는 표현이다. */
export interface Speech {
  /** 발화 번호. 같은 대사가 연달아 나와도 새 말풍선으로 알아본다. */
  seq: number;
  /** 대사 추첨값. `TAUNTS[roll % TAUNTS.length]`로 고른다. */
  roll: number;
}

export interface PetSnapshot {
  x: number;
  y: number;
  facing: Facing;
  vertical: Vertical;
  /** 바닥에서 떠 있는가. 동작만으로는 알 수 없다 (공중에서 클릭한 경우). */
  air: boolean;
  speech: Speech | null;
  /** 빠따를 맞은 누적 횟수. 늘어날 때마다 방망이를 한 번 휘두른다. */
  whack_seq: number;
  behavior: Behavior;
}

/**
 * 펭귄이 하는 킹받는 말들.
 *
 * 코어는 추첨값만 주고 문구는 여기서 고른다 — 문구를 고치자고 Rust를 다시
 * 빌드할 이유가 없다. 늘리려면 줄만 추가하면 된다.
 */
export const TAUNTS: readonly string[] = [
  "그래서 뭐 어쩌라고",
  "일 안 해요?",
  "그거 마감 언제였더라~",
  "손가락 안 아파요?",
  "또 클릭이야? 되게 심심한가 봐",
  "지금 딴짓하는 거 다 보여요",
  "그 코드 어제도 안 됐잖아요",
  "저 근무 중인데요",
  "때린다고 빨라지나",
  "월급 루팡 신고할까",
  "카톡 답장은 하고 이러는 거죠?",
  "쉬는 거 아니고 대기 중입니다",
  "내가 왜 여기 있어야 하지",
  "그거 아까도 실패했어요",
  "커밋은 하고 노는 거예요?",
  "빠따 맞을 짓은 그쪽이 했는데",
  "테스트는 돌려보고 하는 말이죠?",
  "아 진짜 왜요",
  "그 PR 리뷰 언제 볼 건데",
  "생산성 0.02% 감소했습니다",
  "이럴 시간에 한 줄이라도",
  "펭귄한테 화풀이하지 마세요",
];

/** 추첨값으로 대사를 고른다. 목록이 비어도 터지지 않는다. */
export const tauntFor = (roll: number): string =>
  TAUNTS.length === 0 ? "" : TAUNTS[Math.abs(roll) % TAUNTS.length];

export const EVENT_PET_STATE = "pet://state";

/** 클릭과 드래그를 가르는 이동량(px). 이보다 덜 움직였으면 클릭으로 본다. */
export const DRAG_THRESHOLD_PX = 4;

/**
 * 동작 → CSS 클래스. 유휴는 종류까지 내려가야 두리번과 기지개가 구분된다.
 * CSS는 이 클래스 하나만 보고 부위 애니메이션을 고른다 (KTD2).
 */
/** Rust의 snake_case를 CSS 선택자에 쓰는 kebab-case로. 한 곳에서만 바꾼다. */
const kebab = (s: string): string => s.replace(/_/g, "-");

export const behaviorClass = (behavior: Behavior): string => {
  if (behavior.kind === "idle") return `pg--idle-${kebab(behavior.idle)}`;
  if (behavior.kind === "sassy") return `pg--sassy-${kebab(behavior.sassy)}`;
  return `pg--${kebab(behavior.kind)}`;
};

/** 한 번만 재생하고 멈추는 동작인가. 같은 클래스가 다시 와도 되감아야 한다. */
export const isOneShot = (cls: string): boolean =>
  cls === "pg--turn" || cls === "pg--land" || cls.startsWith("pg--sassy-");

export const getPetState = (): Promise<PetSnapshot> => invoke("pet_get_state");

/** 빠따 — 왼쪽 클릭 한 번에 한 번 날아간다 (참고: 쇼핑카트히어로). */
export const whackPet = (): Promise<void> => invoke("pet_whack");

/** 오른쪽 클릭 — 펭귄 옆에서 타이머·설정 창을 연다. 왼쪽은 빠따가 가져갔다. */
export const openPetPopover = (): Promise<void> => invoke("pet_open_popover");

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
  const prior = samples.slice(0, -1);
  // 창 안에서 가장 오래된 샘플을 기준점으로 삼는다. 창 안에 마지막 샘플밖에
  // 없으면 직전 샘플로 물러난다 — 기준점을 마지막 샘플로 잡으면 dt가 0이 되어
  // 정상적인 던지기가 통째로 0이 된다(포인터 보고가 드문 경우).
  let first = prior[prior.length - 1];
  for (const s of prior) {
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
