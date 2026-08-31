import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MotionCard } from "./MotionCard";

afterEach(cleanup);

const 얼음낚시 = () => screen.getByRole("button", { name: "얼음낚시" });

describe("MotionCard", () => {
  it("얼음낚시를_누르면_지정한_펭귄에게_시킨다", async () => {
    const onFish = vi.fn().mockResolvedValue(undefined);
    render(<MotionCard focused={2} onFish={onFish} />);
    await userEvent.click(얼음낚시());
    expect(onFish).toHaveBeenCalledOnce();
  });

  it("우클릭한_펭귄이_없으면_누를_수_없고_이유를_적는다", () => {
    // 트레이로 팝오버를 열면 대상이 없다 — 눌리는데 아무 일도 없으면 고장으로 읽힌다
    render(<MotionCard focused={null} onFish={vi.fn()} />);
    expect(얼음낚시()).toBeDisabled();
    expect(screen.getByText("낚시할 펭귄을 우클릭해서 열어 주세요")).toBeInTheDocument();
  });

  it("던지거나_때리면_중단된다는_것을_누르기_전에_알려_준다", () => {
    // 모르고 건드리면 버튼이 안 먹은 것처럼 보인다
    render(<MotionCard focused={1} onFish={vi.fn()} />);
    expect(screen.getByText(/던지거나 때리면 그 자리에서 그만둬요/)).toBeInTheDocument();
  });

  it("허공에서도_된다는_것을_알려_준다", () => {
    render(<MotionCard focused={1} onFish={vi.fn()} />);
    expect(screen.getByText(/허공에 드리워요/)).toBeInTheDocument();
  });

  it("거절되면_사유를_그대로_보여준다", async () => {
    const onFish = vi.fn().mockRejectedValue("들고 있는 중에는 못 해요. 내려놓고 다시 눌러 주세요");
    render(<MotionCard focused={1} onFish={onFish} />);
    await userEvent.click(얼음낚시());
    expect(
      await screen.findByText("들고 있는 중에는 못 해요. 내려놓고 다시 눌러 주세요"),
    ).toBeInTheDocument();
  });

  it("사유가_문자열이_아니어도_조용히_넘어가지_않는다", async () => {
    const onFish = vi.fn().mockRejectedValue(new Error("boom"));
    render(<MotionCard focused={1} onFish={onFish} />);
    await userEvent.click(얼음낚시());
    expect(await screen.findByText("낚시를 시키지 못했어요")).toBeInTheDocument();
  });

  it("다시_눌러_성공하면_지난_사유가_사라진다", async () => {
    const onFish = vi
      .fn()
      .mockRejectedValueOnce("들고 있는 중에는 못 해요. 내려놓고 다시 눌러 주세요")
      .mockResolvedValueOnce(undefined);
    render(<MotionCard focused={1} onFish={onFish} />);
    await userEvent.click(얼음낚시());
    expect(await screen.findByText(/들고 있는 중에는/)).toBeInTheDocument();
    await userEvent.click(얼음낚시());
    expect(screen.queryByText(/들고 있는 중에는/)).not.toBeInTheDocument();
  });
});
