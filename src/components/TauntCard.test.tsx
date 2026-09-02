import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TauntCard } from "./TauntCard";

afterEach(cleanup);

const LINES = ["일 안 해요?", "아 진짜 왜요", "그래서 뭐 어쩌라고"];

describe("대사 추가", () => {
  it("새_대사를_적고_추가하면_목록_끝에_붙는다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.type(screen.getByLabelText("새 대사"), "커밋은 했어요?");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));

    expect(onChange).toHaveBeenCalledWith([...LINES, "커밋은 했어요?"]);
  });

  it("엔터로도_추가된다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.type(screen.getByLabelText("새 대사"), "그거 아까도 실패했어요{Enter}");

    expect(onChange).toHaveBeenCalledWith([...LINES, "그거 아까도 실패했어요"]);
  });

  it("빈_대사는_추가되지_않는다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    const add = screen.getByRole("button", { name: "추가" });
    expect(add).toBeDisabled();

    await userEvent.type(screen.getByLabelText("새 대사"), "   {Enter}");
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("대사 삭제", () => {
  it("삭제하면_그_줄만_빠진다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.click(screen.getByLabelText("대사 2 삭제"));

    expect(onChange).toHaveBeenCalledWith(["일 안 해요?", "그래서 뭐 어쩌라고"]);
  });

  it("전부_지우면_조용해진다고_알린다", () => {
    render(<TauntCard lines={[]} onChange={() => {}} />);
    expect(screen.getByText(/조용해집니다/)).toBeInTheDocument();
  });
});

describe("대사 수정", () => {
  it("눌러서_고치면_그_자리에_반영된다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.click(screen.getByRole("button", { name: "일 안 해요?" }));
    const input = screen.getByLabelText("대사 1 수정");
    await userEvent.clear(input);
    await userEvent.type(input, "일은 언제 해요?{Enter}");

    expect(onChange).toHaveBeenCalledWith([
      "일은 언제 해요?",
      "아 진짜 왜요",
      "그래서 뭐 어쩌라고",
    ]);
  });

  it("수정에서_비우면_삭제와_같다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.click(screen.getByRole("button", { name: "아 진짜 왜요" }));
    const input = screen.getByLabelText("대사 2 수정");
    await userEvent.clear(input);
    await userEvent.type(input, "{Enter}");

    expect(onChange).toHaveBeenCalledWith(["일 안 해요?", "그래서 뭐 어쩌라고"]);
  });

  it("Escape로_수정을_취소하면_바뀌지_않는다", async () => {
    const onChange = vi.fn();
    render(<TauntCard lines={LINES} onChange={onChange} />);

    await userEvent.click(screen.getByRole("button", { name: "아 진짜 왜요" }));
    const input = screen.getByLabelText("대사 2 수정");
    await userEvent.clear(input);
    await userEvent.type(input, "안 쓸 말{Escape}");

    expect(onChange).not.toHaveBeenCalled();
  });
});
