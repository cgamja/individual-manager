import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PetApp } from "./PetApp";

afterEach(() => {
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
  mockWindows("pet-1");
  (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
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
  init: {
    screenX?: number;
    screenY?: number;
    clientX?: number;
    clientY?: number;
    pointerId?: number;
    button?: number;
  } = {},
) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.assign(event, {
    pointerId: init.pointerId ?? 1,
    button: init.button ?? 0,
    screenX: init.screenX ?? 0,
    screenY: init.screenY ?? 0,
    clientX: init.clientX ?? 0,
    clientY: init.clientY ?? 0,
  });
  target.dispatchEvent(event);
  return event;
}

async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function penguin(): Element {
  const el = screen.getByRole("img", { name: "펭귄" });
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
    expect(drag?.args).toMatchObject({ dx: 30, dy: -10 });
  });

  it("클릭은_맞은_지점을_같이_보낸다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();
    el.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 200, height: 200 }) as DOMRect;

    pointer("pointerdown", el, { screenX: 100, screenY: 100, clientX: 50, clientY: 40 });
    await flush();
    pointer("pointerup", el, { screenX: 100, screenY: 100, clientX: 50, clientY: 40 });
    await flush();

    const whack = calls.find((c) => c.cmd === "pet_whack");
    expect(whack?.args).toMatchObject({ nx: -0.25, ny: -0.3 });
  });

  it("펭귄_크기를_못_재면_정중앙으로_친_것으로_본다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100, clientX: 50, clientY: 40 });
    await flush();
    pointer("pointerup", el, { screenX: 100, screenY: 100, clientX: 50, clientY: 40 });
    await flush();

    expect(calls.find((c) => c.cmd === "pet_whack")?.args).toMatchObject({ nx: 0, ny: 0 });
  });

  it("클릭은_드래그가_아니라_빠따로_해석된다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
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
    const byIndex = calls.findIndex((c) => c.cmd === "pet_drag_by");
    const endIndex = calls.findIndex((c) => c.cmd === "pet_drag_end");
    expect(byIndex).toBeLessThan(endIndex);
  });

  it("우클릭은_빠따가_아니라_창_열기다", async () => {
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
    pointer("pointerdown", el, { screenX: 500, screenY: 500, pointerId: 2 });
    await flush();
    pointer("pointermove", el, { screenX: 110, screenY: 100, pointerId: 1 });
    await flush();

    const drags = calls.filter((c) => c.cmd === "pet_drag_by");
    expect(drags[drags.length - 1]?.args).toMatchObject({ dx: 10 });
    expect(calls.filter((c) => c.cmd === "pet_drag_start")).toHaveLength(1);
  });
});

describe("시선 추적", () => {
  /** 무대의 자리를 고정한다 — jsdom은 레이아웃을 안 재서 전부 0으로 나온다. */
  function stage(): HTMLElement {
    const el = document.querySelector(".pg-stage") as HTMLElement;
    el.getBoundingClientRect = () =>
      ({ left: 52, top: 80, width: 140, height: 140 }) as DOMRect;
    return el;
  }

  function gazeX(): string {
    return (document.querySelector(".penguin") as HTMLElement).style.getPropertyValue(
      "--gaze-x",
    );
  }

  it("실루엣_밖에서_움직여도_눈동자가_따라온다", async () => {
    // 실루엣만 포인터를 받게 되면서(base.css) SVG에 리스너를 두면 눈동자가
    // 펭귄 몸 위에서만 움직인다. 창 전역이어야 한다. client 80은 무대 안이지만
    // 레터박스라 그림이 시작하지도 않은 자리다.
    mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 80, clientY: 150 });
    await flush();

    expect(gazeX()).not.toBe("0px");
  });

  it("드래그_중에는_눈동자가_안_움직인다", async () => {
    mockPet();
    render(<PetApp />);
    await flush();
    stage();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
    const before = gazeX();
    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();

    expect(gazeX()).toBe(before);
  });

  it("통과가_걸리면_눈동자가_가운데로_돌아온다", async () => {
    // 통과가 걸리는 순간 이 창은 포인터 이벤트를 아예 못 받는다 —
    // `pointerleave`조차 안 온다. 그때 시선을 안 되돌리면 눈동자가 마지막
    // 표본(대개 ±1.6 최대치)에 얼어붙어 계속 한쪽을 노려본다 (R7).
    mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 100, clientY: 150 });
    await flush();
    expect(gazeX()).not.toBe("0px");

    pointer("pointermove", window as unknown as Element, { clientX: 4, clientY: 150 });
    await flush();

    expect(gazeX()).toBe("0px");
  });

  it("창을_벗어나면_눈동자가_가운데로_돌아온다", async () => {
    mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 100, clientY: 150 });
    await flush();
    expect(gazeX()).not.toBe("0px");

    document.documentElement.dispatchEvent(new Event("pointerleave"));
    await flush();

    expect(gazeX()).toBe("0px");
  });
});

describe("클릭 통과 요청", () => {
  /** 무대의 자리를 고정한다 — 창 244×220 안의 140×140 무대다. */
  function stage(): void {
    const el = document.querySelector(".pg-stage") as HTMLElement;
    el.getBoundingClientRect = () =>
      ({ left: 52, top: 80, width: 140, height: 140 }) as DOMRect;
  }

  function 요청들(calls: Call[]): unknown[] {
    return calls.filter((c) => c.cmd === "pet_set_click_through").map((c) => c.args.on);
  }

  it("여백으로_나가면_통과를_요청한다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();

    // 창 왼쪽 끝 — 방망이 여백이다.
    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();

    expect(요청들(calls)).toEqual([true]);
  });

  it("거두는_것은_보내지_않는다", async () => {
    // 되돌리는 것은 Rust 몫이다 — 웹뷰는 요청만 한다. `false`를 보내면
    // 되돌리는 주체가 둘이 되고, 통과 중에는 어차피 못 보낸다.
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();
    pointer("pointermove", window as unknown as Element, { clientX: 122, clientY: 150 });
    await flush();

    expect(요청들(calls)).toEqual([true]);
  });

  it("여백을_헤매도_요청이_쌓이지_않는다", async () => {
    // 창이 아직 안 바뀐 채 커서가 여백에서 움직이면 매 이동마다 IPC가 나간다.
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();

    for (const x of [10, 12, 14, 16]) {
      pointer("pointermove", window as unknown as Element, { clientX: x, clientY: 150 });
      await flush();
    }

    expect(요청들(calls)).toEqual([true]);
  });

  it("펭귄_위를_지났다_다시_여백으로_가면_또_요청한다", async () => {
    // **Rust는 되돌릴 때 요청을 지운다(걸쇠).** 웹뷰가 "이미 보냈다"를 믿으면
    // 그 뒤로 다시는 통과가 안 걸린다 — 관측으로 다시 판단해야 한다.
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();
    pointer("pointermove", window as unknown as Element, { clientX: 122, clientY: 150 });
    await flush();
    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();

    expect(요청들(calls)).toEqual([true, true]);
  });

  it("펭귄_위에서는_아무것도_안_보낸다", async () => {
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();

    pointer("pointermove", window as unknown as Element, { clientX: 122, clientY: 150 });
    await flush();

    expect(요청들(calls)).toEqual([]);
  });

  it("드래그_중에는_통과를_요청하지_않는다", async () => {
    // 들고 있는 동안 커서는 창 어디로든 간다. 통과가 걸리면 드래그가 끊긴다.
    const calls = mockPet();
    render(<PetApp />);
    await flush();
    stage();
    const el = penguin();

    pointer("pointerdown", el, { screenX: 100, screenY: 100 });
    await flush();
    pointer("pointermove", window as unknown as Element, { clientX: 10, clientY: 150 });
    await flush();

    expect(요청들(calls)).toEqual([]);
  });
});
