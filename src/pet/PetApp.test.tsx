import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PetApp } from "./PetApp";

afterEach(() => {
  // 언마운트가 unlisten을 부르므로 mock을 걷기 전에 정리해야 한다
  cleanup();
  clearMocks();
});

interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

/** 펫 커맨드 IPC를 가로채 호출 순서를 기록한다. */
function mockPet(): Call[] {
  const calls: Call[] = [];
  // 언마운트 시 unlisten이 이 내부 훅을 찾는다 — 없으면 정리 단계에서 rejection이 샌다
  (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    // 이벤트 구독은 통과시킨다 (listen은 핸들러 id를 기대한다)
    if (cmd === "plugin:event|listen") return 1;
    if (cmd === "plugin:event|unlisten") return undefined;
    calls.push({ cmd, args: a });
    if (cmd === "pet_get_state") {
      return {
        x: 0,
        y: 0,
        facing: "right",
        vertical: "level",
        air: false,
        speech: null,
        whack_seq: 0,
        behavior: { kind: "walk" },
      };
    }
    return undefined;
  });
  return calls;
}

/** 화면 좌표를 실어 포인터 이벤트를 보낸다 — jsdom은 screenX를 기본 0으로 둔다. */
function pointer(
  type: string,
  target: Element,
  init: { screenX?: number; screenY?: number; pointerId?: number; button?: number } = {},
) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.assign(event, {
    pointerId: init.pointerId ?? 1,
    button: init.button ?? 0,
    screenX: init.screenX ?? 0,
    screenY: init.screenY ?? 0,
  });
  target.dispatchEvent(event);
  return event;
}

async function flush() {
  // 커맨드 왕복(Promise)들이 정산될 틈을 준다
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function penguin(): Element {
  const el = screen.getByRole("img", { name: "펭귄" });
  // jsdom에는 포인터 캡처 API가 없다
  Object.assign(el, {
    setPointerCapture: () => {},
    releasePointerCapture: () => {},
  });
  return el;
}

describe("펭귄 드래그", () => {
  it("포인터_이동_델타를_드래그_커맨드로_전달한다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
    pointer("pointermove", el, { screenX: 130, screenY: 90 });
    await flush();

    const drag = calls.find((c) => c.cmd === "pet_drag_by");
    expect(drag).toBeDefined();
    // 화면 좌표 기준의 증분이어야 한다 — 절대 좌표를 보내면 펭귄이 순간이동한다
    expect(drag?.args).toMatchObject({ dx: 30, dy: -10 });
  });

  it("클릭은_드래그가_아니라_빠따로_해석된다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
    // 임계값(4px) 미만으로만 흔들린 뒤 놓는다
    pointer("pointermove", el, { screenX: 101, screenY: 100 });
    pointer("pointerup", el, { screenX: 101, screenY: 100 });
    await flush();

    expect(calls.some((c) => c.cmd === "pet_whack")).toBe(true);
    expect(calls.some((c) => c.cmd === "pet_drag_end")).toBe(false);
  });

  it("충분히_움직이면_클릭이_아니라_드래그로_끝난다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
    pointer("pointermove", el, { screenX: 200, screenY: 100 });
    pointer("pointerup", el, { screenX: 200, screenY: 100 });
    await flush();

    expect(calls.some((c) => c.cmd === "pet_drag_end")).toBe(true);
    expect(calls.some((c) => c.cmd === "pet_whack")).toBe(false);
  });

  it("빠르게_튕겨_놓아도_이동량이_유실되지_않는다", async () => {
    // 시작 왕복이 끝나기 전에 움직이고 놓는 경우 — 이동량을 버리면
    // 펭귄이 제자리에서 떨어지기만 한다
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    pointer("pointermove", el, { screenX: 400, screenY: 100 });
    pointer("pointerup", el, { screenX: 400, screenY: 100 });
    await flush();

    const drag = calls.find((c) => c.cmd === "pet_drag_by");
    expect(drag?.args).toMatchObject({ dx: 300 });
    // 이동량이 놓기보다 먼저 도착해야 코어가 Dragged인 동안 반영된다
    const byIndex = calls.findIndex((c) => c.cmd === "pet_drag_by");
    const endIndex = calls.findIndex((c) => c.cmd === "pet_drag_end");
    expect(byIndex).toBeLessThan(endIndex);
  });

  it("우클릭은_빠따가_아니라_창_열기다", async () => {
    // 왼쪽은 빠따가 가져갔으므로 타이머·설정은 오른쪽 클릭으로 연다
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100, button: 2 });
    pointer("pointerup", el, { screenX: 100, screenY: 100, button: 2 });
    await flush();

    expect(calls.some((c) => c.cmd === "pet_open_popover")).toBe(true);
    expect(calls.some((c) => c.cmd === "pet_drag_start")).toBe(false);
    expect(calls.some((c) => c.cmd === "pet_whack")).toBe(false);
  });

  it("연달아_클릭하면_매번_빠따가_나간다", async () => {
    // 1클릭 1회 — 연타가 먹히지 않으면 저글링이 안 된다
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    for (let i = 0; i < 4; i++) {
      pointer("pointerdown", el, { screenX: 100, screenY: 100 });
      pointer("pointerup", el, { screenX: 100, screenY: 100 });
      await flush();
    }

    expect(calls.filter((c) => c.cmd === "pet_whack")).toHaveLength(4);
  });

  it("드래그_중_두번째_포인터는_기준점을_덮어쓰지_않는다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100, pointerId: 1 });
    await flush();
    // 다른 손가락/버튼이 끼어든다
    pointer("pointerdown", el, { screenX: 500, screenY: 500, pointerId: 2 });
    await flush();
    pointer("pointermove", el, { screenX: 110, screenY: 100, pointerId: 1 });
    await flush();

    const drags = calls.filter((c) => c.cmd === "pet_drag_by");
    // 원래 포인터 기준의 +10이어야 한다. 기준점이 덮였다면 -390이 된다
    expect(drags[drags.length - 1]?.args).toMatchObject({ dx: 10 });
    expect(calls.filter((c) => c.cmd === "pet_drag_start")).toHaveLength(1);
  });
});
