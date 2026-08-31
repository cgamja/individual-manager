import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_TAUNTS } from "./pet";
import {
  loadPetSettings,
  savePetSettings,
  loadTaunts,
  saveTaunts,
} from "./settings";

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

describe("펭귄 설정", () => {
  it("저장된_값이_없으면_켜짐이_기본이다", async () => {
    mockStore();
    await expect(loadPetSettings()).resolves.toEqual({ enabled: true });
  });

  it("저장된_값을_그대로_읽는다", async () => {
    mockStore({ pet: { enabled: false } });
    await expect(loadPetSettings()).resolves.toEqual({ enabled: false });
  });

  it("깨진_값은_켜짐으로_수렴한다", async () => {
    // 펭귄이 원인 모르게 사라지는 것보다 켜져 있는 편이 낫다
    mockStore({ pet: { enabled: "yes" } });
    await expect(loadPetSettings()).resolves.toEqual({ enabled: true });
  });

  it("Rust가_읽는_키에_저장한다", async () => {
    // 키가 어긋나면 앱을 다시 켰을 때 설정이 무시된다
    const data = mockStore();
    await savePetSettings({ enabled: false });
    expect(data.get("pet")).toEqual({ enabled: false });
  });
});

describe("펭귄 대사 목록", () => {
  it("저장된_적이_없으면_기본_목록을_쓴다", async () => {
    mockStore();
    await expect(loadTaunts()).resolves.toEqual([...DEFAULT_TAUNTS]);
  });

  it("전부_지운_상태를_기본값으로_되살리지_않는다", async () => {
    // 저장된 적 없음(기본값)과 사용자가 다 지운 것은 다른 상태다
    mockStore({ taunts: [] });
    await expect(loadTaunts()).resolves.toEqual([]);
  });

  it("저장할_때_앞뒤_공백과_빈_줄을_정리한다", async () => {
    const data = mockStore();
    await saveTaunts(["  일 안 해요?  ", "", "   ", "아  진짜   왜요"]);
    // 연속 공백도 하나로 줄인다 — 말풍선에서 어색하게 벌어진다
    expect(data.get("taunts")).toEqual(["일 안 해요?", "아 진짜 왜요"]);
  });

  it("너무_긴_줄은_잘라_말풍선을_넘기지_않는다", async () => {
    const data = mockStore();
    await saveTaunts(["가".repeat(120)]);
    const saved = data.get("taunts") as string[];
    expect(saved[0].length).toBeLessThanOrEqual(40);
  });

  it("문자열이_아닌_값이_섞여_있어도_버티고_거른다", async () => {
    // 손으로 고친 settings.json이 깨져 있어도 펭귄이 죽으면 안 된다
    mockStore({ taunts: ["멀쩡한 줄", 42, null, { a: 1 }] });
    await expect(loadTaunts()).resolves.toEqual(["멀쩡한 줄"]);
  });

  it("배열이_아니면_기본_목록으로_돌아간다", async () => {
    mockStore({ taunts: "망가진 값" });
    await expect(loadTaunts()).resolves.toEqual([...DEFAULT_TAUNTS]);
  });
});
