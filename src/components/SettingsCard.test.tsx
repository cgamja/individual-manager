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
  size: 100,
  onSizeChange: () => {},
  theme: "system" as const,
  onThemeChange: () => {},
  pinballEnabled: false,
  onPinballEnabledChange: () => {},
  bowlingRunning: false,
  onBowling: () => {},
  volleyballRunning: false,
  onVolleyball: () => {},
};

describe("SettingsCard", () => {
  it("크기를_움직이면_퍼센트로_알린다", () => {
    const onSizeChange = vi.fn();
    render(<SettingsCard {...props} onSizeChange={onSizeChange} />);
    fireEvent.change(screen.getByLabelText("크기"), { target: { value: "60" } });
    expect(onSizeChange).toHaveBeenCalledWith(60);
  });

  it("현재_크기가_퍼센트로_보인다", () => {
    render(<SettingsCard {...props} size={60} />);
    expect(screen.getByText("60%")).toBeInTheDocument();
  });

  it("크기_슬라이더는_50에서_150까지_10단위다", () => {
    // 범위가 어긋나면 저장 쪽 정화가 조용히 되돌려 슬라이더가 안 움직이는 것처럼 보인다.
    render(<SettingsCard {...props} />);
    const slider = screen.getByLabelText("크기");
    expect(slider).toHaveAttribute("min", "50");
    expect(slider).toHaveAttribute("max", "150");
    expect(slider).toHaveAttribute("step", "10");
  });

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

  it("볼링_버튼이_전역_커맨드를_부른다", async () => {
    const onBowling = vi.fn();
    render(<SettingsCard {...props} onBowling={onBowling} />);
    await userEvent.click(screen.getByRole("button", { name: "볼링 한 판" }));
    expect(onBowling).toHaveBeenCalled();
  });

  it("판이_도는_중에는_버튼이_비활성이다", () => {
    // 이미 도는 중에 또 누르면 무시되므로(A3), 눌리는데 아무 일도 없으면
    // 고장으로 읽힌다.
    render(<SettingsCard {...props} bowlingRunning />);
    expect(screen.getByRole("button", { name: /굴리는 중/ })).toBeDisabled();
  });

  it("볼링이_언제_끝나는지_적어_둔다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/공이 멎으면 끝나요/)).toBeInTheDocument();
  });

  it("무엇이_들리는지_적어_둔다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/때리거나 던지면 소리가 나요/)).toBeInTheDocument();
    expect(screen.getByText(/그 밖에는 조용해요/)).toBeInTheDocument();
  });

  it("비치발리볼을_누르면_알린다", async () => {
    const onVolleyball = vi.fn();
    render(<SettingsCard {...props} onVolleyball={onVolleyball} />);
    await userEvent.click(screen.getByRole("button", { name: "비치발리볼 한 판" }));
    expect(onVolleyball).toHaveBeenCalledTimes(1);
  });

  it("판이_도는_동안_두_버튼이_모두_비활성이다", () => {
    // **두 판은 서로를 배제한다** — 동시에 열리면 한쪽이 상대 판의 마리를
    // 끌어가고 창만 남는다. 눌리는데 아무 일도 안 일어나면 고장으로 읽힌다.
    const { rerender } = render(<SettingsCard {...props} volleyballRunning />);
    expect(screen.getByRole("button", { name: "치는 중…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "볼링 한 판" })).toBeDisabled();

    rerender(<SettingsCard {...props} bowlingRunning />);
    expect(screen.getByRole("button", { name: "굴리는 중…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "비치발리볼 한 판" })).toBeDisabled();
  });

  it("아무_판도_안_돌면_둘_다_누를_수_있다", () => {
    render(<SettingsCard {...props} />);
    expect(screen.getByRole("button", { name: "비치발리볼 한 판" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "볼링 한 판" })).toBeEnabled();
  });

  it("구경만_하면_된다고_적어_둔다", () => {
    // 사용자 입력이 없는 유일한 판이라, 안내가 "무엇을 하세요"가 아니라
    // "무엇이 보입니다"여야 한다.
    render(<SettingsCard {...props} />);
    expect(screen.getByText(/구경만 하면 돼요/)).toBeInTheDocument();
    expect(screen.getByText(/두 마리부터/)).toBeInTheDocument();
  });
});
