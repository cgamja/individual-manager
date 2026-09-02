import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

/** 판이 부르는 IPC를 기록한다 — 커맨드와 저장소 둘 다 흉내낸다. */
function mockField(): Call[] {
  const calls: Call[] = [];
  const data = new Map<string, unknown>();
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "plugin:store|load":
        return 1;
      case "plugin:store|get":
        return data.has(a.key as string) ? [data.get(a.key as string), true] : [null, false];
      case "plugin:store|set":
        data.set(a.key as string, a.value);
        return undefined;
      default:
        return undefined;
    }
  });
  return calls;
}

async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function press(key: string) {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

beforeEach(async () => {
  await import("./main");
});

afterEach(() => {
  clearMocks();
});

describe("핀볼 판", () => {
  it("Esc를_누르면_핀볼이_꺼지고_저장된다", async () => {
    const calls = mockField();
    press("Escape");
    await flush();

    const off = calls.find((c) => c.cmd === "pet_set_pinball");
    expect(off?.args).toMatchObject({ on: false });
    const saved = calls.find((c) => c.cmd === "plugin:store|set");
    expect(saved?.args.value).toMatchObject({ pinball: false });

    expect(calls.indexOf(saved!)).toBeLessThan(calls.indexOf(off!));
  });

  it("다른_키는_아무것도_안_한다", async () => {
    const calls = mockField();
    press("a");
    press("Enter");
    press("ArrowUp");
    await flush();

    expect(calls.some((c) => c.cmd === "pet_set_pinball")).toBe(false);
  });
});
