import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DRAG_THRESHOLD_PX } from "../lib/pet";

interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

let calls: Call[] = [];

/** 공 창이 부르는 IPC를 기록한다.
 *
 * `grabbed`는 코어의 대답이다 — **굴러가는 중이면 `false`가 온다.**
 * `delayMs`를 주면 그 답이 늦게 와서, 사용자가 이미 손을 뗀 뒤에 도착한다. */
function mockBall({ grabbed = true, delayMs = 0 } = {}): Call[] {
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
    if (cmd === "ball_drag_start") {
      return delayMs > 0
        ? new Promise((resolve) => setTimeout(() => resolve(grabbed), delayMs))
        : grabbed;
    }
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

/** 드래그 한 번. 집기 응답을 기다린 뒤 끌었다 놓는다. */
async function 끈다(dx: number, dy: number, steps = 4) {
  pointer("pointerdown", { screenX: 0, screenY: 0 });
  await flush();
  for (let i = 1; i <= steps; i += 1) {
    pointer("pointermove", { screenX: (dx * i) / steps, screenY: (dy * i) / steps });
  }
  pointer("pointerup", { screenX: dx, screenY: dy });
  await flush();
}

/** **집기 응답을 기다리지 않고** 끌었다 놓는다 — 빠르게 튕겼을 때의 순서다. */
async function 튕긴다(dx: number, dy: number, waitMs = 40) {
  pointer("pointerdown", { screenX: 0, screenY: 0 });
  pointer("pointermove", { screenX: dx / 2, screenY: dy / 2 });
  pointer("pointerup", { screenX: dx, screenY: dy });
  await new Promise((resolve) => setTimeout(resolve, waitMs));
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
    // 이 창은 `pet://sound`(전역)도 구독하므로 **이벤트를 지정해** 찾는다.
    const listen = calls.find(
      (c) => c.cmd === "plugin:event|listen" && c.args.event === "bowling://ball",
    );
    expect(listen, "공 상태를 구독하지 않았다").toBeDefined();
    const target = listen!.args.target as { kind?: string; label?: string };
    expect(target.kind).not.toBe("Any");
    expect(target.label).toBe("bowling-ball");
  });

  it("소리_설정은_전역으로_받는다", () => {
    // 소리 켜짐/꺼짐은 설정 창이 전 창에 방송하는 값이라 창에 묶으면 안 온다.
    const listen = calls.find(
      (c) => c.cmd === "plugin:event|listen" && c.args.event === "pet://sound",
    );
    expect(listen, "소리 설정을 구독하지 않았다").toBeDefined();
    expect((listen!.args.target as { kind?: string }).kind).toBe("Any");
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

  it("집기가_거절되면_놓기를_보내지_않는다", async () => {
    // 굴러가는 중에는 코어가 집기를 거절한다. 그런데도 놓기를 보내면
    // **굴러가던 공의 속도를 덮어써** 판이 그 자리에서 끝난다.
    clearMocks();
    calls = mockBall({ grabbed: false });
    await 끈다(200, 0);
    expect(calls.some((c) => c.cmd === "ball_drag_end")).toBe(false);
    expect(calls.some((c) => c.cmd === "ball_drag_by")).toBe(false);
  });

  it("집기_응답보다_놓기가_먼저_와도_거절을_지킨다", async () => {
    // 빠르게 튕기면 pointerup이 ball_drag_start의 왕복보다 먼저 도착한다.
    // 그 순간에는 아직 거절인지 모르므로, 결과를 끝까지 들고 가야 한다.
    clearMocks();
    calls = mockBall({ grabbed: false, delayMs: 10 });
    await 튕긴다(300, 0);
    expect(calls.some((c) => c.cmd === "ball_drag_end")).toBe(false);
  });

  it("응답이_늦어도_집었으면_굴린다", async () => {
    clearMocks();
    calls = mockBall({ grabbed: true, delayMs: 10 });
    await 튕긴다(300, 0);
    const end = calls.find((c) => c.cmd === "ball_drag_end");
    expect(end, "늦게 도착한 승낙은 정상으로 처리해야 한다").toBeDefined();
  });

  it("pointercancel도_놓기로_친다", async () => {
    // 취소를 놓기로 안 치면 공이 코어에서 들린 채로 남아 물리가 영영 안 돈다.
    pointer("pointerdown");
    await flush();
    pointer("pointermove", { screenX: 200 });
    pointer("pointercancel", { screenX: 200 });
    await flush();
    expect(calls.some((c) => c.cmd === "ball_drag_end")).toBe(true);
  });

  it("오른쪽_클릭은_아무것도_안_한다", async () => {
    pointer("pointerdown", { button: 2 });
    await flush();
    expect(calls.some((c) => c.cmd === "ball_drag_start")).toBe(false);
  });
});
