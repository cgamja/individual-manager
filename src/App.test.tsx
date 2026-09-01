import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  clearMocks();
});

/** 이벤트 구독이 붙을 수 있게 창 라벨과 내부 훅을 심는다.
 *
 * **설정 창도 창에 묶인 리스너를 쓴다** — 전역 `listen`은 대상을 `Any`로 등록해
 * `emit_to`와 무관하게 모든 창이 받는다. 라벨을 안 심으면 그 자리에서 터지는데,
 * 테스트는 통과한 것처럼 보이고 unhandled error로만 샌다. */
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
    if (cmd === "pet_fish" || cmd === "pet_slide" || cmd === "pet_squawk" || cmd === "pet_freakout") return null;
    // 저장소 플러그인 — 대사·설정을 읽는다
    if (cmd.startsWith("plugin:store|")) return null;
    return undefined;
  });
}

describe("설정 창", () => {
  // 카드를 하나 더 얹다가 앞의 카드를 밀어내도 아무것도 실패하지 않는다.
  // 실제로 "펭귄 추가/제거가 어디 갔냐"는 말을 들었던 자리라 목록으로 못박는다.
  it("카드가_전부_그려진다", async () => {
    mockSettings();
    render(<App />);
    for (const name of ["펭귄 추가", "이 펭귄 삭제", "얼음낚시", "슬라이딩", "빽빽거리기", "발작"]) {
      expect(await screen.findByRole("button", { name }), name).toBeInTheDocument();
    }
    expect(screen.getByLabelText("바탕화면 펭귄")).toBeInTheDocument();
  });

  it("다시_열면_맨_위로_되돌아온다", async () => {
    // 창은 닫을 때 파괴되지 않고 숨겨질 뿐이라 스크롤이 남는다 — 대사를 편집하러
    // 한 번 내려가면 그다음부터 맨 위 카드가 사라진 것처럼 보인다
    mockSettings();
    const scrollTo = vi.fn();
    Object.defineProperty(window, "scrollTo", { value: scrollTo, writable: true });
    render(<App />);
    await screen.findByRole("button", { name: "펭귄 추가" });

    document.dispatchEvent(new Event("visibilitychange"));
    expect(scrollTo).toHaveBeenCalledWith(0, 0);
  });

  it("다시_열면_펭귄_설정을_다시_읽는다", async () => {
    // **이 창 밖에서 바뀔 수 있다** — 핀볼 판에서 Esc를 누르면 저장소가 바뀐다.
    // 여기서 안 읽으면 설정 창은 켜진 것으로 보여, 체크를 껐다 켜야 실제로
    // 켜지는 꼴이 된다.
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

    // 판에서 Esc를 눌러 꺼진 상황
    store.set("pet", { enabled: true, sound: false, pinball: false });
    document.dispatchEvent(new Event("visibilitychange"));

    await waitFor(() => expect(screen.getByLabelText("핀볼 모드")).not.toBeChecked());
  });

  it("소리를_켜면_방송한다", async () => {
    // 이미 떠 있는 펭귄 전부에, 앱을 다시 띄우지 않고 반영돼야 한다 (R2)
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
      // get은 `[값, 존재 여부]` 튜플이어야 한다 — null을 주면 구조 분해에서 터져
      // 저장 자체가 실패로 빠진다
      if (cmd === "plugin:store|get") return [null, false];
      if (cmd.startsWith("plugin:store|")) return null;
      return undefined;
    });
    render(<App />);
    fireEvent.click(await screen.findByLabelText("효과음"));

    await waitFor(() => {
      const sound = events.find((e) => e.event === "pet://sound");
      // 음량도 항상 같이 실린다 — 절반이 undefined인 페이로드를 만들지 않는다
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
    // "저장은 실패했는데 소리는 켜진" 상태를 만들지 않는다 — 표시도 되돌린다
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

  it("우클릭_대상이_없으면_낚시와_삭제가_함께_잠긴다", async () => {
    mockSettings({ count: 2, max: 8, focused: null as unknown as number });
    render(<App />);
    expect(await screen.findByRole("button", { name: "얼음낚시" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "슬라이딩" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "빽빽거리기" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "발작" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "이 펭귄 삭제" })).toBeDisabled();
    // 추가는 대상이 필요 없다 — 같이 잠기면 안 된다
    expect(screen.getByRole("button", { name: "펭귄 추가" })).toBeEnabled();
  });
});
