import { load } from "@tauri-apps/plugin-store";
import { DEFAULT_TAUNTS, normalizeTaunts } from "./pet";

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

/** 50~150의 정수만 유효하다. 깨진 값은 100(원래 크기)으로 수렴한다 — 손으로 고친
 * 저장 파일이 화면을 덮는 펭귄을 만들면 안 된다. **10 단위로 강제하지는 않는다**:
 * 슬라이더가 이미 `step`으로 막고, 손으로 넣은 55를 100으로 되돌리면 더 놀란다. */
const sanitizeSize = (v: unknown): number =>
  typeof v === "number" && Number.isInteger(v) && v >= SIZE_MIN && v <= SIZE_MAX ? v : 100;

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
