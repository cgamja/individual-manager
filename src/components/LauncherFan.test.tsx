import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LauncherFan } from "./LauncherFan";
import { DEFAULT_LAUNCHER } from "../lib/launcher";

afterEach(cleanup);

function renderFan(overrides: Partial<Parameters<typeof LauncherFan>[0]> = {}) {
  const onOpen = vi.fn().mockResolvedValue(undefined);
  render(
    <LauncherFan
      services={DEFAULT_LAUNCHER}
      onOpen={onOpen}
      timerPanel={<p>타이머 자리</p>}
      {...overrides}
    />,
  );
  return { onOpen };
}

const card = (name: string) => screen.getByRole("button", { name });

describe("1단 — 팬", () => {
  it("4장의_카드가_지정된_순서로_보인다", () => {
    renderFan();
    const labels = screen
      .getAllByRole("button")
      .map((b) => b.textContent)
      .filter((t): t is string => Boolean(t));
    expect(labels.slice(0, 4)).toEqual(["NOTION", "JIRA", "GOOGLE CALENDAR", "POMODORO"]);
  });

  it("처음에는_아무_카드도_펼쳐져_있지_않다", () => {
    renderFan();
    for (const service of DEFAULT_LAUNCHER) {
      expect(card(service.label).getAttribute("aria-expanded")).toBe("false");
    }
  });
});

describe("2단 — 토글", () => {
  it("카드를_누르면_그_하위_항목이_펼쳐진다", async () => {
    renderFan();
    await userEvent.click(card("NOTION"));

    expect(card("NOTION").getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("button", { name: /TODO 보드/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /오늘 페이지/ })).toBeTruthy();
  });

  it("다른_카드를_누르면_앞서_펼친_카드는_접힌다", async () => {
    renderFan();
    await userEvent.click(card("NOTION"));
    await userEvent.click(card("JIRA"));

    expect(card("NOTION").getAttribute("aria-expanded")).toBe("false");
    expect(card("JIRA").getAttribute("aria-expanded")).toBe("true");
    expect(screen.queryByRole("button", { name: /TODO 보드/ })).toBeNull();
  });

  it("펼쳐진_카드를_다시_누르면_접힌다", async () => {
    renderFan();
    await userEvent.click(card("NOTION"));
    await userEvent.click(card("NOTION"));

    expect(card("NOTION").getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("button", { name: /TODO 보드/ })).toBeNull();
  });

  it("Esc는_펼쳐진_하위_항목을_먼저_접는다", async () => {
    const onEscape = vi.fn();
    renderFan({ onEscape });
    await userEvent.click(card("GOOGLE CALENDAR"));
    await userEvent.keyboard("{Escape}");

    expect(card("GOOGLE CALENDAR").getAttribute("aria-expanded")).toBe("false");
    // 접을 게 있었으므로 바깥으로 넘기지 않는다 — 한 번에 한 단계만 물러난다
    expect(onEscape).not.toHaveBeenCalled();
  });

  it("접을_게_없으면_Esc를_바깥에_넘긴다", async () => {
    const onEscape = vi.fn();
    renderFan({ onEscape });
    await userEvent.keyboard("{Escape}");

    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it("POMODORO_카드는_URL_항목_대신_타이머를_펼친다", async () => {
    renderFan();
    await userEvent.click(card("POMODORO"));

    expect(screen.getByText("타이머 자리")).toBeTruthy();
  });
});

describe("항목 열기", () => {
  it("하위_항목을_누르면_그_URL로_연다", async () => {
    const { onOpen } = renderFan();
    await userEvent.click(card("GOOGLE CALENDAR"));
    await userEvent.click(screen.getByRole("button", { name: /주간 뷰/ }));

    expect(onOpen).toHaveBeenCalledWith("https://calendar.google.com/calendar/r/week");
  });

  it("URL이_비어_있으면_열지_않고_안내를_보여준다", async () => {
    const { onOpen } = renderFan();
    await userEvent.click(card("JIRA"));
    await userEvent.click(screen.getByRole("button", { name: /내 티켓/ }));

    expect(onOpen).not.toHaveBeenCalled();
    expect(screen.getByText("URL이 아직 없어요")).toBeTruthy();
  });

  it("열기에_실패하면_그_자리에_알리고_런처는_그대로_있는다", async () => {
    const onOpen = vi.fn().mockRejectedValue(new Error("Chrome 없음"));
    renderFan({ onOpen });
    await userEvent.click(card("GOOGLE CALENDAR"));
    await userEvent.click(screen.getByRole("button", { name: /주간 뷰/ }));

    expect(await screen.findByText("열지 못했어요")).toBeTruthy();
    expect(card("GOOGLE CALENDAR").getAttribute("aria-expanded")).toBe("true");
  });
});
