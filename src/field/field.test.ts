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
  // 모듈이 import 시점에 리스너를 건다 — 매번 새로 불러온다
  await import("./main");
});

afterEach(() => {
  clearMocks();
});

describe("핀볼 판", () => {
  it("Esc를_누르면_핀볼이_꺼지고_저장된다", async () => {
    // **나가는 문 둘 중 하나다** (다른 하나는 트레이 아이콘). 화면 전체의 클릭을
    // 먹는 기능이라 되돌리는 길이 하나뿐이면 안 된다.
    const calls = mockField();
    press("Escape");
    await flush();

    const off = calls.find((c) => c.cmd === "pet_set_pinball");
    expect(off?.args).toMatchObject({ on: false });
    // **저장도 해야 한다** — 안 하면 다음 실행에 다시 켜진 채로 뜬다
    const saved = calls.find((c) => c.cmd === "plugin:store|set");
    expect(saved?.args.value).toMatchObject({ pinball: false });

    // **저장이 커맨드보다 먼저다.** 뒤집으면 판 창이 닫히는 순간 설정 창이
    // 저장소를 다시 읽는데 저장이 아직 안 끝나 있어 체크가 도로 켜진다
    expect(calls.indexOf(saved!)).toBeLessThan(calls.indexOf(off!));
  });

  it("다른_키는_아무것도_안_한다", async () => {
    // 판이 키보드를 삼키면 "마우스만이 아니라 키보드도 죽었다"가 된다
    const calls = mockField();
    press("a");
    press("Enter");
    press("ArrowUp");
    await flush();

    expect(calls.some((c) => c.cmd === "pet_set_pinball")).toBe(false);
  });
});
