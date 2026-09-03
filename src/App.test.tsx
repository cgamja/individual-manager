import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  clearMocks();
});

/** 이벤트 구독이 붙을 수 있게 창 라벨과 내부 훅을 심는다. */
function mockWindow() {
  mockWindows("main");
  (window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: () => {},
  };
}

/** 설정 창이 부르는 커맨드를 전부 가로챈다. */
function mockSettings(summary = { count: 1, max: 8, focused: 3 }) {
  mockWindow();
  mockIPC((cmd) => {
    if (cmd === "plugin:event|listen") return 1;
    if (cmd === "plugin:event|unlisten") return undefined;
    if (cmd === "pet_summary") return summary;
    if (
      cmd === "pet_fish" ||
      cmd === "pet_slide" ||
      cmd === "pet_squawk" ||
      cmd === "pet_freakout" ||
      cmd === "pet_dont_ask"
    )
      return null;
    if (cmd.startsWith("plugin:store|")) return null;
    return undefined;
  });
}

describe("설정 창", () => {
  it("카드가_전부_그려진다", async () => {
    mockSettings();
    render(<App />);
    for (const name of [
      "펭귄 추가",
      "이 펭귄 삭제",
      "얼음낚시",
      "슬라이딩",
      "빽빽거리기",
      "발작",
      "안물",
    ]) {
      expect(await screen.findByRole("button", { name }), name).toBeInTheDocument();
    }
    expect(screen.getByLabelText("바탕화면 펭귄")).toBeInTheDocument();
  });

  it("다시_열면_맨_위로_되돌아온다", async () => {
    mockSettings();
    const scrollTo = vi.fn();
    Object.defineProperty(window, "scrollTo", { value: scrollTo, writable: true });
    render(<App />);
    await screen.findByRole("button", { name: "펭귄 추가" });

    document.dispatchEvent(new Event("visibilitychange"));
    expect(scrollTo).toHaveBeenCalledWith(0, 0);
  });

  it("다시_열면_펭귄_설정을_다시_읽는다", async () => {
    const store = new Map<string, unknown>([["pet", { enabled: true, sound: false, pinball: true }]]);
    mockWindow();
    mockIPC((cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 3 };
      if (cmd === "plugin:store|load") return 1;
      if (cmd === "plugin:store|get") {
        return store.has(a.key as string) ? [store.get(a.key as string), true] : [null, false];
      }
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    Object.defineProperty(window, "scrollTo", { value: vi.fn(), writable: true });
    render(<App />);
    expect(await screen.findByLabelText("핀볼 모드")).toBeChecked();

    store.set("pet", { enabled: true, sound: false, pinball: false });
    document.dispatchEvent(new Event("visibilitychange"));

    await waitFor(() => expect(screen.getByLabelText("핀볼 모드")).not.toBeChecked());
  });

  it("소리를_켜면_방송한다", async () => {
    const events: Array<Record<string, unknown>> = [];
    mockWindow();
    mockIPC((cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "plugin:event|emit") {
        events.push(a);
        return undefined;
      }
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 3 };
      if (cmd === "plugin:store|load") return 1;
      if (cmd === "plugin:store|get") return [null, false];
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    render(<App />);
    fireEvent.click(await screen.findByLabelText("효과음"));

    await waitFor(() => {
      const sound = events.find((e) => e.event === "pet://sound");
      expect(sound?.payload).toMatchObject({ sound: true, volume: 2 });
    });
  });

  it("음량을_바꾸면_저장하고_방송한다", async () => {
    const events: Array<Record<string, unknown>> = [];
    const store = new Map<string, unknown>();
    mockWindow();
    mockIPC((cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "plugin:event|emit") {
        events.push(a);
        return undefined;
      }
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 3 };
      if (cmd === "plugin:store|load") return 1;
      if (cmd === "plugin:store|get") {
        return store.has(a.key as string) ? [store.get(a.key as string), true] : [null, false];
      }
      if (cmd === "plugin:store|set") {
        store.set(a.key as string, a.value);
        return undefined;
      }
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    render(<App />);
    const slider = await screen.findByLabelText("음량");
    fireEvent.change(slider, { target: { value: "4" } });

    await waitFor(() => {
      expect(store.get("pet")).toMatchObject({ volume: 4 });
      const sound = events.find((e) => e.event === "pet://sound");
      expect(sound?.payload).toMatchObject({ sound: false, volume: 4 });
    });
  });

  it("저장에_실패하면_방송하지_않는다", async () => {
    const events: Array<Record<string, unknown>> = [];
    mockWindow();
    mockIPC((cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "plugin:event|emit") {
        events.push(a);
        return undefined;
      }
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 3 };
      if (cmd === "plugin:store|load") return 1;
      if (cmd === "plugin:store|get") return [null, false];
      if (cmd === "plugin:store|set") throw new Error("저장 실패");
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    render(<App />);
    const toggle = await screen.findByLabelText("효과음");
    fireEvent.click(toggle);

    await waitFor(() => expect(toggle).not.toBeChecked());
    expect(events.some((e) => e.event === "pet://sound")).toBe(false);
  });

  it("테마를_고르면_커맨드를_걸고_저장한다", async () => {
    const calls: Array<[string, Record<string, unknown>]> = [];
    const store = new Map<string, unknown>();
    mockWindow();
    mockIPC((cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 3 };
      if (cmd === "pet_set_theme") {
        calls.push([cmd, a]);
        return undefined;
      }
      if (cmd === "plugin:store|load") return 1;
      if (cmd === "plugin:store|get") {
        return store.has(a.key as string) ? [store.get(a.key as string), true] : [null, false];
      }
      if (cmd === "plugin:store|set") {
        store.set(a.key as string, a.value);
        return undefined;
      }
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    render(<App />);
    fireEvent.change(await screen.findByLabelText("테마"), { target: { value: "dark" } });

    await waitFor(() => {
      expect(calls).toContainEqual(["pet_set_theme", { theme: "dark" }]);
      expect(store.get("pet")).toMatchObject({ theme: "dark" });
    });
  });

  it("안물_버튼이_pet_dont_ask를_부른다", async () => {
    const calls: string[] = [];
    mockWindow();
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "pet_summary") return { count: 1, max: 8, focused: 0, bowling: false, volleyball: false };
      if (cmd.startsWith("plugin:store|")) return null;
      calls.push(cmd);
      return null;
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "안물" }));

    await waitFor(() => expect(calls).toContain("pet_dont_ask"));
  });

  it("우클릭_대상이_없으면_낚시와_삭제가_함께_잠긴다", async () => {
    mockSettings({ count: 2, max: 8, focused: null as unknown as number });
    render(<App />);
    expect(await screen.findByRole("button", { name: "얼음낚시" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "슬라이딩" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "빽빽거리기" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "발작" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "안물" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "이 펭귄 삭제" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "펭귄 추가" })).toBeEnabled();
  });
});

describe("판을 못 열면 이유를 보여 준다", () => {
  // **버튼이 눌렸는데 아무 일도 안 일어나면 고장으로 읽힌다.** 코어가 이유를
  // 셋으로 갈라 주는데(두 마리부터 / 이미 판이 돈다 / 코트를 깔 자리가 없다)
  // 콘솔에만 찍으면 사용자에게는 "안 눌린다"로 보인다 (PRD §5.10).
  function mockBoard(거절: () => boolean) {
    mockWindow();
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten") return undefined;
      if (cmd === "pet_summary")
        return { count: 1, max: 8, focused: 3, bowling: false, volleyball: false };
      if (cmd === "volleyball_start") {
        if (거절()) throw "두 마리부터 할 수 있어요";
        return null;
      }
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
  }

  it("비치발리볼_거절_사유가_화면에_뜬다", async () => {
    mockBoard(() => true);
    render(<App />);
    const 버튼 = await screen.findByRole("button", { name: "비치발리볼 한 판" });
    fireEvent.click(버튼);
    expect(await screen.findByText("두 마리부터 할 수 있어요")).toBeInTheDocument();
  });

  it("성공하면_사유가_사라진다", async () => {
    let 거절 = true;
    mockBoard(() => 거절);
    render(<App />);
    const 버튼 = await screen.findByRole("button", { name: "비치발리볼 한 판" });
    fireEvent.click(버튼);
    expect(await screen.findByText("두 마리부터 할 수 있어요")).toBeInTheDocument();
    거절 = false;
    fireEvent.click(버튼);
    await waitFor(() =>
      expect(screen.queryByText("두 마리부터 할 수 있어요")).not.toBeInTheDocument(),
    );
  });
});
