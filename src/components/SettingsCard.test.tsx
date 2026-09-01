import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsCard } from "./SettingsCard";

afterEach(cleanup);

const props = {
  petEnabled: true,
  onPetEnabledChange: () => {},
  soundEnabled: false,
  onSoundEnabledChange: () => {},
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
    // 상주 앱이 예고 없이 소리를 내면 사고가 난다 (PRD Q6)
    render(<SettingsCard {...props} />);
    expect(screen.getByLabelText("효과음")).not.toBeChecked();
  });

  it("소리_토글을_켜면_true로_알린다", async () => {
    const onChange = vi.fn();
    render(<SettingsCard {...props} onSoundEnabledChange={onChange} />);
    await userEvent.click(screen.getByLabelText("효과음"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("핀볼은_기본이_꺼짐으로_보인다", () => {
    // 착지 4단계를 가리는 모드다 — 사용자가 켜기 전에는 아무것도 바뀌지 않아야 한다
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
    // 이름만 보고는 **클릭이 바뀌는 것**을 알 수 없고, 끄면 돌아온다는 것도
    // 모르면 켜기가 무섭다
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/방망이 대신 펭귄이 날아가고/)).toBeInTheDocument();
    expect(screen.getByText(/끄면 원래대로 돌아와요/)).toBeInTheDocument();
  });

  it("아직_낼_소리가_없다는_것을_알린다", () => {
    // 켰는데 조용하면 고장으로 읽힌다
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/아직 낼 소리가 없어요/)).toBeInTheDocument();
  });
});
