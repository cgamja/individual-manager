import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PetCountCard } from "./PetCountCard";

afterEach(cleanup);

const props = {
  count: 2,
  max: 8,
  focused: 1 as number | null,
  onAdd: () => {},
  onRemove: () => {},
};

describe("PetCountCard", () => {
  it("마리마다_현재_마릿수를_보여준다", () => {
    render(<PetCountCard {...props} />);
    expect(screen.getByRole("heading")).toHaveTextContent("펭귄 2마리");
  });

  it("추가를_누르면_onAdd를_부른다", async () => {
    const onAdd = vi.fn();
    render(<PetCountCard {...props} onAdd={onAdd} />);
    await userEvent.click(screen.getByRole("button", { name: "펭귄 추가" }));
    expect(onAdd).toHaveBeenCalledOnce();
  });

  it("마지막_한_마리면_삭제_버튼이_비활성된다", () => {
    render(<PetCountCard {...props} count={1} />);
    expect(screen.getByRole("button", { name: "이 펭귄 삭제" })).toBeDisabled();
    expect(screen.getByText(/마지막 한 마리는 지울 수 없어요/)).toBeInTheDocument();
  });

  it("우클릭_대상이_없으면_삭제_버튼이_비활성된다", () => {
    // 트레이로 열면 어느 펭귄 이야기인지 알 수 없다
    render(<PetCountCard {...props} focused={null} />);
    expect(screen.getByRole("button", { name: "이 펭귄 삭제" })).toBeDisabled();
    expect(screen.getByText(/우클릭해서 열어 주세요/)).toBeInTheDocument();
  });

  it("상한에_닿으면_추가_버튼이_비활성된다", () => {
    render(<PetCountCard {...props} count={8} />);
    expect(screen.getByRole("button", { name: "펭귄 추가" })).toBeDisabled();
    expect(screen.getByText("8마리가 최대예요")).toBeInTheDocument();
  });
});
