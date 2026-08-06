import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TimerSnapshot } from "../lib/timer";
import { TimerCard } from "./TimerCard";

afterEach(cleanup);

const noop = {
  onStart: () => {},
  onPause: () => {},
  onResume: () => {},
  onReset: () => {},
};

function renderCard(snapshot: TimerSnapshot, overrides: Partial<typeof noop> = {}) {
  return render(<TimerCard snapshot={snapshot} {...noop} {...overrides} />);
}

describe("상태별 버튼 노출", () => {
  it("idle에서는_시작_버튼만_보인다", () => {
    renderCard({ state: "idle" });
    expect(screen.getByRole("button", { name: "집중 시작" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "일시정지" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "리셋" })).not.toBeInTheDocument();
  });

  it("running에서는_일시정지와_리셋이_보인다", () => {
    renderCard({ state: "running", phase: "focus", remaining_ms: 1_500_000 });
    expect(screen.getByRole("button", { name: "일시정지" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "리셋" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "재개" })).not.toBeInTheDocument();
  });

  it("paused에서는_재개와_리셋이_보인다", () => {
    renderCard({ state: "paused", phase: "focus", remaining_ms: 900_000 });
    expect(screen.getByRole("button", { name: "재개" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "리셋" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "일시정지" })).not.toBeInTheDocument();
  });

  it("finished에서는_다음_단계_시작과_리셋이_보인다", async () => {
    const onStart = vi.fn();
    renderCard({ state: "finished", phase: "focus" }, { onStart });
    expect(screen.getByText("집중 세션 종료")).toBeInTheDocument();
    const next = screen.getByRole("button", { name: "휴식 시작" });
    expect(screen.getByRole("button", { name: "리셋" })).toBeInTheDocument();

    // Covers AE3: 집중 종료 후 원클릭으로 휴식 세션을 시작한다 (교대 규칙)
    await userEvent.click(next);
    expect(onStart).toHaveBeenCalledWith("break");
  });

  it("휴식_세션_종료_후에는_집중_시작이_보인다", () => {
    renderCard({ state: "finished", phase: "break" });
    expect(screen.getByText("휴식 세션 종료")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "집중 시작" })).toBeInTheDocument();
  });
});

describe("남은 시간 렌더링", () => {
  it("running의_남은_시간을_mmss로_표시한다", () => {
    renderCard({ state: "running", phase: "focus", remaining_ms: 1_499_000 });
    expect(screen.getByText("24:59")).toBeInTheDocument();
  });

  it("paused는_일시정지_표시와_고정_시간을_보여준다", () => {
    renderCard({ state: "paused", phase: "break", remaining_ms: 90_000 });
    expect(screen.getByText(/일시정지/)).toBeInTheDocument();
    expect(screen.getByText("01:30")).toBeInTheDocument();
  });
});
