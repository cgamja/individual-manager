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
    await expect(loadPetSettings()).resolves.toEqual({
      enabled: true,
      sound: false,
      pinball: false,
    });
  });

  it("저장된_값을_그대로_읽는다", async () => {
    mockStore({ pet: { enabled: false } });
    await expect(loadPetSettings()).resolves.toEqual({
      enabled: false,
      sound: false,
      pinball: false,
    });
  });

  it("깨진_값은_켜짐으로_수렴한다", async () => {
    // 펭귄이 원인 모르게 사라지는 것보다 켜져 있는 편이 낫다
    mockStore({ pet: { enabled: "yes" } });
    await expect(loadPetSettings()).resolves.toEqual({
      enabled: true,
      sound: false,
      pinball: false,
    });
  });

  it("핀볼은_저장된_적이_없으면_꺼짐이다", async () => {
    // **`enabled`와 반대 방향의 기본값이다.** 뒤집히면 착지 4단계가 통째로
    // 가려진 채로 처음 실행된다. Rust의 `핀볼_설정이_없으면_꺼짐이다`와 짝이다
    mockStore({ pet: { enabled: true } });
    await expect(loadPetSettings()).resolves.toMatchObject({ pinball: false });
  });

  it("핀볼만_저장해도_마릿수가_남는다", async () => {
    // 같은 `pet` 키 아래에 Rust가 쓰는 마릿수가 함께 산다 — 통째로 덮어쓰면
    // 모드를 켜는 것만으로 펭귄이 한 마리로 줄어든다 (이미 한 번 겪은 버그다)
    const data = mockStore();
    data.set("pet", { enabled: true, count: 4 });
    await savePetSettings({ pinball: true });
    expect(data.get("pet")).toMatchObject({ enabled: true, count: 4, pinball: true });
  });

  it("Rust가_읽는_키에_저장한다", async () => {
    // 키가 어긋나면 앱을 다시 켰을 때 설정이 무시된다
    const data = mockStore();
    await savePetSettings({ enabled: false });
    expect(data.get("pet")).toMatchObject({ enabled: false });
  });

  it("같은_키에_있는_마릿수를_덮어쓰지_않는다", async () => {
    // Rust가 같은 `pet` 객체에 count를 쓴다. 통째로 덮어쓰면 켜고 끄는 것만으로
    // 마릿수가 1로 돌아간다
    const data = mockStore();
    data.set("pet", { enabled: true, count: 3 });
    await savePetSettings({ enabled: false });
    expect(data.get("pet")).toEqual({ enabled: false, count: 3 });
  });

  it("소리는_기본이_꺼짐이다", async () => {
    // 상주 앱이 예고 없이 소리를 내면 회의 중에 사고가 난다 (PRD Q6)
    mockStore();
    expect((await loadPetSettings()).sound).toBe(false);
  });

  it("한_항목이_깨져도_나머지는_살린다", async () => {
    const data = mockStore();
    data.set("pet", { enabled: false, sound: "네" });
    const loaded = await loadPetSettings();
    expect(loaded).toEqual({ enabled: false, sound: false, pinball: false });
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
