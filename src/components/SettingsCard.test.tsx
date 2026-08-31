import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsCard } from "./SettingsCard";

afterEach(cleanup);

describe("SettingsCard", () => {
  it("펭귄_토글의_현재_상태를_보여준다", () => {
    render(<SettingsCard petEnabled onPetEnabledChange={() => {}} />);
    expect(screen.getByLabelText("바탕화면 펭귄")).toBeChecked();
  });

  it("토글을_끄면_false로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard petEnabled onPetEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("바탕화면 펭귄"));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("토글을_켜면_true로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard petEnabled={false} onPetEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("바탕화면 펭귄"));
    expect(onChange).toHaveBeenCalledWith(true);
  });
});
