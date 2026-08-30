import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// 팝오버 자체는 Tauri 커맨드 위에 있다. 여기서 보고 싶은 건 "무엇이 최상위이고
// 무엇이 어디에 들어갔는가"(KTD1)이므로 래퍼 계층에서 자른다.
const openInBrowser = vi.fn().mockResolvedValue(undefined);

vi.mock("./lib/launcher", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/launcher")>()),
  openInBrowser: (url: string) => openInBrowser(url),
}));

const hide = vi.fn().mockResolvedValue(undefined);
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ hide }) }));

const notifGranted = { value: true };

vi.mock("./lib/notification", () => ({
  ensureNotificationPermission: () => Promise.resolve(notifGranted.value),
}));

vi.mock("./lib/pet", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/pet")>()),
  setPetEnabled: () => Promise.resolve(),
}));

vi.mock("./lib/timer", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/timer")>()),
  getTimerState: () => Promise.resolve({ state: "idle" }),
  setTimerConfig: (c: unknown) => Promise.resolve(c),
  onTick: () => Promise.resolve(() => {}),
  startTimer: () => Promise.resolve({ state: "idle" }),
  pauseTimer: () => Promise.resolve({ state: "idle" }),
  resumeTimer: () => Promise.resolve({ state: "idle" }),
  resetTimer: () => Promise.resolve({ state: "idle" }),
}));

vi.mock("./lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./lib/settings")>();
  return {
    ...actual,
    loadSettings: () => Promise.resolve(actual.DEFAULT_SETTINGS),
    saveSettings: () => Promise.resolve(),
    loadPetSettings: () => Promise.resolve(actual.DEFAULT_PET_SETTINGS),
    savePetSettings: () => Promise.resolve(),
    loadTaunts: () => Promise.resolve(["일 안 해요?"]),
    saveTaunts: () => Promise.resolve(),
    loadLauncher: async () => [...(await import("./lib/launcher")).DEFAULT_LAUNCHER],
  };
});

const { default: App } = await import("./App");

beforeEach(() => {
  openInBrowser.mockClear();
  hide.mockClear();
  notifGranted.value = true;
});
afterEach(cleanup);

describe("팝오버 최상위", () => {
  it("팝오버는_런처를_최상위로_그린다", async () => {
    render(<App />);
    expect(await screen.findByRole("button", { name: "NOTION" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "POMODORO" })).toBeTruthy();
  });

  it("타이머는_처음에_보이지_않고_POMODORO를_펼쳐야_나온다", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "POMODORO" });
    expect(screen.queryByRole("button", { name: "집중 시작" })).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: "POMODORO" }));
    expect(screen.getByRole("button", { name: "집중 시작" })).toBeTruthy();
  });

  it("설정_버튼을_눌러야_펭귄_설정과_대사_편집이_보인다", async () => {
    render(<App />);
    const gear = await screen.findByRole("button", { name: "설정" });
    expect(screen.queryByLabelText("새 대사")).toBeNull();

    await userEvent.click(gear);
    expect(screen.getByLabelText("새 대사")).toBeTruthy();
  });

  it("서비스_항목을_누르면_브라우저로_넘긴다", async () => {
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "GOOGLE CALENDAR" }));
    await userEvent.click(screen.getByRole("button", { name: /주간 뷰/ }));

    expect(openInBrowser).toHaveBeenCalledWith(
      "https://calendar.google.com/calendar/r/week",
    );
  });
});

describe("Esc — 한 번에 한 단계씩 물러난다 (R6)", () => {
  it("펼쳐진_카드가_있으면_그것만_접고_팝오버는_그대로_있는다", async () => {
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "NOTION" }));
    await userEvent.keyboard("{Escape}");

    expect(screen.getByRole("button", { name: "NOTION" }).getAttribute("aria-expanded")).toBe(
      "false",
    );
    expect(hide).not.toHaveBeenCalled();
  });

  it("설정이_열려_있으면_그것을_접는다", async () => {
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "설정" }));
    await userEvent.keyboard("{Escape}");

    expect(screen.queryByLabelText("새 대사")).toBeNull();
    expect(hide).not.toHaveBeenCalled();
  });

  it("전부_접혀_있으면_팝오버를_닫는다", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "NOTION" });
    await userEvent.keyboard("{Escape}");

    expect(hide).toHaveBeenCalledTimes(1);
  });
});

describe("기존 동작 회귀", () => {
  it("알림_권한이_없으면_안내_문구가_그대로_보인다", async () => {
    notifGranted.value = false;
    render(<App />);
    expect(await screen.findByRole("status")).toHaveTextContent("알림 권한이 꺼져 있어요");
  });
});
