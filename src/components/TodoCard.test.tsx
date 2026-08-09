import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { TodoOutcome, TodoSnapshot } from "../lib/notion";
import { TodoCard } from "./TodoCard";

afterEach(() => {
  cleanup();
  clearMocks();
  Reflect.deleteProperty(window, "Notification");
});

const LOADED: TodoSnapshot = {
  state: "loaded",
  date: "2026-08-09",
  page_id: "page-1",
  title: "[TODO]",
  items: [
    { id: "b1", text: "아침 운동", checked: true },
    { id: "b2", text: "보고서 작성", checked: false },
    { id: "b3", text: "이메일 정리", checked: false },
  ],
};
const NO_PAGE: TodoSnapshot = { state: "no_page", date: "2026-08-09" };
const NOT_CONNECTED: TodoSnapshot = {
  state: "not_connected",
  missing: ["token", "database", "data_source"],
};

const outcome = (
  snapshot: TodoSnapshot | null,
  notice: string | null = null,
): TodoOutcome => ({
  snapshot,
  notice,
});

/** TodoCard를 기본 props + 오버라이드로 렌더한다. */
function renderCard(overrides: Partial<Parameters<typeof TodoCard>[0]> = {}) {
  const props = {
    snapshot: LOADED as TodoSnapshot | null,
    isBusy: false,
    onRefresh: vi.fn(),
    onCreatePage: vi.fn(),
    onAdd: vi.fn(async (): Promise<boolean> => true),
    onToggle: vi.fn(),
    onEdit: vi.fn(async (): Promise<boolean> => true),
    ...overrides,
  };
  const view = render(<TodoCard {...props} />);
  return { props, view };
}

/**
 * App 통합 테스트용 IPC mock — 타이머/스토어/이벤트 커맨드는 기본 응답을 주고,
 * todo·notion 커맨드는 handlers로 오버라이드한다. 알림 권한은 Notification 심으로 통과시킨다.
 */
function mockAppIPC(handlers: Record<string, (args: unknown) => unknown> = {}) {
  Object.defineProperty(window, "Notification", {
    configurable: true,
    value: { permission: "granted", requestPermission: vi.fn(async () => "granted") },
  });
  const calls: Array<{ cmd: string; args: unknown }> = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (handlers[cmd]) return handlers[cmd](args);
    switch (cmd) {
      case "plugin:store|load":
        return 1;
      case "plugin:store|get":
        return [null, false];
      case "timer_set_config":
        return { focus_minutes: 25, break_minutes: 5 };
      case "timer_get_state":
        return { state: "idle" };
      case "notion_get_status":
        return { state: "connected", title: "계획표" };
      case "notion_todo_list":
        return LOADED;
      default:
        return undefined;
    }
  });
  return calls;
}

describe("목록 표시", () => {
  it("로드된_스냅샷의_항목들이_순서와_체크_상태대로_렌더링된다", () => {
    renderCard();

    const boxes = screen.getAllByRole("checkbox");
    expect(boxes).toHaveLength(3);
    expect(boxes[0]).toHaveAccessibleName("아침 운동");
    expect(boxes[1]).toHaveAccessibleName("보고서 작성");
    expect(boxes[2]).toHaveAccessibleName("이메일 정리");
    expect(boxes[0]).toBeChecked();
    expect(boxes[1]).not.toBeChecked();
    expect(boxes[2]).not.toBeChecked();
    // 헤더에 로드된 페이지 제목 표시 (비-[TODO] 행 대비)
    expect(screen.getByText("[TODO]")).toBeInTheDocument();
  });

  it("스냅샷_도착_전에는_불러오는_중_안내를_표시한다", () => {
    renderCard({ snapshot: null });

    expect(screen.getByRole("status")).toHaveTextContent("불러오는 중");
  });

  it("미연결_스냅샷은_안내_문구를_표시한다", () => {
    renderCard({ snapshot: NOT_CONNECTED });

    expect(screen.getByText(/Notion 연결 카드에서/)).toBeInTheDocument();
    // 미연결에서는 목록·추가 입력이 없다
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "새 할 일" })).not.toBeInTheDocument();
  });

  it("항목이_없으면_짧은_안내를_표시한다", () => {
    renderCard({ snapshot: { ...LOADED, items: [] } });

    expect(screen.getByText(/할 일이 아직 없어요/)).toBeInTheDocument();
    // 추가 입력은 그대로 쓸 수 있다
    expect(screen.getByRole("textbox", { name: "새 할 일" })).toBeInTheDocument();
  });
});

describe("페이지 없음 플로", () => {
  it("페이지_없음_상태는_만들기_버튼을_보여주고_클릭이_커맨드를_invoke한다", async () => {
    const calls = mockAppIPC({
      notion_todo_list: () => NO_PAGE,
      notion_todo_create_page: () => outcome({ ...LOADED, items: [] }),
    });
    render(<App />);

    expect(await screen.findByText("오늘 페이지 없음")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "오늘 페이지 만들기" }));

    expect(calls.some((c) => c.cmd === "notion_todo_create_page")).toBe(true);
    // 생성 후 빈 목록 안내가 표시된다 (AE2)
    expect(await screen.findByText(/할 일이 아직 없어요/)).toBeInTheDocument();
  });
});

describe("토글 플로", () => {
  it("체크박스_클릭이_notion_todo_toggle을_올바른_인자로_invoke하고_응답_스냅샷으로_갱신한다", async () => {
    const toggled: TodoSnapshot = {
      ...LOADED,
      items: [
        { id: "b1", text: "아침 운동", checked: true },
        { id: "b2", text: "보고서 작성", checked: true },
        { id: "b3", text: "이메일 정리", checked: false },
      ],
    };
    const calls = mockAppIPC({ notion_todo_toggle: () => outcome(toggled) });
    render(<App />);

    const box = await screen.findByRole("checkbox", { name: "보고서 작성" });
    expect(box).not.toBeChecked();
    await userEvent.click(box);

    const call = calls.find((c) => c.cmd === "notion_todo_toggle");
    expect(call).toBeDefined();
    expect(call!.args).toMatchObject({
      pageId: "page-1",
      blockId: "b2",
      checked: true,
      pageTitle: "[TODO]",
    });
    await waitFor(() =>
      expect(screen.getByRole("checkbox", { name: "보고서 작성" })).toBeChecked(),
    );
  });

  it("쓰기_반영_후_재조회_실패는_목록을_유지하고_안내를_보여준다", async () => {
    // snapshot: null = 쓰기는 반영됐지만 재조회만 실패 — 기존 목록 유지 + 안내 배너
    const 안내 = "변경은 반영됐지만 목록 조회에 실패했습니다. 새로고침해 주세요.";
    mockAppIPC({ notion_todo_toggle: () => outcome(null, 안내) });
    render(<App />);

    const box = await screen.findByRole("checkbox", { name: "보고서 작성" });
    await userEvent.click(box);

    await waitFor(() => expect(screen.getByText(안내)).toBeInTheDocument());
    // 목록은 마지막 스냅샷 그대로 유지된다 (비워지지 않음)
    expect(screen.getByRole("checkbox", { name: "아침 운동" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "이메일 정리" })).toBeInTheDocument();
  });
});

describe("추가 플로", () => {
  it("추가_입력은_성공_시에만_비워지고_실패_시_값이_유지된다", async () => {
    // 성공 → 비움
    const { props: ok, view } = renderCard({
      onAdd: vi.fn(async (): Promise<boolean> => true),
    });
    const input = screen.getByRole("textbox", { name: "새 할 일" });
    await userEvent.type(input, "새 항목");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));
    expect(ok.onAdd).toHaveBeenCalledWith("새 항목");
    await waitFor(() => expect(input).toHaveValue(""));
    view.unmount();

    // 실패 → 유지
    const { props: fail } = renderCard({
      onAdd: vi.fn(async (): Promise<boolean> => false),
    });
    const input2 = screen.getByRole("textbox", { name: "새 할 일" });
    await userEvent.type(input2, "새 항목");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));
    await waitFor(() => expect(fail.onAdd).toHaveBeenCalled());
    expect(input2).toHaveValue("새 항목");
  });

  it("빈_텍스트_저장은_커맨드를_invoke하지_않는다", async () => {
    const { props } = renderCard();

    // 추가 입력 — 공백만
    await userEvent.type(screen.getByRole("textbox", { name: "새 할 일" }), "   ");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));
    expect(props.onAdd).not.toHaveBeenCalled();

    // 인라인 편집 — 비우고 저장
    await userEvent.click(screen.getByRole("button", { name: "보고서 작성 편집" }));
    const editInput = screen.getByRole("textbox", { name: "할 일 편집" });
    await userEvent.clear(editInput);
    await userEvent.click(screen.getByRole("button", { name: "저장" }));
    expect(props.onEdit).not.toHaveBeenCalled();
  });
});

describe("인라인 편집", () => {
  it("편집은_Enter로_커밋되고_Escape는_원복한다", async () => {
    const { props } = renderCard();

    await userEvent.click(screen.getByRole("button", { name: "보고서 작성 편집" }));
    const editInput = screen.getByRole("textbox", { name: "할 일 편집" });
    expect(editInput).toHaveValue("보고서 작성");

    // Escape — 원복, invoke 없음
    await userEvent.type(editInput, " 수정");
    await userEvent.keyboard("{Escape}");
    expect(props.onEdit).not.toHaveBeenCalled();
    expect(screen.queryByRole("textbox", { name: "할 일 편집" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "보고서 작성 편집" })).toBeInTheDocument();

    // 다시 열어 Enter로 커밋 — 성공(true) 시 편집 모드가 닫힌다
    await userEvent.click(screen.getByRole("button", { name: "보고서 작성 편집" }));
    const editInput2 = screen.getByRole("textbox", { name: "할 일 편집" });
    await userEvent.clear(editInput2);
    await userEvent.type(editInput2, "보고서 마무리{Enter}");
    await waitFor(() => expect(props.onEdit).toHaveBeenCalledWith("b2", "보고서 마무리"));
    await waitFor(() =>
      expect(screen.queryByRole("textbox", { name: "할 일 편집" })).not.toBeInTheDocument(),
    );
  });

  it("blur는_자동_커밋하지_않는다", async () => {
    const { props } = renderCard();

    await userEvent.click(screen.getByRole("button", { name: "보고서 작성 편집" }));
    const editInput = screen.getByRole("textbox", { name: "할 일 편집" });
    await userEvent.type(editInput, " 수정");
    await userEvent.tab();

    expect(props.onEdit).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "할 일 편집" })).toBeInTheDocument();
  });

  it("편집_실패_시_편집_모드와_입력값이_유지된다", async () => {
    const { props } = renderCard({
      onEdit: vi.fn(async (): Promise<boolean> => false),
    });

    await userEvent.click(screen.getByRole("button", { name: "보고서 작성 편집" }));
    const editInput = screen.getByRole("textbox", { name: "할 일 편집" });
    await userEvent.clear(editInput);
    await userEvent.type(editInput, "보고서 마무리");
    await userEvent.click(screen.getByRole("button", { name: "저장" }));

    await waitFor(() => expect(props.onEdit).toHaveBeenCalled());
    expect(screen.getByRole("textbox", { name: "할 일 편집" })).toHaveValue("보고서 마무리");
  });
});

describe("busy·새로고침", () => {
  it("busy_중에는_추가·토글·편집·새로고침이_모두_비활성화된다", () => {
    const { view } = renderCard({ isBusy: true });

    expect(screen.getByRole("button", { name: "새로고침" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "추가" })).toBeDisabled();
    // 텍스트 입력도 잠근다 — 진행 중 타이핑이 성공 시 setAddRaw("")/cancelEdit()로 유실되는 것 방지
    expect(screen.getByRole("textbox", { name: "새 할 일" })).toBeDisabled();
    for (const box of screen.getAllByRole("checkbox")) {
      expect(box).toBeDisabled();
    }
    expect(screen.getByRole("button", { name: "아침 운동 편집" })).toBeDisabled();
    // 스냅샷이 있는 새로고침은 로딩 뷰로 바뀌지 않고 목록을 유지한다
    expect(screen.queryByText(/불러오는 중/)).not.toBeInTheDocument();
    view.unmount();

    // 페이지 없음 상태의 만들기 버튼도 잠긴다
    renderCard({ snapshot: NO_PAGE, isBusy: true });
    expect(screen.getByRole("button", { name: "오늘 페이지 만들기" })).toBeDisabled();
  });

  it("새로고침_클릭이_onRefresh를_호출한다", async () => {
    const { props } = renderCard();

    await userEvent.click(screen.getByRole("button", { name: "새로고침" }));

    expect(props.onRefresh).toHaveBeenCalled();
  });

  it("스냅샷_도착_전에도_새로고침_버튼이_보인다", async () => {
    // 첫 로드가 실패해 스냅샷이 null로 남아도 새로고침으로 재시도할 수 있어야 한다
    const { props } = renderCard({ snapshot: null });

    const refresh = screen.getByRole("button", { name: "새로고침" });
    expect(refresh).toBeEnabled();
    await userEvent.click(refresh);

    expect(props.onRefresh).toHaveBeenCalled();
  });
});

describe("오류·안내 배너", () => {
  it("실패_시_배너에_원인_메시지가_표시된다", async () => {
    const calls = mockAppIPC({
      notion_todo_add: () => {
        throw "요청 한도를 초과했습니다. 잠시 후 다시 시도해 주세요";
      },
    });
    render(<App />);

    const input = await screen.findByRole("textbox", { name: "새 할 일" });
    await userEvent.type(input, "새 항목");
    const listCallsBefore = calls.filter((c) => c.cmd === "notion_todo_list").length;
    await userEvent.click(screen.getByRole("button", { name: "추가" }));

    await screen.findByText("요청 한도를 초과했습니다. 잠시 후 다시 시도해 주세요");
    // 실패 시에도 목록을 1회 재조회한다 (R8)
    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(
        listCallsBefore + 1,
      ),
    );
    // 입력값은 유지된다
    expect(input).toHaveValue("새 항목");
  });

  it("성공_응답의_notice가_배너로_표시된다", async () => {
    mockAppIPC({
      notion_todo_toggle: () =>
        outcome(LOADED, "할 일이 원격에서 바뀌어 목록을 새로 불러왔습니다. 다시 시도해 주세요."),
    });
    render(<App />);

    await userEvent.click(await screen.findByRole("checkbox", { name: "보고서 작성" }));

    expect(await screen.findByText(/원격에서 바뀌어/)).toBeInTheDocument();
  });
});

describe("재표시 재조회", () => {
  it("팝오버_재표시_시_todo_목록을_재조회한다", async () => {
    const calls = mockAppIPC();
    render(<App />);
    await screen.findByRole("checkbox", { name: "아침 운동" });

    const before = calls.filter((c) => c.cmd === "notion_todo_list").length;
    document.dispatchEvent(new Event("visibilitychange"));

    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(before + 1),
    );
  });

  it("팝오버_재표시_재조회는_진행_중_쓰기와_경쟁하지_않는다", async () => {
    // 토글 쓰기를 미해결 promise로 붙잡아 둔 채 재표시 이벤트를 쏜다
    let resolveToggle!: (value: TodoOutcome) => void;
    const pendingToggle = new Promise<TodoOutcome>((resolve) => {
      resolveToggle = resolve;
    });
    const written: TodoSnapshot = {
      ...LOADED,
      items: [
        { id: "b1", text: "아침 운동", checked: true },
        { id: "b2", text: "보고서 작성", checked: true },
        { id: "b3", text: "이메일 정리", checked: false },
      ],
    };
    const calls = mockAppIPC({ notion_todo_toggle: () => pendingToggle });
    render(<App />);

    const box = await screen.findByRole("checkbox", { name: "보고서 작성" });
    await userEvent.click(box);
    // 쓰기 진행 중(busy) — 재표시 재조회는 스킵된다
    await waitFor(() =>
      expect(screen.getByRole("checkbox", { name: "보고서 작성" })).toBeDisabled(),
    );
    const listCallsBefore = calls.filter((c) => c.cmd === "notion_todo_list").length;
    document.dispatchEvent(new Event("visibilitychange"));
    expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(listCallsBefore);

    // 쓰기 완료 — 최종 목록은 쓰기의 스냅샷이다 (낡은 재조회 응답이 이기지 않는다)
    resolveToggle(outcome(written));
    await waitFor(() =>
      expect(screen.getByRole("checkbox", { name: "보고서 작성" })).toBeChecked(),
    );
    expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(listCallsBefore);
  });
});
