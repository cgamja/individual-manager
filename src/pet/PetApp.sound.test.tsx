import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PetSnapshot } from "../lib/pet";

/** 소리 배선 테스트 — `PetApp.test.tsx`와 달리 IPC가 아니라 **모듈을 모킹한다. */

const h = vi.hoisted(() => ({
  players: [] as Array<{
    label: string;
    enabled: boolean | null;
    volume: number | null;
    played: string[];
    nudged: number;
    closed: number;
  }>,
  stateCb: null as ((s: PetSnapshot) => void) | null,
  soundCb: null as ((p: { sound: boolean; volume: number }) => void) | null,
  soundsForSpy: vi.fn(),
  savedSound: false,
  savedVolume: 2,
}));

vi.mock("./sound", async (importOriginal) => {
  const real = await importOriginal<typeof import("./sound")>();
  h.soundsForSpy.mockImplementation(real.soundsFor);
  return {
    ...real,
    soundsFor: h.soundsForSpy,
    SoundPlayer: class {
      label: string;
      enabled: boolean | null = null;
      volume: number | null = null;
      played: string[] = [];
      nudged = 0;
      closed = 0;
      constructor(label: string) {
        this.label = label;
        h.players.push(this);
      }
      setEnabled(on: boolean) {
        this.enabled = on;
      }
      setVolume(step: number) {
        this.volume = step;
      }
      nudge() {
        this.nudged += 1;
      }
      play(name: string) {
        this.played.push(name);
      }
      close() {
        this.closed += 1;
      }
    },
  };
});

vi.mock("../lib/settings", () => ({
  loadTaunts: () => Promise.resolve(["안녕"]),
  loadPetSettings: () =>
    Promise.resolve({ enabled: true, sound: h.savedSound, pinball: false, volume: h.savedVolume }),
}));

vi.mock("../lib/pet", async (importOriginal) => {
  const real = await importOriginal<typeof import("../lib/pet")>();
  return {
    ...real,
    getPetState: () => Promise.resolve(null),
    startPetDrag: () => Promise.resolve(),
    onPetState: (cb: (s: PetSnapshot) => void) => {
      h.stateCb = cb;
      return Promise.resolve(() => {});
    },
    onPetSound: (cb: (p: { sound: boolean; volume: number }) => void) => {
      h.soundCb = cb;
      return Promise.resolve(() => {});
    },
  };
});

import { PetApp } from "./PetApp";

const snap = (over: Partial<PetSnapshot> = {}): PetSnapshot => ({
  x: 0,
  y: 0,
  facing: "right",
  vertical: "level",
  air: false,
  speech: null,
  whack_seq: 0,
  pinball: false,
  behavior: { kind: "walk" },
  ...over,
});

async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  mockWindows("pet-1");
  h.players.length = 0;
  h.stateCb = null;
  h.soundCb = null;
  h.savedSound = false;
  h.savedVolume = 2;
  h.soundsForSpy.mockClear();
});

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("PetApp 소리 배선", () => {
  it("소리가_꺼져_있으면_재생을_시도하지_않는다", async () => {
    render(<PetApp />);
    await flush();
    const player = h.players[0];
    expect(player.enabled).toBe(false);
    h.stateCb?.(snap({ whack_seq: 1 }));
    expect(player.enabled).toBe(false);
  });

  it("설정_이벤트가_오면_즉시_반영된다", async () => {
    render(<PetApp />);
    await flush();
    h.soundCb?.({ sound: true, volume: 2 });
    expect(h.players[0].enabled).toBe(true);
    h.soundCb?.({ sound: false, volume: 2 });
    expect(h.players[0].enabled).toBe(false);
  });

  it("음량_방송이_즉시_반영된다", async () => {
    render(<PetApp />);
    await flush();
    h.soundCb?.({ sound: true, volume: 4 });
    expect(h.players[0].volume).toBe(4);
  });

  it("시작_음량은_저장소에서_건다", async () => {
    h.savedVolume = 3;
    render(<PetApp />);
    await flush();
    expect(h.players[0].volume).toBe(3);
  });

  it("스냅샷_전이마다_판정을_한_번씩_한다", async () => {
    render(<PetApp />);
    await flush();
    const s1 = snap({ whack_seq: 1 });
    const s2 = snap({ whack_seq: 1, behavior: { kind: "thrown" }, air: true });
    const s3 = snap({ whack_seq: 1, behavior: { kind: "land" } });
    h.stateCb?.(s1);
    h.stateCb?.(s2);
    h.stateCb?.(s3);
    expect(h.soundsForSpy.mock.calls).toEqual([
      [null, s1],
      [s1, s2],
      [s2, s3],
    ]);
  });

  it("판정_결과를_재생한다", async () => {
    render(<PetApp />);
    await flush();
    h.stateCb?.(snap({ whack_seq: 0 }));
    h.stateCb?.(snap({ whack_seq: 1 }));
    expect(h.players[0].played).toEqual(["whack"]);
  });

  it("누르면_오디오를_깨운다", async () => {
    render(<PetApp />);
    await flush();
    const el = screen.getByRole("img", { name: "펭귄" });
    Object.assign(el, { setPointerCapture: () => {}, releasePointerCapture: () => {} });
    const down = new Event("pointerdown", { bubbles: true });
    Object.assign(down, { pointerId: 1, button: 0, screenX: 0, screenY: 0 });
    el.dispatchEvent(down);
    expect(h.players[0].nudged).toBe(1);
    const right = new Event("pointerdown", { bubbles: true });
    Object.assign(right, { pointerId: 1, button: 2, screenX: 0, screenY: 0 });
    el.dispatchEvent(right);
    expect(h.players[0].nudged).toBe(2);
  });

  it("자기_창의_라벨로_목소리를_만든다", async () => {
    render(<PetApp />);
    await flush();
    expect(h.players[0].label).toBe("pet-1");
  });

  it("언마운트하면_컨텍스트를_닫는다", async () => {
    const { unmount } = render(<PetApp />);
    await flush();
    unmount();
    expect(h.players[0].closed).toBe(1);
  });

  it("핀볼_클릭은_낙관적으로_휙을_재생한다", async () => {
    render(<PetApp />);
    await flush();
    h.stateCb?.(snap({ pinball: true }));
    await flush();
    const el = screen.getByRole("img", { name: "펭귄" });
    Object.assign(el, { setPointerCapture: () => {}, releasePointerCapture: () => {} });
    const down = new Event("pointerdown", { bubbles: true });
    Object.assign(down, { pointerId: 1, button: 0, screenX: 0, screenY: 0 });
    el.dispatchEvent(down);
    const up = new Event("pointerup", { bubbles: true });
    Object.assign(up, { pointerId: 1, button: 0, screenX: 0, screenY: 0 });
    el.dispatchEvent(up);
    await flush();
    expect(h.players[0].played).toContain("whoosh");
  });

  it("핀볼이_아니면_클릭으로_휙이_안_난다", async () => {
    render(<PetApp />);
    await flush();
    h.stateCb?.(snap({ pinball: false }));
    const el = screen.getByRole("img", { name: "펭귄" });
    Object.assign(el, { setPointerCapture: () => {}, releasePointerCapture: () => {} });
    const down = new Event("pointerdown", { bubbles: true });
    Object.assign(down, { pointerId: 1, button: 0, screenX: 0, screenY: 0 });
    el.dispatchEvent(down);
    const up = new Event("pointerup", { bubbles: true });
    Object.assign(up, { pointerId: 1, button: 0, screenX: 0, screenY: 0 });
    el.dispatchEvent(up);
    await flush();
    expect(h.players[0].played).not.toContain("whoosh");
  });
});

describe("안물 소리와 춤의 분리", () => {
  it("안물_스냅샷이_오면_dont_ask를_재생한다", async () => {
    render(<PetApp />);
    await flush();
    h.soundCb?.({ sound: true, volume: 2 });
    h.stateCb?.(snap());
    h.stateCb?.(snap({ behavior: { kind: "dont_ask" } }));
    expect(h.players[0].played).toContain("dont_ask");
  });

  it("효과음이_꺼져도_춤과_말풍선은_나온다", async () => {
    // 소리와 동작을 묶지 않는다 — 소리 스위치가 모션 on/off가 되면 안 된다.
    render(<PetApp />);
    await flush();
    expect(h.players[0].enabled).toBe(false);
    h.stateCb?.(snap({ behavior: { kind: "dont_ask" } }));
    await flush();
    expect(screen.getByText("묻지 않았습니다~~")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "펭귄" }).getAttribute("class")).toContain(
      "pg--dont-ask",
    );
  });
});
