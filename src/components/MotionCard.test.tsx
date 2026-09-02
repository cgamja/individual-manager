import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MotionCard, type Motion } from "./MotionCard";

afterEach(cleanup);

const 동작들 = (run = vi.fn().mockResolvedValue(undefined)): Motion[] => [
  { name: "얼음낚시", note: "30~60초 앉아 있어요", run },
  { name: "슬라이딩", note: "2.4초면 끝나요", run },
];

describe("MotionCard", () => {
  it("누른_동작을_시킨다", async () => {
    const 낚시 = vi.fn().mockResolvedValue(undefined);
    const 슬라이딩 = vi.fn().mockResolvedValue(undefined);
    render(
      <MotionCard
        focused={2}
        motions={[
          { name: "얼음낚시", note: "a", run: 낚시 },
          { name: "슬라이딩", note: "b", run: 슬라이딩 },
        ]}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    expect(슬라이딩).toHaveBeenCalledOnce();
    expect(낚시).not.toHaveBeenCalled();
  });

  it("동작마다_다른_설명을_보여준다", async () => {
    render(<MotionCard focused={1} motions={동작들()} />);
    expect(screen.getByText("30~60초 앉아 있어요")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    expect(screen.getByText("2.4초면 끝나요")).toBeInTheDocument();
    expect(screen.queryByText("30~60초 앉아 있어요")).not.toBeInTheDocument();
  });

  it("우클릭한_펭귄이_없으면_전부_잠기고_이유를_적는다", () => {
    render(<MotionCard focused={null} motions={동작들()} />);
    for (const name of ["얼음낚시", "슬라이딩"]) {
      expect(screen.getByRole("button", { name }), name).toBeDisabled();
    }
    expect(screen.getByText("시킬 펭귄을 우클릭해서 열어 주세요")).toBeInTheDocument();
  });

  it("거절되면_사유를_그대로_보여준다", async () => {
    render(
      <MotionCard focused={1} motions={동작들(vi.fn().mockRejectedValue("바닥에 내려놓고 눌러 주세요"))} />,
    );
    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    expect(await screen.findByText("바닥에 내려놓고 눌러 주세요")).toBeInTheDocument();
  });

  it("늦게_도착한_거절_사유는_버린다", async () => {
    let 낚시_거절: (reason: string) => void = () => {};
    const 낚시 = vi.fn(
      () => new Promise<void>((_, reject) => { 낚시_거절 = reject; }),
    );
    render(
      <MotionCard
        focused={1}
        motions={[
          { name: "얼음낚시", note: "a", run: 낚시 },
          { name: "슬라이딩", note: "b", run: vi.fn().mockResolvedValue(undefined) },
        ]}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "얼음낚시" }));
    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    낚시_거절("이미 낚시하는 중이거나 들고 계세요");
    await new Promise((r) => setTimeout(r, 0));

    expect(screen.queryByText("이미 낚시하는 중이거나 들고 계세요")).not.toBeInTheDocument();
    expect(screen.getByText("b")).toBeInTheDocument();
  });

  it("사유가_문자열이_아니어도_조용히_넘어가지_않는다", async () => {
    render(<MotionCard focused={1} motions={동작들(vi.fn().mockRejectedValue(new Error("boom")))} />);
    await userEvent.click(screen.getByRole("button", { name: "얼음낚시" }));
    expect(await screen.findByText("시키지 못했어요")).toBeInTheDocument();
  });

  it("다시_눌러_성공하면_지난_사유가_사라진다", async () => {
    const run = vi
      .fn()
      .mockRejectedValueOnce("바닥에 내려놓고 눌러 주세요")
      .mockResolvedValueOnce(undefined);
    render(<MotionCard focused={1} motions={동작들(run)} />);
    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    expect(await screen.findByText("바닥에 내려놓고 눌러 주세요")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "슬라이딩" }));
    expect(screen.queryByText("바닥에 내려놓고 눌러 주세요")).not.toBeInTheDocument();
  });
});
