import { load } from "@tauri-apps/plugin-store";
import { DEFAULT_TAUNTS, normalizeTaunts } from "./pet";
import type { TimerConfig } from "./timer";

export const DEFAULT_SETTINGS: TimerConfig = {
  focus_minutes: 25,
  break_minutes: 5,
};

const STORE_FILE = "settings.json";
const TIMER_KEY = "timer";
/** Rust의 `pet_bridge::PET_KEY`와 같은 키 — 시작 시점 판단을 Rust가 직접 읽는다. */
const PET_KEY = "pet";
const TAUNTS_KEY = "taunts";

export interface PetSettings {
  enabled: boolean;
}

/** 사용자가 직접 요청한 기능이라 기본은 켜짐이다 (A2). */
export const DEFAULT_PET_SETTINGS: PetSettings = { enabled: true };

/** 저장된 타이머 설정을 로드한다. 없으면 기본값(25/5). */
export async function loadSettings(): Promise<TimerConfig> {
  const store = await load(STORE_FILE);
  const value = await store.get<TimerConfig>(TIMER_KEY);
  return value ?? DEFAULT_SETTINGS;
}

/** 타이머 설정을 저장한다 (store 플러그인 자동 저장). */
export async function saveSettings(config: TimerConfig): Promise<void> {
  const store = await load(STORE_FILE);
  await store.set(TIMER_KEY, config);
}

/** 저장된 펭귄 설정을 로드한다. 없으면 켜짐. */
export async function loadPetSettings(): Promise<PetSettings> {
  const store = await load(STORE_FILE);
  const value = await store.get<PetSettings>(PET_KEY);
  // 저장된 값이 깨져 있어도 켜짐으로 수렴시킨다 — 펭귄이 조용히 사라지지 않게
  return typeof value?.enabled === "boolean" ? value : DEFAULT_PET_SETTINGS;
}

/** 펭귄 설정을 저장한다. Rust는 다음 실행의 시작 시점에 이 값을 읽는다. */
export async function savePetSettings(settings: PetSettings): Promise<void> {
  const store = await load(STORE_FILE);
  await store.set(PET_KEY, settings);
}

/**
 * 펭귄이 할 말 목록. 저장된 게 없으면 기본 목록을 쓴다.
 *
 * 펭귄 창과 팝오버가 서로 다른 웹뷰라 이 저장소가 둘 사이의 유일한 통로다.
 * 펭귄 창은 새 대사가 나올 때마다 다시 읽어 편집을 곧바로 반영한다.
 */
export async function loadTaunts(): Promise<string[]> {
  const store = await load(STORE_FILE);
  const value = await store.get<unknown>(TAUNTS_KEY);
  if (!Array.isArray(value)) return [...DEFAULT_TAUNTS];
  const lines = normalizeTaunts(value.filter((v): v is string => typeof v === "string"));
  // 전부 지웠다면 그 뜻을 존중한다 — 기본값으로 되살리지 않는다.
  // 저장된 적이 없는 것(위 Array 검사)과는 다른 상태다
  return lines;
}

export async function saveTaunts(lines: readonly string[]): Promise<void> {
  const store = await load(STORE_FILE);
  await store.set(TAUNTS_KEY, normalizeTaunts(lines));
}
