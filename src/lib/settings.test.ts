import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "./settings";

afterEach(() => {
  clearMocks();
});

/** store 플러그인 IPC를 인메모리 맵으로 흉내낸다. */
function mockStore(initial: Record<string, unknown> = {}) {
  const data = new Map(Object.entries(initial));
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "plugin:store|load":
        return 1;
      case "plugin:store|get":
        return data.has(a.key as string)
          ? [data.get(a.key as string), true]
          : [null, false];
      case "plugin:store|set":
        data.set(a.key as string, a.value);
        return undefined;
      default:
        return undefined;
    }
  });
  return data;
}

describe("설정 저장소", () => {
  it("저장된_값이_없으면_기본값_25_5를_반환한다", async () => {
    mockStore();
    expect(await loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  // Covers AE4: 설정 저장 후 다시 로드하면 같은 값이 유지된다
  it("저장_후_같은_값을_다시_로드한다", async () => {
    mockStore();
    await saveSettings({ focus_minutes: 50, break_minutes: 10 });
    expect(await loadSettings()).toEqual({ focus_minutes: 50, break_minutes: 10 });
  });
});
