import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsCard } from "./SettingsCard";

afterEach(cleanup);

const CONFIG = { focus_minutes: 25, break_minutes: 5 };

describe("설정 입력 검증", () => {
  it("유효한_분_값을_입력하면_onChange가_호출된다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard config={CONFIG} disabled={false} onChange={onChange} />);

    const focus = screen.getByLabelText("집중(분)");
    await userEvent.clear(focus);
    await userEvent.type(focus, "50");
    await userEvent.tab();

    expect(onChange).toHaveBeenCalledWith({ focus_minutes: 50, break_minutes: 5 });
  });

  it("일_미만_값은_거부되어_onChange가_호출되지_않는다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard config={CONFIG} disabled={false} onChange={onChange} />);

    const focus = screen.getByLabelText("집중(분)");
    await userEvent.clear(focus);
    await userEvent.type(focus, "0");
    await userEvent.tab();

    expect(onChange).not.toHaveBeenCalled();
    expect(focus).toHaveValue("25");
  });

  it("비숫자_입력은_거부되어_onChange가_호출되지_않는다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard config={CONFIG} disabled={false} onChange={onChange} />);

    const brk = screen.getByLabelText("휴식(분)");
    await userEvent.clear(brk);
    await userEvent.type(brk, "abc");
    await userEvent.tab();

    expect(onChange).not.toHaveBeenCalled();
  });

  it("거부된_입력은_되돌려진다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard config={CONFIG} disabled={false} onChange={onChange} />);

    const focus = screen.getByLabelText("집중(분)");
    await userEvent.clear(focus);
    await userEvent.type(focus, "0");
    await userEvent.tab();

    expect(focus).toHaveValue("25");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("상한_초과_입력은_거부되어_되돌려진다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard config={CONFIG} disabled={false} onChange={onChange} />);

    const focus = screen.getByLabelText("집중(분)");
    await userEvent.clear(focus);
    await userEvent.type(focus, "1000");
    await userEvent.tab();

    expect(onChange).not.toHaveBeenCalled();
    expect(focus).toHaveValue("25");
  });
});

describe("idle 외 상태 비활성화", () => {
  it("disabled면_입력이_비활성화되고_안내_문구가_보인다", () => {
    render(<SettingsCard config={CONFIG} disabled={true} onChange={() => {}} />);
    expect(screen.getByLabelText("집중(분)")).toBeDisabled();
    expect(screen.getByLabelText("휴식(분)")).toBeDisabled();
    expect(screen.getByText(/유휴 상태일 때 변경/)).toBeInTheDocument();
  });
});
