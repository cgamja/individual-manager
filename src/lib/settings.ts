import { load } from "@tauri-apps/plugin-store";
import type { TimerConfig } from "./timer";

export const DEFAULT_SETTINGS: TimerConfig = {
  focus_minutes: 25,
  break_minutes: 5,
};

const STORE_FILE = "settings.json";
const TIMER_KEY = "timer";

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
