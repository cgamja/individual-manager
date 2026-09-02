import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DRAG_THRESHOLD_PX } from "../lib/pet";

interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

let calls: Call[] = [];

/** 공 창이 부르는 IPC를 기록한다. `ball_drag_start`는 "집었다"로 답한다. */
function mockBall(grabbed = true): Call[] {
  const recorded: Call[] = [];
  mockWindows("bowling-ball");
  (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    recorded.push({ cmd, args: a });
    if (cmd === "plugin:event|listen") return 1;
    if (cmd === "plugin:event|unlisten") return undefined;
    if (cmd === "ball_drag_start") return grabbed;
    return undefined;
  });
  return recorded;
}

/** 화면 좌표를 실어 포인터 이벤트를 보낸다 — jsdom은 screenX를 기본 0으로 둔다. */
function pointer(
  type: string,
  init: { screenX?: number; screenY?: number; pointerId?: number; button?: number } = {},
) {
  const root = document.getElementById("ball-root")!;
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.assign(event, {
    pointerId: init.pointerId ?? 1,
    button: init.button ?? 0,
    screenX: init.screenX ?? 0,
    screenY: init.screenY ?? 0,
  });
  root.dispatchEvent(event);
}

async function flush() {
  for (let i = 0; i < 4; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

/** 드래그 한 번. `dt`ms 동안 (dx, dy)만큼 끌었다 놓는다. */
async function 끈다(dx: number, dy: number, steps = 4) {
  pointer("pointerdown", { screenX: 0, screenY: 0 });
  await flush();
  for (let i = 1; i <= steps; i += 1) {
    pointer("pointermove", { screenX: (dx * i) / steps, screenY: (dy * i) / steps });
  }
  pointer("pointerup", { screenX: dx, screenY: dy });
  await flush();
}

beforeEach(async () => {
  // `main.ts`는 import 시점에 `#ball-root`를 잡아 리스너를 건다. 모듈 캐시를
  // 비우지 않으면 두 번째 테스트부터는 이미 버려진 노드에 붙은 채로 돈다.
  vi.resetModules();
  document.body.innerHTML = '<div id="ball-root"></div>';
  calls = mockBall();
  await import("./main");
});

afterEach(() => {
  clearMocks();
  document.body.innerHTML = "";
});

describe("볼링 공 창", () => {
  it("공_상태_구독이_창에_묶여_있다", () => {
    // 전역 `listen()`은 대상을 `Any`로 등록해 emit 대상과 무관하게 전부
    // 호출된다. 창이 하나일 때는 안 드러나다가 여러 창에서 터진다.
    const listen = calls.find((c) => c.cmd === "plugin:event|listen");
    expect(listen, "공 상태를 구독하지 않았다").toBeDefined();
    expect(listen!.args.event).toBe("bowling://ball");
    const target = listen!.args.target as { kind?: string; label?: string };
    expect(target.kind).not.toBe("Any");
    expect(target.label).toBe("bowling-ball");
  });

  it("집을_때_코어에_먼저_알린다", async () => {
    pointer("pointerdown");
    await flush();
    expect(calls.some((c) => c.cmd === "ball_drag_start")).toBe(true);
  });

  it("놓는_순간의_가로_속도만_전달한다", async () => {
    await 끈다(200, 150);
    const end = calls.find((c) => c.cmd === "ball_drag_end");
    expect(end, "놓기를 안 알렸다").toBeDefined();
    expect(Object.keys(end!.args)).toEqual(["vx"]);
    expect(end!.args.vx as number).toBeGreaterThan(0);
  });

  it("세로로만_그으면_굴러가지_않는다", async () => {
    await 끈다(0, 300);
    const end = calls.find((c) => c.cmd === "ball_drag_end");
    expect(end!.args.vx).toBe(0);
  });

  it("문턱보다_짧게_움직이면_굴리지_않는다", async () => {
    await 끈다(DRAG_THRESHOLD_PX - 2, 0, 1);
    const end = calls.find((c) => c.cmd === "ball_drag_end");
    expect(end, "놓기 자체는 알려야 한다 — 안 그러면 공이 들린 채로 남는다").toBeDefined();
    expect(end!.args.vx).toBe(0);
  });

  it("가로_이동량만_보낸다", async () => {
    await 끈다(120, 90);
    const moves = calls.filter((c) => c.cmd === "ball_drag_by");
    expect(moves.length).toBeGreaterThan(0);
    for (const move of moves) {
      expect(Object.keys(move.args)).toEqual(["dx"]);
    }
  });

  it("오른쪽_클릭은_아무것도_안_한다", async () => {
    pointer("pointerdown", { button: 2 });
    await flush();
    expect(calls.some((c) => c.cmd === "ball_drag_start")).toBe(false);
  });
});
