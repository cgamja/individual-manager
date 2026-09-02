import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsCard } from "./SettingsCard";

afterEach(cleanup);

const props = {
  petEnabled: true,
  onPetEnabledChange: () => {},
  soundEnabled: false,
  onSoundEnabledChange: () => {},
  volume: 2,
  onVolumeChange: () => {},
  theme: "system" as const,
  onThemeChange: () => {},
  pinballEnabled: false,
  onPinballEnabledChange: () => {},
};

describe("SettingsCard", () => {
  it("펭귄_토글의_현재_상태를_보여준다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByLabelText("바탕화면 펭귄")).toBeChecked();
  });

  it("펭귄_토글을_끄면_false로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onPetEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("바탕화면 펭귄"));
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("소리는_기본이_꺼짐으로_보인다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByLabelText("효과음")).not.toBeChecked();
  });

  it("소리_토글을_켜면_true로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onSoundEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("효과음"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("음량을_움직이면_숫자로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onVolumeChange={onChange} />);
    fireEvent.change(screen.getByLabelText("음량"), { target: { value: "0" } });
    expect(onChange).toHaveBeenCalledWith(0);
  });

  it("테마를_고르면_값으로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onThemeChange={onChange} />);
    fireEvent.change(screen.getByLabelText("테마"), { target: { value: "dark" } });
    expect(onChange).toHaveBeenCalledWith("dark");
  });

  it("테마는_기본이_시스템이다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByLabelText("테마")).toHaveValue("system");
  });

  it("핀볼은_기본이_꺼짐으로_보인다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByLabelText("핀볼 모드")).not.toBeChecked();
  });

  it("핀볼_토글을_켜면_true로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onPinballEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("핀볼 모드"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("핀볼이_무엇을_바꾸는지_적어_둔다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/방망이 대신 펭귄이 날아가고/)).toBeInTheDocument();
    expect(screen.getByText(/끄면 원래대로 돌아와요/)).toBeInTheDocument();
  });

  it("무엇이_들리는지_적어_둔다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/때리거나 던지면 소리가 나요/)).toBeInTheDocument();
    expect(screen.getByText(/그 밖에는 조용해요/)).toBeInTheDocument();
  });
});
