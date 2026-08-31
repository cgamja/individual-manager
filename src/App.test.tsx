import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  clearMocks();
});

/** 설정 창이 부르는 커맨드를 전부 가로챈다. */
function mockSettings(summary = { count: 1, max: 8, focused: 3 }) {
  mockIPC((cmd) => {
    if (cmd === "pet_summary") return summary;
    if (cmd === "pet_fish") return null;
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
    for (const name of ["펭귄 추가", "이 펭귄 삭제", "얼음낚시"]) {
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

  it("우클릭_대상이_없으면_낚시와_삭제가_함께_잠긴다", async () => {
    mockSettings({ count: 2, max: 8, focused: null as unknown as number });
    render(<App />);
    expect(await screen.findByRole("button", { name: "얼음낚시" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "이 펭귄 삭제" })).toBeDisabled();
    // 추가는 대상이 필요 없다 — 같이 잠기면 안 된다
    expect(screen.getByRole("button", { name: "펭귄 추가" })).toBeEnabled();
  });
});
