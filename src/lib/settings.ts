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
}

export type AppTheme = "system" | "light" | "dark";

/** 셋 중 하나만 유효하다. 깨진 값은 시스템으로 수렴한다 — Rust의 `theme_from`과 짝. */
const sanitizeTheme = (v: unknown): AppTheme =>
  v === "light" || v === "dark" ? v : "system";

/** 0~4의 정수만 유효하다. 깨진 값은 가운데 단계(지금 크기)로 수렴한다. */
const sanitizeVolume = (v: unknown): number =>
  typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= 4 ? v : 2;

/** 펭귄은 기본 켜짐(사용자가 직접 요청한 기능이라 opt-in으로 숨기지 않는다), */
export const DEFAULT_PET_SETTINGS: PetSettings = {
  enabled: true,
  sound: false,
  pinball: false,
  volume: 2,
  theme: "system",
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
