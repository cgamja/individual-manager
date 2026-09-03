import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_TAUNTS } from "./pet";
import {
  DEFAULT_PET_SETTINGS,
  loadPetSettings,
  savePetSettings,
  loadTaunts,
  saveTaunts,
  SIZE_MAX,
  SIZE_MIN,
  SIZE_STEP,
  snapToStep,
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
      volume: 2,
      theme: "system",
      size: 100,
    });
  });

  it("저장된_값을_그대로_읽는다", async () => {
    mockStore({ pet: { enabled: false } });
    await expect(loadPetSettings()).resolves.toEqual({
      enabled: false,
      sound: false,
      pinball: false,
      volume: 2,
      theme: "system",
      size: 100,
    });
  });

  it("깨진_값은_켜짐으로_수렴한다", async () => {
    mockStore({ pet: { enabled: "yes" } });
    await expect(loadPetSettings()).resolves.toEqual({
      enabled: true,
      sound: false,
      pinball: false,
      volume: 2,
      theme: "system",
      size: 100,
    });
  });

  it("핀볼은_저장된_적이_없으면_꺼짐이다", async () => {
    mockStore({ pet: { enabled: true } });
    await expect(loadPetSettings()).resolves.toMatchObject({ pinball: false });
  });

  it("핀볼만_저장해도_마릿수가_남는다", async () => {
    const data = mockStore();
    data.set("pet", { enabled: true, count: 4 });
    await savePetSettings({ pinball: true });
    expect(data.get("pet")).toMatchObject({ enabled: true, count: 4, pinball: true });
  });

  it("Rust가_읽는_키에_저장한다", async () => {
    const data = mockStore();
    await savePetSettings({ enabled: false });
    expect(data.get("pet")).toMatchObject({ enabled: false });
  });

  it("같은_키에_있는_마릿수를_덮어쓰지_않는다", async () => {
    const data = mockStore();
    data.set("pet", { enabled: true, count: 3 });
    await savePetSettings({ enabled: false });
    expect(data.get("pet")).toEqual({ enabled: false, count: 3 });
  });

  it("소리는_기본이_꺼짐이다", async () => {
    mockStore();
    expect((await loadPetSettings()).sound).toBe(false);
  });

  it("음량은_저장된_적이_없으면_가운데_단계다", async () => {
    mockStore({ pet: { enabled: true, sound: true } });
    expect((await loadPetSettings()).volume).toBe(2);
  });

  it("저장된_음량을_그대로_읽는다", async () => {
    mockStore({ pet: { volume: 4 } });
    expect((await loadPetSettings()).volume).toBe(4);
  });

  it("깨진_음량은_가운데_단계로_수렴한다", async () => {
    for (const bad of ["크게", 9, -1, 1.5, null]) {
      mockStore({ pet: { volume: bad } });
      expect((await loadPetSettings()).volume, String(bad)).toBe(2);
    }
  });

  it("크기는_저장된_적이_없으면_100퍼센트다", async () => {
    mockStore({ pet: { enabled: true, sound: true } });
    expect((await loadPetSettings()).size).toBe(100);
  });

  it("저장된_크기를_그대로_읽는다", async () => {
    mockStore({ pet: { size: 60 } });
    expect((await loadPetSettings()).size).toBe(60);
  });

  it("범위를_벗어나거나_깨진_크기는_100으로_수렴한다", async () => {
    // 손으로 고친 저장 파일이 화면을 덮는 펭귄을 만들면 안 된다.
    for (const bad of ["크게", 5000, 49, 151, 0, -30, 60.5, null]) {
      mockStore({ pet: { size: bad } });
      expect((await loadPetSettings()).size, String(bad)).toBe(100);
    }
  });

  it("눈금_밖_크기는_가까운_눈금으로_붙는다", async () => {
    // 55를 그냥 두면 배율과 라벨은 55%인데 슬라이더 thumb는 `step`에 맞춰 60%에
    // 서서 셋이 갈린다. **Rust의 `snap_to_step`과 같은 결과여야 한다.**
    for (const [저장, 기대] of [
      [55, 60],
      [54, 50],
      [56, 60],
      [61, 60],
      [149, 150],
    ] as const) {
      mockStore({ pet: { size: 저장 } });
      expect((await loadPetSettings()).size, String(저장)).toBe(기대);
    }
  });

  it("눈금_위의_크기는_그대로다", async () => {
    for (let p = SIZE_MIN; p <= SIZE_MAX; p += SIZE_STEP) {
      expect(snapToStep(p), String(p)).toBe(p);
    }
  });

  it("붙인_뒤에도_범위를_안_벗어난다", () => {
    for (let p = 0; p <= 300; p += 1) {
      const s = snapToStep(p);
      expect(s, String(p)).toBeGreaterThanOrEqual(SIZE_MIN);
      expect(s, String(p)).toBeLessThanOrEqual(SIZE_MAX);
      expect((s - SIZE_MIN) % SIZE_STEP, String(p)).toBe(0);
    }
  });

  it("테마가_없으면_시스템이다", async () => {
    mockStore({ pet: { enabled: true } });
    expect((await loadPetSettings()).theme).toBe("system");
  });

  it("저장된_테마를_그대로_읽는다", async () => {
    mockStore({ pet: { theme: "dark" } });
    expect((await loadPetSettings()).theme).toBe("dark");
  });

  it("깨진_테마는_시스템으로_수렴한다", async () => {
    for (const bad of ["어둡게", 2, null, true]) {
      mockStore({ pet: { theme: bad } });
      expect((await loadPetSettings()).theme, String(bad)).toBe("system");
    }
  });

  it("한_항목이_깨져도_나머지는_살린다", async () => {
    const data = mockStore();
    data.set("pet", { enabled: false, sound: "네" });
    const loaded = await loadPetSettings();
    expect(loaded).toEqual({
      enabled: false,
      sound: false,
      pinball: false,
      volume: 2,
      theme: "system",
      size: 100,
    });
  });
});

describe("크기 상수가 Rust와 같다", () => {
  // 한쪽만 고치면 슬라이더가 낼 수 있는 값과 Rust가 받아 주는 값이 갈린다 —
  // 두 러너·타입 검사가 전부 통과하면서. `pet-css.test.ts`의 여백 상수 대조와 같은 장치다.
  const scaleRs = readFileSync(resolve("src-tauri/src/pet_bridge/scale.rs"), "utf8");
  const rustConst = (name: string): number | null => {
    const m = scaleRs.match(new RegExp(`pub const ${name}: u32 = (\\d+)`));
    return m ? Number(m[1]) : null;
  };

  it.each([
    ["SIZE_MIN", SIZE_MIN],
    ["SIZE_MAX", SIZE_MAX],
    ["SIZE_STEP", SIZE_STEP],
  ])("%s 가 Rust와 같다", (name, ts) => {
    const rs = rustConst(name);
    expect(rs, `Rust에서 ${name}을 못 찾았다`).not.toBeNull();
    expect(ts).toBe(rs);
  });

  it("기본_크기가_Rust의_SIZE_DEFAULT와_같다", () => {
    expect(DEFAULT_PET_SETTINGS.size).toBe(rustConst("SIZE_DEFAULT"));
  });
});

describe("펭귄 대사 목록", () => {
  it("저장된_적이_없으면_기본_목록을_쓴다", async () => {
    mockStore();
    await expect(loadTaunts()).resolves.toEqual([...DEFAULT_TAUNTS]);
  });

  it("전부_지운_상태를_기본값으로_되살리지_않는다", async () => {
    mockStore({ taunts: [] });
    await expect(loadTaunts()).resolves.toEqual([]);
  });

  it("저장할_때_앞뒤_공백과_빈_줄을_정리한다", async () => {
    const data = mockStore();
    await saveTaunts(["  일 안 해요?  ", "", "   ", "아  진짜   왜요"]);
    expect(data.get("taunts")).toEqual(["일 안 해요?", "아 진짜 왜요"]);
  });

  it("너무_긴_줄은_잘라_말풍선을_넘기지_않는다", async () => {
    const data = mockStore();
    await saveTaunts(["가".repeat(120)]);
    const saved = data.get("taunts") as string[];
    expect(saved[0].length).toBeLessThanOrEqual(40);
  });

  it("문자열이_아닌_값이_섞여_있어도_버티고_거른다", async () => {
    mockStore({ taunts: ["멀쩡한 줄", 42, null, { a: 1 }] });
    await expect(loadTaunts()).resolves.toEqual(["멀쩡한 줄"]);
  });

  it("배열이_아니면_기본_목록으로_돌아간다", async () => {
    mockStore({ taunts: "망가진 값" });
    await expect(loadTaunts()).resolves.toEqual([...DEFAULT_TAUNTS]);
  });
});
