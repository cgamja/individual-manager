import { clearMocks, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PetSnapshot } from "../lib/pet";

/**
 * 소리 배선 테스트 — `PetApp.test.tsx`와 달리 IPC가 아니라 **모듈을 모킹한다.**
 * 스냅샷 흐름과 SoundPlayer 호출을 직접 쥐어야 "전이마다 판정 한 번"을 셀 수
 * 있는데, `vi.mock`은 파일 전체에 걸리므로 실제 IPC를 쓰는 기존 파일과 섞지
 * 않고 따로 둔다.
 */

const h = vi.hoisted(() => ({
  players: [] as Array<{
    label: string;
    enabled: boolean | null;
    played: string[];
    nudged: number;
    closed: number;
  }>,
  stateCb: null as ((s: PetSnapshot) => void) | null,
  soundCb: null as ((on: boolean) => void) | null,
  soundsForSpy: vi.fn(),
  savedSound: false,
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
    Promise.resolve({ enabled: true, sound: h.savedSound, pinball: false }),
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
    onPetSound: (cb: (on: boolean) => void) => {
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
    // 저장된 설정(꺼짐)이 초기값으로 걸린다
    expect(player.enabled).toBe(false);
    // 퍽이 날 전이를 흘려도, 판정은 하되 재생 게이트는 player가 쥔다 —
    // 꺼짐은 SoundPlayer 안에서 걸러진다 (sound.test.ts가 증명한다)
    h.stateCb?.(snap({ whack_seq: 1 }));
    expect(player.enabled).toBe(false);
  });

  it("설정_이벤트가_오면_즉시_반영된다", async () => {
    render(<PetApp />);
    await flush();
    // 설정 창의 방송이 앱 재시작 없이 걸린다 (R2)
    h.soundCb?.(true);
    expect(h.players[0].enabled).toBe(true);
    h.soundCb?.(false);
    expect(h.players[0].enabled).toBe(false);
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
    // 직전 값과 짝지어 세 번 — React 렌더 배칭에 얹으면 중간 스냅샷이 스킵된다
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
    // 우클릭도 제스처다 — 깨우기는 버튼 분기보다 앞이다 (KTD4)
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
    // 핀볼에서 공중 재타격은 Thrown→Thrown이라 전이 검출이 못 본다 (리뷰 #1).
    // 클릭이 곧 타격이므로 스냅샷을 기다리지 않고 재생한다
    render(<PetApp />);
    await flush();
    h.stateCb?.(snap({ pinball: true }));
    // 스냅샷이 effect(pinballRef 갱신)까지 반영된 뒤에 클릭해야 한다 —
    // 실제 앱에서도 클릭은 언제나 스냅샷 뒤에 온다
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
    // 평소의 빠따 소리는 whack_seq 스냅샷이 낸다 — 클릭 자체는 조용하다
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
