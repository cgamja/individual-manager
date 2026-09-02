import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export type Facing = "left" | "right";

/** 오르는 중인지 내려가는 중인지 — 몸 기울기를 고르는 데 쓴다. */
export type Vertical = "up" | "down" | "level";

export type IdleKind = "look_around" | "stretch" | "shake" | "shift_feet";

/** 얼음낚시 한 판이 거쳐 가는 국면. 어느 국면인지는 코어가 정한다. */
export type FishingPhase = "dig" | "wait" | "bite" | "catch" | "miss" | "pack";

/** 발작 한 판이 거쳐 가는 국면. 어느 국면인지는 코어가 정한다. */
export type FreakoutPhase = "dash" | "pant";

/** 볼링 한 판에서 **마리 하나가** 거쳐 가는 국면. 판 전체의 국면은 코어 안에만
 * 있다 — 웹뷰는 자기 펭귄이 무엇을 하는지만 알면 된다.
 *
 * **"맞은 상태"가 없다.** 맞은 핀은 `thrown`이 되어 평소 던져졌을 때의 그림을
 * 그대로 쓴다 (2026-09-02 사용자 지시). */
export type BowlingPhase = "gather" | "ready" | "scatter";

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
  | { kind: "squawk" }
  | { kind: "freakout"; freakout: FreakoutPhase }
  | { kind: "dragged" }
  | { kind: "falling" }
  | { kind: "thrown" }
  | { kind: "land" }
  | { kind: "splat" }
  | { kind: "sprawl" }
  | { kind: "tumble" }
  | { kind: "slide" }
  | { kind: "ice_fishing"; fishing: FishingPhase }
  | { kind: "bowling"; bowling: BowlingPhase };

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
  /** 핀볼 모드인가. 커서를 채로 바꾸는 데 쓴다. */
  pinball: boolean;
  behavior: Behavior;
}

/** 펭귄이 하는 킹받는 말들. */
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

/** 추첨값으로 대사를 고른다. 목록이 비어도 터지지 않는다. */
export const tauntFor = (roll: number, lines: readonly string[]): string =>
  lines.length === 0 ? "" : lines[Math.abs(roll) % lines.length];

/** 저장 전 정리 — 앞뒤 공백을 없애고, 빈 줄과 너무 긴 줄을 걸러낸다. */
export const normalizeTaunts = (lines: readonly string[]): string[] =>
  lines
    .map((l) => l.trim().replace(/\s+/g, " "))
    .filter((l) => l.length > 0)
    .map((l) => (l.length > TAUNT_MAX_LEN ? l.slice(0, TAUNT_MAX_LEN) : l));

export const EVENT_PET_STATE = "pet://state";

/** 공 창이 구독하는 상태 이벤트. 펭귄과 나누는 이유는 받는 창과 페이로드가
 * 다르기 때문이다 — 하나로 합치면 공 창이 마릿수만큼의 이벤트를 걸러 내야 한다. */
export const EVENT_BALL_STATE = "bowling://ball";

/** 볼링 판이 **끝났을 때** 온다. 판을 끝내는 것은 공이지 사용자가 아니라서,
 * 이게 없으면 "볼링 한 판" 버튼이 비활성인 채로 남는다. */
export const EVENT_BOWLING_OVER = "bowling://over";

/** 설정이 **이 창 밖에서** 바뀌었을 때 오는 알림 (핀볼 판의 Esc 등). */
export const EVENT_PET_SETTINGS = "pet://settings";

/** 효과음 설정의 방송. `pet://settings`에 얹지 않는 이유: 그쪽은 **Rust가 */
export const EVENT_PET_SOUND = "pet://sound";

/** 클릭과 드래그를 가르는 이동량(px). 이보다 덜 움직였으면 클릭으로 본다. */
export const DRAG_THRESHOLD_PX = 4;

/** 동작 → CSS 클래스. 유휴는 종류까지 내려가야 두리번과 기지개가 구분된다. */
/** Rust의 snake_case를 CSS 선택자에 쓰는 kebab-case로. 한 곳에서만 바꾼다. */
const kebab = (s: string): string => s.replace(/_/g, "-");

export const behaviorClass = (behavior: Behavior): string => {
  if (behavior.kind === "idle") return `pg--idle-${kebab(behavior.idle)}`;
  if (behavior.kind === "sassy") return `pg--sassy-${kebab(behavior.sassy)}`;
  if (behavior.kind === "ice_fishing") return `pg--fishing-${kebab(behavior.fishing)}`;
  if (behavior.kind === "freakout") return `pg--freakout-${kebab(behavior.freakout)}`;
  if (behavior.kind === "bowling") return `pg--bowling-${kebab(behavior.bowling)}`;
  return `pg--${kebab(behavior.kind)}`;
};

/** 한 번만 재생하고 멈추는 동작인가. 같은 클래스가 다시 와도 되감아야 한다. */
export const isOneShot = (cls: string): boolean =>
  cls === "pg--turn" ||
  cls === "pg--land" ||
  cls === "pg--splat" ||
  cls === "pg--sprawl" ||
  cls === "pg--tumble" ||
  cls === "pg--slide" ||
  cls === "pg--squawk" ||
  cls === "pg--freakout-pant" ||
  cls === "pg--bowling-scatter" ||
  cls === "pg--swing" ||
  cls.startsWith("pg--sassy-") ||
  (cls.startsWith("pg--fishing-") && cls !== "pg--fishing-wait");

/** 되감기 판정에 필요한 것 — 동작 클래스와 빠따 횟수. */
export interface RestartKey {
  cls: string;
  whackSeq: number;
}

/** 한 번짜리 애니메이션을 처음부터 다시 재생해야 하는가.
 *
 * **클래스가 그대로여도 자극이 새로 왔으면 되감아야 한다.** 방망이가 그
 * 경우다 — 360ms 안에 다시 때리면 코어는 `Swing`을 다시 걸지만 클래스가 안
 * 바뀌어 브라우저가 애니메이션을 재생하지 않는다. `whack_seq`가 "새로 때렸다"의
 * 유일한 신호다.
 *
 * **되감기를 방망이로만 한정한다.** 빽빽거리기는 때리는 동안 판이 계속
 * 연장되므로(`MOTIONS.md` 빽빽거리기 절), 매 클릭에 되감으면 애니메이션의 첫
 * 100ms만 반복하며 **영원히 부풀기만 한다.** 그건 이 항목이 고치려는 것과
 * 정확히 반대다. */
export const shouldRestart = (prev: RestartKey | null, next: RestartKey): boolean => {
  if (!isOneShot(next.cls)) return false;
  if (prev === null || prev.cls !== next.cls) return true;
  return next.cls === "pg--swing" && next.whackSeq > prev.whackSeq;
};

/** 자기 창의 펭귄 상태. 펫 창이 아닌 곳에서 부르면 `null`이다. */
export const getPetState = (): Promise<PetSnapshot | null> => invoke("pet_get_state");

/** 팝오버가 버튼 상태를 정하는 데 쓰는 요약. */
export interface PetSummary {
  count: number;
  max: number;
  /** 마지막으로 우클릭된 펭귄 — "이 펭귄 삭제"의 대상. */
  focused: number | null;
  /** 볼링 판이 도는 중인가. 도는 중에 또 누르면 무시되므로 버튼을 끈다 (A3). */
  bowling: boolean;
}

/** 볼링 공의 상태. **위치는 여기 없다** — 창이 옮기므로, 넣으면 굴러가는
 * 내내 20Hz로 리렌더한다. */
export interface BallSnapshot {
  x: number;
  y: number;
  rolling: boolean;
  held: boolean;
}

export const getPetSummary = (): Promise<PetSummary> => invoke("pet_summary");

/** 펭귄 한 마리를 부른 펭귄 옆에 추가한다. 상한에 걸리면 reject된다. */
export const addPet = (): Promise<number> => invoke("pet_add");

/** 우클릭한 펭귄을 삭제한다. 마지막 한 마리면 reject된다. */
export const removePet = (): Promise<void> => invoke("pet_remove");

/** 우클릭해서 연 그 펭귄에게 얼음낚시를 시킨다. */
export const fishPet = (): Promise<void> => invoke("pet_fish");

/** 우클릭해서 연 그 펭귄을 미끄러뜨린다. */
export const slidePet = (): Promise<void> => invoke("pet_slide");

/** 우클릭해서 연 그 펭귄을 빽빽거리게 한다. */
export const squawkPet = (): Promise<void> => invoke("pet_squawk");

/** 우클릭해서 연 그 펭귄을 발작시킨다. */
export const freakoutPet = (): Promise<void> => invoke("pet_freakout");

/** 볼링 한 판을 연다. **우클릭한 한 마리가 아니라 화면의 펭귄 전부**가
 * 참여한다 (R1) — 그래서 다른 동작들과 달리 대상을 안 고른다. */
export const startBowling = (): Promise<void> => invoke("bowling_start");

/** 공을 집는다. 굴러가는 중이면 `false` — 한 판에 한 번 굴린다. */
export const startBallDrag = (): Promise<boolean> => invoke("ball_drag_start");

/** 공의 가로 이동량만 보낸다 — 조준 각도가 없다 (R6). 펭귄과 마찬가지로
 * 창 위치의 소유자는 Rust 하나다. */
export const dragBallBy = (dx: number): Promise<void> => invoke("ball_drag_by", { dx });

/** 공을 놓는다. **가로 속도만** 넘긴다 — 세로는 버린다 (R6). */
export const endBallDrag = (vx: number): Promise<void> => invoke("ball_drag_end", { vx });

/** 빠따 — 왼쪽 클릭 한 번에 한 번 날아간다 (참고: 쇼핑카트히어로). */
/** 왼쪽 클릭을 코어에 넘긴다. `nx`/`ny`는 **맞은 지점**을 펭귄 기준으로 */
export const whackPet = (nx: number, ny: number): Promise<void> =>
  invoke("pet_whack", { nx, ny });

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

/** 최근 포인터 궤적에서 놓는 순간의 속도를 잰다 (논리 px/초). */
export const throwVelocity = (
  samples: readonly { x: number; y: number; t: number }[],
  windowMs = 120,
): { vx: number; vy: number } => {
  if (samples.length < 2) return { vx: 0, vy: 0 };
  const last = samples[samples.length - 1];
  const prior = samples.slice(0, -1);
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

/** 테마를 지금 떠 있는 창·트레이에 건다. 저장은 `savePetSettings`가 따로 한다 — */
export const setPetTheme = (theme: string): Promise<void> =>
  invoke("pet_set_theme", { theme });

/** 펭귄을 켜고 끈다 (R8). 끄면 창이 닫힌다. */
export const setPetEnabled = (enabled: boolean): Promise<void> =>
  invoke("pet_set_enabled", { enabled });

/** 핀볼 모드를 켜고 끈다. **살아 있는 전 마리에 즉시 걸린다** — 앱 전역 */
export const setPetPinball = (on: boolean): Promise<void> =>
  invoke("pet_set_pinball", { on });

/** 자기 창의 펭귄 상태만 구독한다. */
export const onPetState = (cb: (snapshot: PetSnapshot) => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<PetSnapshot>(EVENT_PET_STATE, (event) => cb(event.payload));

/** 자기 창의 공 상태만 구독한다. 펭귄과 같은 이유로 **창에 묶는다.** */
export const onBallState = (cb: (ball: BallSnapshot) => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<BallSnapshot>(EVENT_BALL_STATE, (event) => cb(event.payload));

/** 볼링 판이 끝나면 알려 준다. 설정 창이 버튼을 되살리는 데 쓴다. */
export const onBowlingOver = (cb: () => void): Promise<UnlistenFn> =>
  listen(EVENT_BOWLING_OVER, () => cb());

/** 설정이 이 창 밖에서 바뀌면 알려 준다 — 지금은 핀볼 판의 Esc가 유일한 경우다. */
export const onPetSettings = (
  cb: (settings: { pinball: boolean }) => void,
): Promise<UnlistenFn> =>
  listen<{ pinball: boolean }>(EVENT_PET_SETTINGS, (event) => cb(event.payload));

/** 효과음 토글이 바뀌면 알려 준다 — 설정 창이 보내고 펭귄 창들이 듣는다. */
export const onPetSound = (
  cb: (settings: { sound: boolean; volume: number }) => void,
): Promise<UnlistenFn> =>
  listen<{ sound: boolean; volume: number }>(EVENT_PET_SOUND, (event) =>
    cb(event.payload),
  );

/** 효과음 설정을 방송한다 — 프론트가 직접 emit한다. Rust를 거치지 않는다 (KTD2). */
export const emitPetSound = (on: boolean, volume: number): Promise<void> =>
  emit(EVENT_PET_SOUND, { sound: on, volume });
