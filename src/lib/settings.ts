import { load } from "@tauri-apps/plugin-store";
import { DEFAULT_TAUNTS, normalizeTaunts } from "./pet";

const STORE_FILE = "settings.json";
/** Rust의 `pet_bridge::PET_KEY`와 같은 키 — 시작 시점 판단을 Rust가 직접 읽는다. */
const PET_KEY = "pet";
const TAUNTS_KEY = "taunts";

export interface PetSettings {
  enabled: boolean;
  /**
   * 효과음을 낼지. **기본은 꺼짐이다** (PRD Q6).
   *
   * 상주 앱이 예고 없이 소리를 내면 회의 중에 사고가 난다. 켜는 것은 사용자의
   * 선택이어야 한다.
   */
  sound: boolean;
}

/** 펭귄은 기본 켜짐(사용자가 직접 요청한 기능이라 opt-in으로 숨기지 않는다),
 * 소리는 기본 꺼짐(예고 없이 소리를 내면 사고가 난다). */
export const DEFAULT_PET_SETTINGS: PetSettings = { enabled: true, sound: false };

/** 저장된 펭귄 설정을 로드한다. 깨진 값은 항목별로 기본값에 수렴시킨다 —
 * 한 항목이 깨졌다고 나머지까지 되돌리면 펭귄이 조용히 사라진다. */
export async function loadPetSettings(): Promise<PetSettings> {
  const store = await load(STORE_FILE);
  const value = await store.get<Partial<PetSettings>>(PET_KEY);
  return {
    enabled: typeof value?.enabled === "boolean" ? value.enabled : DEFAULT_PET_SETTINGS.enabled,
    sound: typeof value?.sound === "boolean" ? value.sound : DEFAULT_PET_SETTINGS.sound,
  };
}

/**
 * 펭귄 설정을 저장한다. Rust는 다음 실행의 시작 시점에 이 값을 읽는다.
 *
 * **읽고-고쳐-쓰기여야 한다.** 같은 `pet` 키 아래에 Rust가 쓰는 마릿수(`count`)가
 * 함께 살아서, 객체를 통째로 덮어쓰면 켜고 끄는 것만으로 마릿수가 1로 돌아간다.
 */
export async function savePetSettings(settings: Partial<PetSettings>): Promise<void> {
  const store = await load(STORE_FILE);
  const current = (await store.get<Record<string, unknown>>(PET_KEY)) ?? {};
  await store.set(PET_KEY, { ...current, ...settings });
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
