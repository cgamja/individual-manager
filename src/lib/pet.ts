import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export type Facing = "left" | "right";

/** 오르는 중인지 내려가는 중인지 — 몸 기울기를 고르는 데 쓴다. */
export type Vertical = "up" | "down" | "level";

export type IdleKind = "look_around" | "stretch" | "shake" | "shift_feet";

/** 얼음낚시 한 판이 거쳐 가는 국면. 어느 국면인지는 코어가 정한다. */
export type FishingPhase = "dig" | "wait" | "bite" | "catch" | "miss" | "pack";

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
  | { kind: "swing" }
  | { kind: "dragged" }
  | { kind: "falling" }
  | { kind: "thrown" }
  | { kind: "land" }
  | { kind: "splat" }
  | { kind: "sprawl" }
  | { kind: "tumble" }
  | { kind: "ice_fishing"; fishing: FishingPhase };

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
export const DEFAULT_TAUNTS: readonly string[] = [
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

/** 대사 한 줄의 최대 길이. 말풍선이 창을 넘지 않을 만큼만 받는다. */
export const TAUNT_MAX_LEN = 40;

/**
 * 추첨값으로 대사를 고른다. 목록이 비어도 터지지 않는다.
 * 목록을 인자로 받는 이유는 사용자가 설정에서 고칠 수 있기 때문이다.
 */
export const tauntFor = (roll: number, lines: readonly string[]): string =>
  lines.length === 0 ? "" : lines[Math.abs(roll) % lines.length];

/** 저장 전 정리 — 앞뒤 공백을 없애고, 빈 줄과 너무 긴 줄을 걸러낸다. */
export const normalizeTaunts = (lines: readonly string[]): string[] =>
  lines
    .map((l) => l.trim().replace(/\s+/g, " "))
    .filter((l) => l.length > 0)
    .map((l) => (l.length > TAUNT_MAX_LEN ? l.slice(0, TAUNT_MAX_LEN) : l));

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
  // 국면을 클래스에 싣지 않으면 30초 내내 한 그림으로 굳는다
  if (behavior.kind === "ice_fishing") return `pg--fishing-${kebab(behavior.fishing)}`;
  return `pg--${kebab(behavior.kind)}`;
};

/** 한 번만 재생하고 멈추는 동작인가. 같은 클래스가 다시 와도 되감아야 한다. */
export const isOneShot = (cls: string): boolean =>
  cls === "pg--turn" ||
  cls === "pg--land" ||
  cls === "pg--splat" ||
  cls === "pg--sprawl" ||
  cls === "pg--tumble" ||
  cls.startsWith("pg--sassy-") ||
  // 드리우기만 빼고 낚시 국면은 전부 한 번짜리다 — 드리우기는 입질이 올
  // 때까지 찌가 계속 까딱거려야 하므로 되감으면 끊긴다
  (cls.startsWith("pg--fishing-") && cls !== "pg--fishing-wait");

/**
 * 한 번짜리 애니메이션을 처음부터 다시 재생해야 하는가.
 *
 * **"한 번짜리다"만으로는 부족하다.** 스냅샷은 동작이 그대로여도 날아온다 —
 * 말풍선이 뜨거나 사라지기만 해도 브릿지의 `Look`이 달라져 다시 알린다.
 * 그때마다 되감으면 1.8초짜리 잡기 동작이 중간에 처음으로 돌아갔다가 코어가
 * 다음 동작으로 넘어가며 잘린다. 말은 7~18초마다 나오므로 자주 겹친다.
 *
 * 그래서 **새로 시작한 것인지**, 즉 클래스가 달라졌는지만 본다.
 *
 * 이 판정은 "같은 한 번짜리 클래스가 연달아 오지는 않는다"에 기댄다. 지금은
 * 참이다 — 착지·굴러떨어지기는 `get_up`을 거치고, 싸가지는 코어가 같은 종류를
 * 연속으로 고르지 않고, 낚시 국면은 사이에 드리우기가 낀다. 연달아 올 수 있는
 * 동작을 한 번짜리로 만들려면 여기에 구분자(누적 횟수 같은 것)를 함께 넘겨야 한다.
 */
export const shouldRestart = (prev: string | null, next: string): boolean =>
  isOneShot(next) && prev !== next;

/** 자기 창의 펭귄 상태. 펫 창이 아닌 곳에서 부르면 `null`이다. */
export const getPetState = (): Promise<PetSnapshot | null> => invoke("pet_get_state");

/** 팝오버가 버튼 상태를 정하는 데 쓰는 요약. */
export interface PetSummary {
  count: number;
  max: number;
  /** 마지막으로 우클릭된 펭귄 — "이 펭귄 삭제"의 대상. */
  focused: number | null;
}

export const getPetSummary = (): Promise<PetSummary> => invoke("pet_summary");

/** 펭귄 한 마리를 부른 펭귄 옆에 추가한다. 상한에 걸리면 reject된다. */
export const addPet = (): Promise<number> => invoke("pet_add");

/** 우클릭한 펭귄을 삭제한다. 마지막 한 마리면 reject된다. */
export const removePet = (): Promise<void> => invoke("pet_remove");

/**
 * 우클릭해서 연 그 펭귄에게 얼음낚시를 시킨다.
 *
 * 저절로는 십 분에 한 번쯤 나오는 동작이라 보고 싶을 때 못 본다.
 * 대상이 없거나 그 펭귄이 바닥에 없으면 사유와 함께 reject된다.
 */
export const fishPet = (): Promise<void> => invoke("pet_fish");

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

/**
 * 자기 창의 펭귄 상태만 구독한다.
 *
 * **`listen()`을 그냥 쓰면 안 된다.** 전역 `listen`은 대상을 `Any`로 등록하는데,
 * Tauri는 `Any` 리스너를 **emit 대상과 무관하게 전부** 호출한다(`listener.rs`의
 * `*target == EventTarget::Any || filter(...)`). 그래서 Rust가 `emit_to`로 한 창에만
 * 보내도 모든 펭귄이 남의 스냅샷까지 받아, 다 같이 동시에 떠들고 남이 맞은 빠따를
 * 자기가 휘두른다. 창에 묶인 리스너여야 그 창 대상 이벤트만 온다.
 */
export const onPetState = (cb: (snapshot: PetSnapshot) => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<PetSnapshot>(EVENT_PET_STATE, (event) => cb(event.payload));
