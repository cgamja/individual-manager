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

/** 비치발리볼 한 판에서 **마리 하나가** 거쳐 가는 국면. 판 전체의 국면은 코어
 * 안에만 있다 — 웹뷰는 자기 펭귄이 무엇을 하는지만 알면 된다.
 *
 * `cheer`/`sulk`는 **싸가지 반응의 그림을 CSS에서 재사용한다.** 국면을
 * `Volleyball` 안에 남긴 이유는 그래야 축하하는 동안에도 옷이 남기 때문이다. */
/** 단체 야차의 마리 국면. 판 국면은 Rust가 갖고 웹뷰는 이것만 본다. */
export type YachaPhase =
  | "gather"
  | "hunt"
  | "circle"
  | "back"
  | "guard"
  | "punch"
  | "hurt"
  | "down"
  | "win"
  | "champ";

export type VolleyPhase =
  | "gather"
  | "ready"
  | "chase"
  | "bump"
  | "cheer"
  | "sulk";

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
  | { kind: "dont_ask" }
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
  | { kind: "bowling"; bowling: BowlingPhase }
  | { kind: "volleyball"; volley: VolleyPhase }
  | { kind: "yacha"; yacha: YachaPhase };

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
  /** 야차에서 **이 마리가 이번 라운드의 대표 타격**으로 뽑힌 누적 횟수.
   * 늘어날 때마다 "퍽"이 한 발 난다 — 라운드마다 딱 한 마리만 오른다. */
  punch_seq: number;
  /** 그 한 발이 쓰러뜨린 한 방인가. 반음을 낮춰 더 낮고 길게 낸다. */
  punch_down: boolean;
  /** 그 한 발이 막혔는가. 화남 표시가 회색으로 뜨고 소리도 둔탁하다.
   *
   * **국면으로는 알 수 없다** — 막히면 맞은 쪽이 `Guard` 그대로다. */
  punch_blocked: boolean;
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

/** 안물의 문구. **대사 목록과 별도 채널이다** — 목록을 다 지워도 이건 나온다.
 * 코어는 동작만 알고 문구는 웹뷰가 갖는다 (PRINCIPLE 4). */
export const DONT_ASK_LINE = "묻지 않았습니다~~";

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

/** 비치볼 창이 구독하는 상태 이벤트. 공 창이 따로라 이벤트도 따로다. */
export const EVENT_YACHA_QUEEN = "yacha://queen";
export const EVENT_YACHA_OVER = "yacha://over";
export const EVENT_VOLLEY_STATE = "volley://ball";

/** 비치발리볼 판이 **끝났을 때** 온다. 판을 끝내는 것은 예산이지 사용자가
 * 아니라서, 이게 없으면 "비치발리볼 한 판" 버튼이 비활성인 채로 남는다. */
export const EVENT_VOLLEY_OVER = "volley://over";

/** 설정이 **이 창 밖에서** 바뀌었을 때 오는 알림 (핀볼 판의 Esc 등). */
export const EVENT_PET_SETTINGS = "pet://settings";

/** 효과음 설정의 방송. `pet://settings`에 얹지 않는 이유: 그쪽은 **Rust가 */
export const EVENT_PET_SOUND = "pet://sound";

/** 크기 배율의 방송. 창 크기는 Rust가 정하지만 **그림을 줄이는 것은 웹뷰**라
 * (`--pg-scale`), 살아 있는 창들이 새 배율을 알아야 한다. */
export const EVENT_PET_SCALE = "pet://scale";

/** 클릭과 드래그를 가르는 이동량(px). 이보다 덜 움직였으면 클릭으로 본다. */
export const DRAG_THRESHOLD_PX = 4;

/** 창 px → 펭귄 기준 정규화 좌표(-0.5~0.5).
 *
 * **웹뷰의 px가 그림 좌표로 넘어가는 유일한 문이다.** 빠따 히트(`nx`/`ny`)와
 * 시선이 둘 다 여기를 지난다. 크기 배율이 들어와도 식이 안 바뀌는 이유는
 * 나누는 값이 **그때그때의 요소 크기**라서다 — 배율이 이미 그 안에 들어 있다. */
export function normalizedIn(
  rect: { left: number; top: number; width: number; height: number },
  clientX: number,
  clientY: number,
): { nx: number; ny: number } {
  return {
    nx: rect.width > 0 ? (clientX - rect.left) / rect.width - 0.5 : 0,
    ny: rect.height > 0 ? (clientY - rect.top) / rect.height - 0.5 : 0,
  };
}

/** 눈동자가 흰자를 벗어나지 않을 만큼만 움직인다 (**SVG user unit**). */
export const GAZE_LIMIT = 1.6;

/** 정규화 좌표 → 눈동자 이동량(SVG user unit).
 *
 * **배율 항이 없는 것이 맞다.** 들어오는 값은 이미 정규화됐고, 나가는 값은
 * SVG user unit이다 — `.pg-gaze`의 `translate()`가 그 공간에서 해석하므로
 * 그림 전체를 얼마로 줄이든 눈동자는 같은 자리를 가리킨다. 창 px를 그대로
 * 넣으면 그 순간 배율만큼 어긋난다. */
export function gazeFor(nx: number, ny: number): { x: number; y: number } {
  const clamp = (v: number) => Math.max(-GAZE_LIMIT, Math.min(GAZE_LIMIT, v));
  return { x: clamp(nx * 4), y: clamp(ny * 4) };
}

/** 이 펭귄이 암컷인가 — **창 라벨(`pet-<id>`)에서 결정적으로 뽑는다.**
 *
 * 성별은 **웹뷰 소유**다 (PRINCIPLE 4): "어떻게 보이는지"라 Rust는 모른다.
 * 효과음의 목소리 높이(`voiceOffsetFor`)가 이미 같은 방식이다.
 *
 * **난수를 안 쓴다** — 같은 시드가 같은 결과를 내야 하고(PRINCIPLE 3), id는
 * 증가만 하므로 **앱을 껐다 켜도 같은 펭귄은 같은 성별**이다. 그래서
 * 저장하지도 않는다 (PRD §7).
 *
 * **홀짝으로 가르면 안 된다.** 처음에 `(id * 11) % 2`로 썼는데 그건 `id % 2`와
 * 같아서 곱수가 아무 일도 안 했고, **팀 배정(`assign_sides`)도 id 홀짝 교대**라
 * 매 판이 **여자팀 대 남자팀**이 됐다. 팀 편성과 성별은 무관해야 한다.
 *
 * 그래서 곱한 뒤 **윗자리 비트를 꺼낸다** — 낮은 자리는 곱셈으로 안 섞이고
 * 입력의 홀짝을 그대로 물고 있다. 목소리 오프셋(`voiceOffsetFor`)과도 다른
 * 상수·다른 자리라 "높은 목소리 = 암컷" 같은, 아무도 요구하지 않은 규칙이 안 생긴다.
 *
 * 마릿수가 짝수여도 **성비는 짝수가 아닐 수 있다.** 그래도 된다. */
export const isFemalePet = (label: string): boolean => {
  const m = /^pet-(\d+)$/.exec(label);
  if (!m) return false;
  // Knuth 승산 해시. 32비트로 자른 뒤 16번째 비트를 본다.
  const h = (Number(m[1]) * 2654435761) >>> 0;
  return ((h >>> 16) & 1) === 1;
};

/** 동작 → CSS 클래스. 유휴는 종류까지 내려가야 두리번과 기지개가 구분된다. */
/** Rust의 snake_case를 CSS 선택자에 쓰는 kebab-case로. 한 곳에서만 바꾼다. */
const kebab = (s: string): string => s.replace(/_/g, "-");

export const behaviorClass = (behavior: Behavior): string => {
  if (behavior.kind === "idle") return `pg--idle-${kebab(behavior.idle)}`;
  if (behavior.kind === "sassy") return `pg--sassy-${kebab(behavior.sassy)}`;
  if (behavior.kind === "ice_fishing") return `pg--fishing-${kebab(behavior.fishing)}`;
  if (behavior.kind === "freakout") return `pg--freakout-${kebab(behavior.freakout)}`;
  if (behavior.kind === "bowling") return `pg--bowling-${kebab(behavior.bowling)}`;
  if (behavior.kind === "volleyball") return `pg--volley-${kebab(behavior.volley)}`;
  if (behavior.kind === "yacha") return `pg--yacha-${kebab(behavior.yacha)}`;
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
  cls === "pg--dont-ask" ||
  cls === "pg--freakout-pant" ||
  cls === "pg--bowling-scatter" ||
  cls === "pg--volley-bump" ||
  cls === "pg--volley-cheer" ||
  cls === "pg--volley-sulk" ||
  cls === "pg--yacha-punch" ||
  cls === "pg--yacha-hurt" ||
  cls === "pg--yacha-win" ||
  cls === "pg--swing" ||
  cls.startsWith("pg--sassy-") ||
  (cls.startsWith("pg--fishing-") && cls !== "pg--fishing-wait");

/** 되감기 판정에 필요한 것 — 동작 클래스와 빠따 횟수. */
export interface RestartKey {
  cls: string;
  whackSeq: number;
  /** 야차의 대표 타격 번호. **연타를 구분하는 유일한 값이다.** */
  punchSeq: number;
}

/** 한 번짜리 애니메이션을 처음부터 다시 재생해야 하는가.
 *
 * **실제로 방망이를 고치는 것은 `isOneShot`에 `pg--swing`을 넣은 쪽이다.**
 * 클릭 한 번은 `pointerdown`(→`Dragged`)과 `pointerup`(→`Swing`)으로 두 번
 * 알려지므로 클래스가 매번 `pg--dragged` → `pg--swing`으로 바뀐다 — 되감기를
 * 막고 있던 것은 `pg--swing`이 한 번짜리 목록에 없다는 사실 하나였다.
 * **아래 `whackSeq` 분기를 믿고 `isOneShot`에서 `pg--swing`을 빼면 안 된다.**
 *
 * `whackSeq` 분기는 **보험이다** — 사이의 `Dragged` 알림이 사라져 같은 클래스가
 * 연달아 올 때를 위한 것이다. `whack_seq`가 "새로 때렸다"의 유일한 신호다.
 *
 * **되감기를 방망이로만 한정한다.** 빽빽거리기는 때리는 동안 판이 계속
 * 연장되므로(`MOTIONS.md` 빽빽거리기 절), 매 클릭에 되감으면 애니메이션의 첫
 * 100ms만 반복하며 **영원히 부풀기만 한다.** 그건 이 항목이 고치려는 것과
 * 정확히 반대다. */
export const shouldRestart = (prev: RestartKey | null, next: RestartKey): boolean => {
  if (prev === null) return isOneShot(next.cls);
  // **야차의 연타는 클래스가 안 바뀐다.** 스윙 뒤 또 스윙이 64%라 국면이
  // `Punch` 그대로고, 막힌 주먹은 맞은 쪽을 `Guard` 그대로 둔다. 그래서
  // 번호로만 구분된다 — `pg--swing`이 `whackSeq`로 구분되는 것과 같은 자리다.
  // **일회성 클래스인지 안 따진다**: 막힌 주먹은 `Guard`(반복 자세)인 채로
  // 화남 표시만 다시 떠야 하기 때문이다.
  if (next.punchSeq > prev.punchSeq) return true;
  if (!isOneShot(next.cls)) return false;
  if (prev.cls !== next.cls) return true;
  return next.cls === "pg--swing" && next.whackSeq > prev.whackSeq;
};

/** 자기 창의 펭귄 상태. 펫 창이 아닌 곳에서 부르면 `null`이다. */
export const getPetState = (): Promise<PetSnapshot | null> => invoke("pet_get_state");

/** 포인터가 펭귄 밖이니 창을 통과시켜 달라고 알린다 (`on = false`면 거둔다).
 *
 * **요청일 뿐이다** — 실제로 플래그를 걸고 되돌리는 것은 Rust 틱이다. 통과
 * 중에는 이 창에 포인터 이벤트가 안 오므로 웹뷰는 스스로 되돌릴 수 없다. */
export const setPetClickThrough = (on: boolean): Promise<void> =>
  invoke("pet_set_click_through", { on });

/** 팝오버가 버튼 상태를 정하는 데 쓰는 요약. */
export interface PetSummary {
  count: number;
  max: number;
  /** 마지막으로 우클릭된 펭귄 — "이 펭귄 삭제"의 대상. */
  focused: number | null;
  /** 볼링 판이 도는 중인가. 도는 중에 또 누르면 무시되므로 버튼을 끈다 (A3). */
  bowling: boolean;
  /** 비치발리볼 판이 도는 중인가. **두 판은 서로를 배제하므로** 어느 쪽이든
   * 도는 동안 버튼 둘이 함께 비활성된다. */
  volleyball: boolean;
  yacha: boolean;
}

/** 볼링 공의 상태. **위치는 여기 없다** — 창이 옮기므로, 넣으면 굴러가는
 * 내내 20Hz로 리렌더한다. */
export interface BallSnapshot {
  x: number;
  y: number;
  rolling: boolean;
  held: boolean;
}

/** 비치볼의 상태. **위치는 창이 옮기므로 겉모습에 안 들어간다** — 넣으면
 * 날아가는 내내 20Hz로 리렌더한다. */
export interface VolleyBallSnapshot {
  x: number;
  y: number;
  /** 날아가는 중인가 — 도는 그림을 그리는 데 쓴다. */
  flying: boolean;
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

/** 우클릭해서 연 그 펭귄에게 안물을 시킨다. */
export const dontAskPet = (): Promise<void> => invoke("pet_dont_ask");

/** 볼링 한 판을 연다. **우클릭한 한 마리가 아니라 화면의 펭귄 전부**가
 * 참여한다 (R1) — 그래서 다른 동작들과 달리 대상을 안 고른다. */
export const startBowling = (): Promise<void> => invoke("bowling_start");

/** 비치발리볼 한 판을 연다. **사용자 입력이 없는 유일한 판이다** — 버튼을
 * 누르면 20초쯤 알아서 놀고 끝난다. 볼링과 마찬가지로 화면의 펭귄 전부가
 * 참여하므로 대상을 안 고른다. 두 마리부터 열린다. */
export const startVolleyball = (): Promise<void> => invoke("volleyball_start");

/** 단체 야차 한 판. 볼링·발리볼과 셋이 서로를 배제한다. */
export const startYacha = (): Promise<void> => invoke("yacha_start");

/** 미녀 펭귄의 자세. 자리는 Rust가 창을 옮겨 정한다. */
export type QueenPose = "walk_in" | "belting" | "clap" | "walk_out";

export interface QueenSnapshot {
  x: number;
  y: number;
  facing: Facing;
  pose: QueenPose;
}

/** 미녀 창이 **처음 뜰 때** 한 번 받아 간다 — 구독만으로는 첫 국면을 놓친다. */
export const getQueenState = (): Promise<QueenSnapshot | null> =>
  invoke("yacha_get_queen");

export const onQueenState = (
  cb: (queen: QueenSnapshot) => void,
): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<QueenSnapshot>(EVENT_YACHA_QUEEN, (event) =>
    cb(event.payload),
  );

export const onYachaOver = (cb: () => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen(EVENT_YACHA_OVER, () => cb());

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

/** **저장된** 크기를 지금 떠 있는 창에 건다. 값을 안 넘기는 이유는 크기도 자리도
 * 진실 원천이 저장소 하나여야 해서다 — 반드시 저장한 **뒤에** 부른다. */
export const applyPetSize = (): Promise<void> => invoke("pet_apply_size");

/** 자기 창의 펭귄 상태만 구독한다. */
export const onPetState = (cb: (snapshot: PetSnapshot) => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<PetSnapshot>(EVENT_PET_STATE, (event) => cb(event.payload));

/** 자기 창의 공 상태만 구독한다. 펭귄과 같은 이유로 **창에 묶는다.** */
export const onBallState = (cb: (ball: BallSnapshot) => void): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<BallSnapshot>(EVENT_BALL_STATE, (event) => cb(event.payload));

/** 볼링 판이 끝나면 알려 준다. 설정 창이 버튼을 되살리는 데 쓴다. */
export const onBowlingOver = (cb: () => void): Promise<UnlistenFn> =>
  listen(EVENT_BOWLING_OVER, () => cb());

/** 비치볼 창이 **뜨자마자** 현재 상태를 한 번 받아 간다.
 *
 * **없으면 공이 판 내내 안 돈다** — 틱이 창을 만들고 같은 호출에서 첫 상태를
 * 보내는데 그때 이 파일은 아직 실행되지도 않았다. 그 뒤로는 "달라진 게 없다"로
 * 걸러져 다시 안 온다. */
export const getVolleyState = (): Promise<VolleyBallSnapshot | null> =>
  invoke("volley_get_state");

/** 자기 창의 비치볼 상태만 구독한다. 펭귄·볼링 공과 같은 이유로 **창에 묶는다.** */
export const onVolleyState = (
  cb: (ball: VolleyBallSnapshot) => void,
): Promise<UnlistenFn> =>
  getCurrentWebviewWindow().listen<VolleyBallSnapshot>(EVENT_VOLLEY_STATE, (event) =>
    cb(event.payload),
  );

/** 비치발리볼 판이 끝나면 알려 준다. */
export const onVolleyOver = (cb: () => void): Promise<UnlistenFn> =>
  listen(EVENT_VOLLEY_OVER, () => cb());

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

/** 크기가 바뀌면 알려 준다 — 커서 방망이를 다시 그리는 창들이 듣는다. */
export const onPetScale = (cb: (settings: { size: number }) => void): Promise<UnlistenFn> =>
  listen<{ size: number }>(EVENT_PET_SCALE, (event) => cb(event.payload));

/** 크기(퍼센트)를 방송한다. 효과음과 같은 경로다. */
export const emitPetScale = (size: number): Promise<void> =>
  emit(EVENT_PET_SCALE, { size });
