import type { UnlistenFn } from "@tauri-apps/api/event";
import { load } from "@tauri-apps/plugin-store";
import { DEFAULT_TAUNTS, normalizeTaunts, onPetScale } from "./pet";

const STORE_FILE = "settings.json";
/** Rust의 `pet_bridge::PET_KEY`와 같은 키 — 시작 시점 판단을 Rust가 직접 읽는다. */
const PET_KEY = "pet";
const TAUNTS_KEY = "taunts";

export interface PetSettings {
  enabled: boolean;
  /** 효과음을 낼지. **기본은 꺼짐이다** (PRD Q6). */
  sound: boolean;
  /** 핀볼 모드. **기본은 꺼짐이다. */
  pinball: boolean;
  /** 음량 단계 0~4. 가운데(2)가 원래 크기(-18 dBFS)라 기존 사용자의 소리 */
  volume: number;
  /** 겉모습 테마 — 설정 창이 따른다 (2026-09-01 사용자 지시). 트레이 아이콘은 */
  theme: AppTheme;
  /** 펭귄 크기 **퍼센트** (50~150, 기본 100). 소품·물리가 함께 따라온다.
   * Rust는 이 값을 배율(0.5~1.5)로 바꿔 쓴다 — `pet_bridge/scale.rs`. */
  size: number;
}

/** Rust가 창을 만들 때 심어 준 배율 (`scale_init_script`). 저장소를 읽어 오기
 * 전에 쓰는 값이라, 없으면 1이다. */
declare global {
  interface Window {
    __PG_SCALE?: number;
  }
}

/** 첫 페인트에 쓸 배율. 저장소 왕복을 기다리면 창은 작은데 그림은 배율 1로
 * 한 프레임 그려져 잘린 펭귄이 번쩍인다. */
export function initialScale(): number {
  const v = typeof window !== "undefined" ? window.__PG_SCALE : undefined;
  return typeof v === "number" && Number.isFinite(v) && v > 0 ? v : 1;
}

/** 크기 슬라이더의 범위·단계. Rust의 `SIZE_MIN`·`SIZE_MAX`·`SIZE_STEP`과 짝이다. */
export const SIZE_MIN = 50;
export const SIZE_MAX = 150;
export const SIZE_STEP = 10;

export type AppTheme = "system" | "light" | "dark";

/** 셋 중 하나만 유효하다. 깨진 값은 시스템으로 수렴한다 — Rust의 `theme_from`과 짝. */
const sanitizeTheme = (v: unknown): AppTheme =>
  v === "light" || v === "dark" ? v : "system";

/** 0~4의 정수만 유효하다. 깨진 값은 가운데 단계(지금 크기)로 수렴한다. */
const sanitizeVolume = (v: unknown): number =>
  typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= 4 ? v : 2;

/** 가장 가까운 슬라이더 눈금으로 붙인다. 정확히 가운데면 위로 올린다 —
 * **Rust의 `snap_to_step`과 규칙이 같아야 한다.** 반올림 방향이 갈리면 55가
 * 한쪽에서는 60, 다른 쪽에서는 50이 된다. */
export function snapToStep(percent: number): number {
  const p = Math.min(Math.max(percent, SIZE_MIN), SIZE_MAX);
  const 칸 = Math.floor((p - SIZE_MIN + SIZE_STEP / 2) / SIZE_STEP);
  return Math.min(SIZE_MIN + 칸 * SIZE_STEP, SIZE_MAX);
}

/** 50~150의 정수만 유효하다. 깨진 값은 100(원래 크기)으로 수렴한다 — 손으로 고친
 * 저장 파일이 화면을 덮는 펭귄을 만들면 안 된다.
 *
 * **눈금 밖 값은 가까운 눈금으로 붙인다.** 55를 그냥 두면 배율과 라벨은 55%인데
 * 슬라이더 thumb는 `step`에 맞춰 60%에 서서 셋이 갈린다. */
const sanitizeSize = (v: unknown): number =>
  typeof v === "number" && Number.isInteger(v) && v >= SIZE_MIN && v <= SIZE_MAX
    ? snapToStep(v)
    : 100;

/** 펭귄은 기본 켜짐(사용자가 직접 요청한 기능이라 opt-in으로 숨기지 않는다), */
export const DEFAULT_PET_SETTINGS: PetSettings = {
  enabled: true,
  sound: false,
  pinball: false,
  volume: 2,
  theme: "system",
  size: 100,
};

/** 저장된 펭귄 설정을 로드한다. 깨진 값은 항목별로 기본값에 수렴시킨다 — */
export async function loadPetSettings(): Promise<PetSettings> {
  const store = await load(STORE_FILE);
  const value = await store.get<Partial<PetSettings>>(PET_KEY);
  return {
    enabled: typeof value?.enabled === "boolean" ? value.enabled : DEFAULT_PET_SETTINGS.enabled,
    sound: typeof value?.sound === "boolean" ? value.sound : DEFAULT_PET_SETTINGS.sound,
    pinball:
      typeof value?.pinball === "boolean" ? value.pinball : DEFAULT_PET_SETTINGS.pinball,
    volume: sanitizeVolume(value?.volume),
    theme: sanitizeTheme(value?.theme),
    size: sanitizeSize(value?.size),
  };
}

/** 펭귄 설정을 저장한다. Rust는 다음 실행의 시작 시점에 이 값을 읽는다. */
export async function savePetSettings(settings: Partial<PetSettings>): Promise<void> {
  const store = await load(STORE_FILE);
  const current = (await store.get<Record<string, unknown>>(PET_KEY)) ?? {};
  await store.set(PET_KEY, { ...current, ...settings });
}

/** 창이 크기 배율을 따라가는 **유일한 길**. 세 창(펭귄·판·코트)이 같은 부팅
 * 경쟁 위에 있어 한 곳에 모은다.
 *
 * 순서가 정확성이다:
 * 1. Rust가 창을 만들며 심어 준 값으로 **첫 페인트**를 맞춘다.
 * 2. **방송을 먼저 구독한다.** `listen`은 비동기라, 저장소부터 읽으면 그 사이에
 *    날아온 방송이 통째로 유실되고 옛 값이 새 값을 덮는다.
 * 3. 그다음 저장소를 읽는다. 읽는 동안 방송이 오면 세대가 올라가 이 결과를 버린다.
 *
 * 그리고 **화해자**: 창 크기를 바꾸는 것은 배율뿐이라 `resize`가 곧 "Rust가 크기를
 * 바꿨다"는 신호다. 방송이 유실돼도 여기서 스스로 낫는다. 값은 창 크기에서
 * 역산하지 않고 저장소에서 다시 읽는다 — 창 크기는 정수로 반올림돼 배율이 미세하게
 * 어긋나고, 그 값이 Rust의 히트 상자와 갈린다. */
export async function followPetScale(apply: (scale: number) => void): Promise<UnlistenFn> {
  apply(initialScale());
  let 세대 = 0;
  const 읽어_적용 = async () => {
    const 내_세대 = 세대;
    const saved = await loadPetSettings().catch(() => null);
    if (saved && 내_세대 === 세대) apply(saved.size / 100);
  };
  const unlisten = await onPetScale(({ size }) => {
    세대 += 1;
    apply(snapToStep(size) / 100);
  });
  window.addEventListener("resize", () => {
    세대 += 1;
    void 읽어_적용();
  });
  await 읽어_적용();
  return unlisten;
}

/** 펭귄이 할 말 목록. 저장된 게 없으면 기본 목록을 쓴다. */
export async function loadTaunts(): Promise<string[]> {
  const store = await load(STORE_FILE);
  const value = await store.get<unknown>(TAUNTS_KEY);
  if (!Array.isArray(value)) return [...DEFAULT_TAUNTS];
  const lines = normalizeTaunts(value.filter((v): v is string => typeof v === "string"));
  return lines;
}

export async function saveTaunts(lines: readonly string[]): Promise<void> {
  const store = await load(STORE_FILE);
  await store.set(TAUNTS_KEY, normalizeTaunts(lines));
}
