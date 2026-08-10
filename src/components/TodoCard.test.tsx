import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { TodoOutcome, TodoSnapshot } from "../lib/notion";
import { TodoCard, type CreateRowFormResult } from "./TodoCard";

afterEach(() => {
  cleanup();
  clearMocks();
  Reflect.deleteProperty(window, "Notification");
});

/** loaded 변형만 — 스프레드 변형이 유니언으로 넓어지지 않게 좁혀 둔다. */
type LoadedSnapshot = Extract<TodoSnapshot, { state: "loaded" }>;

const LOADED: LoadedSnapshot = {
  state: "loaded",
  date: "2026-08-09",
  page_id: "page-1",
  title: "[TODO]",
  items: [
    { id: "b1", text: "아침 운동", checked: true, category: "공부" },
    { id: "b2", text: "보고서 작성", checked: false, category: "공부" },
    { id: "b3", text: "이메일 정리", checked: false, category: "기타" },
  ],
  is_today: true,
  performance: null,
  range_start: null,
  range_end: null,
};

/** LOADED 변형 — 결과 타입이 loaded로 고정돼 `items` 접근·오버라이드가 안전하다. */
const loaded = (overrides: Partial<LoadedSnapshot> = {}): LoadedSnapshot => ({
  ...LOADED,
  ...overrides,
});

const NO_PAGE: TodoSnapshot = {
  state: "no_page",
  date: "2026-08-09",
  is_today: true,
};
const NOT_CONNECTED: TodoSnapshot = {
  state: "not_connected",
  missing: ["token", "database", "data_source"],
};
/** 날짜 전환 후 스냅샷 — 과거 [TODO] 행을 열어 둔 상태. */
const NOT_TODAY: LoadedSnapshot = {
  state: "loaded",
  date: "2026-08-08",
  page_id: "page-0",
  title: "[TODO]",
  items: [{ id: "c1", text: "쉬기", checked: false, category: null }],
  is_today: false,
  performance: "일부",
  range_start: null,
  range_end: null,
};

/** 수행도 세그먼트 안에서만 버튼을 찾는다 — "기타"가 카테고리와 겹친다. */
const perfGroup = () => screen.getByRole("group", { name: "수행도" });
const perfButton = (name: string) =>
  within(perfGroup()).getByRole("button", { name });
/** 카테고리 세그먼트 안에서만 버튼을 찾는다 — 수행도와 "기타"가 겹친다. */
const catButton = (name: string) =>
  within(screen.getByRole("group", { name: "추가 카테고리" })).getByRole("button", {
    name,
  });

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
    onCreateRow: vi.fn(
      async (): Promise<CreateRowFormResult> => ({ state: "created" }),
    ),
    onOpenPage: vi.fn(async (): Promise<boolean> => true),
    onSetPerformance: vi.fn(),
    ...overrides,
  };
  const view = render(<TodoCard {...props} />);
  return { props, view };
}

/** 쓰기 커맨드 인자의 `meta.performance` — 백엔드가 되싣는 값을 흉내 낸다.
 * 프론트가 넘긴 값이 곧 스냅샷에 남는 값이므로, 무엇을 넘겼는지가 화면에 드러난다. */
function metaPerformance(args: unknown): string | null {
  return (args as { meta?: { performance?: string } }).meta?.performance ?? null;
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
    // 헤더에 로드된 페이지 제목 표시
    expect(screen.getByText("[TODO]")).toBeInTheDocument();
  });

  it("목록이_카테고리_라벨로_나뉘어_보인다", () => {
    // 공부 2 + 기타 1 — 페이지 순서를 유지한 채 category가 바뀌는 지점에 라벨 삽입
    const { view } = renderCard();

    const rows = screen.getAllByRole("listitem");
    expect(rows.map((r) => r.textContent)).toEqual([
      "공부",
      "아침 운동",
      "보고서 작성",
      "기타",
      "이메일 정리",
    ]);
    // 라벨은 체크박스가 없는 순수 라벨이다
    expect(screen.getAllByRole("checkbox")).toHaveLength(3);
    view.unmount();

    // 첫 헤딩 전(category null) 항목은 라벨 없이 맨 앞 그대로
    renderCard({
      snapshot: loaded({
        items: [
          { id: "b0", text: "머리말 항목", checked: false, category: null },
          ...LOADED.items,
        ],
      }),
    });
    const rows2 = screen.getAllByRole("listitem");
    expect(rows2.map((r) => r.textContent)).toEqual([
      "머리말 항목",
      "공부",
      "아침 운동",
      "보고서 작성",
      "기타",
      "이메일 정리",
    ]);
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
    renderCard({ snapshot: loaded({ items: [] }) });

    expect(screen.getByText(/할 일이 아직 없어요/)).toBeInTheDocument();
    // 추가 입력은 그대로 쓸 수 있다
    expect(screen.getByRole("textbox", { name: "새 할 일" })).toBeInTheDocument();
  });
});

describe("페이지 없음 플로", () => {
  it("페이지_없음_상태는_만들기_버튼을_보여주고_클릭이_커맨드를_invoke한다", async () => {
    const calls = mockAppIPC({
      notion_todo_list: () => NO_PAGE,
      notion_todo_create_page: () => outcome(loaded({ items: [] })),
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
    const toggled = loaded({
      items: [
        { id: "b1", text: "아침 운동", checked: true, category: "공부" },
        { id: "b2", text: "보고서 작성", checked: true, category: "공부" },
        { id: "b3", text: "이메일 정리", checked: false, category: "기타" },
      ],
    });
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
    expect(ok.onAdd).toHaveBeenCalledWith("새 항목", "공부");
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

  it("추가_시_선택한_카테고리가_전달된다", async () => {
    const calls = mockAppIPC({ notion_todo_add: () => outcome(LOADED) });
    render(<App />);
    const input = await screen.findByRole("textbox", { name: "새 할 일" });

    // 기본 선택은 공부다
    expect(catButton("공부")).toHaveAttribute("aria-pressed", "true");
    await userEvent.type(input, "단어 암기");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));
    await waitFor(() => {
      const first = calls.find((c) => c.cmd === "notion_todo_add");
      expect(first).toBeDefined();
      expect(first!.args).toMatchObject({ text: "단어 암기", category: "공부" });
    });

    // 기타 선택 후 추가 → category "기타"
    await userEvent.click(catButton("기타"));
    await userEvent.type(input, "짐 정리");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));
    await waitFor(() => {
      const adds = calls.filter((c) => c.cmd === "notion_todo_add");
      expect(adds).toHaveLength(2);
      expect(adds[1].args).toMatchObject({ text: "짐 정리", category: "기타" });
    });
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
    // 카테고리 세그먼트도 잠근다
    expect(catButton("공부")).toBeDisabled();
    expect(catButton("기타")).toBeDisabled();
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
    const written = loaded({
      items: [
        { id: "b1", text: "아침 운동", checked: true, category: "공부" },
        { id: "b2", text: "보고서 작성", checked: true, category: "공부" },
        { id: "b3", text: "이메일 정리", checked: false, category: "기타" },
      ],
    });
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

  it("팝오버_재표시_시_오늘로_복귀한다", async () => {
    // 전환된 날짜(어제)에서 재표시하면 refreshTodos(오늘 조회)로 돌아온다
    let first = true;
    mockAppIPC({
      notion_todo_list: () => {
        const snap = first ? NOT_TODAY : LOADED;
        first = false;
        return snap;
      },
    });
    render(<App />);
    expect(
      await screen.findByRole("button", { name: "오늘로 돌아가기" }),
    ).toBeInTheDocument();

    document.dispatchEvent(new Event("visibilitychange"));

    await screen.findByRole("checkbox", { name: "아침 운동" });
    expect(
      screen.queryByRole("button", { name: "오늘로 돌아가기" }),
    ).not.toBeInTheDocument();
  });
});

describe("행 만들기 폼", () => {
  /** 폼을 열어 둔 카드를 렌더한다. */
  async function renderOpenForm(
    overrides: Partial<Parameters<typeof TodoCard>[0]> = {},
  ) {
    const result = renderCard(overrides);
    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    return result;
  }

  it("행_만들기_폼은_날짜만_받는다", async () => {
    const { props } = await renderOpenForm();

    // 기본 날짜는 스냅샷의 date — 폼에는 날짜 입력 하나뿐이다
    expect(screen.getByLabelText("날짜")).toHaveValue("2026-08-09");
    expect(screen.queryByRole("textbox", { name: "행 제목" })).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "만들기" }));
    await waitFor(() =>
      expect(props.onCreateRow).toHaveBeenCalledWith({ start: "2026-08-09" }),
    );
    // created → 폼 접힘
    await waitFor(() =>
      expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument(),
    );
  });

  it("날짜를_바꿔_제출하면_그_날짜가_전달된다", async () => {
    const { props } = await renderOpenForm();

    fireEvent.change(screen.getByLabelText("날짜"), {
      target: { value: "2026-08-15" },
    });
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    await waitFor(() =>
      expect(props.onCreateRow).toHaveBeenCalledWith({ start: "2026-08-15" }),
    );
  });

  it("빈_날짜는_제출이_비활성화된다", async () => {
    await renderOpenForm();

    fireEvent.change(screen.getByLabelText("날짜"), { target: { value: "" } });

    expect(screen.getByRole("button", { name: "만들기" })).toBeDisabled();
  });

  it("busy_중_폼_입력이_비활성화된다", async () => {
    const { props, view } = await renderOpenForm();

    view.rerender(<TodoCard {...props} isBusy={true} />);

    expect(screen.getByLabelText("날짜")).toBeDisabled();
    expect(screen.getByRole("button", { name: "만들기" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "취소" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "행 만들기" })).toBeDisabled();
  });

  it("Escape와_취소로_폼이_닫히고_재열림_시_초기화된다", async () => {
    await renderOpenForm();
    const date = screen.getByLabelText("날짜");
    fireEvent.change(date, { target: { value: "2026-08-20" } });

    // Escape는 폼 안(입력에 포커스)에서 눌러야 폼으로 버블된다
    await userEvent.click(date);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument();

    // 재열림 — 입력이 버려져 기본 날짜로 돌아온다
    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    expect(screen.getByLabelText("날짜")).toHaveValue("2026-08-09");

    // 취소 버튼도 폼을 닫는다
    await userEvent.click(screen.getByRole("button", { name: "취소" }));
    expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument();
  });

  it("행_만들기_실패_시_입력이_유지되고_재조회가_실행된다", async () => {
    const calls = mockAppIPC({
      notion_todo_create_row: () => {
        throw "행 생성에 실패했습니다";
      },
    });
    render(<App />);
    await screen.findByRole("checkbox", { name: "아침 운동" });

    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    fireEvent.change(screen.getByLabelText("날짜"), {
      target: { value: "2026-08-20" },
    });
    const listCallsBefore = calls.filter((c) => c.cmd === "notion_todo_list").length;
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    await screen.findByText("행 생성에 실패했습니다");
    // 타임아웃 뒤 실제로 생성됐다면 재조회가 그 행/exists 상태를 드러낸다 (R10)
    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(
        listCallsBefore + 1,
      ),
    );
    // 폼 입력값은 유지된다 — 재시도할 수 있어야 한다
    expect(screen.getByLabelText("날짜")).toHaveValue("2026-08-20");
  });

  it("생성_후_스냅샷_없음_응답은_기존_목록을_유지한다", async () => {
    // snapshot: null = 생성은 됐지만 재조회만 실패 — 기존 목록 유지 + 안내 배너
    const 안내 = "행은 생성됐지만 목록 조회에 실패했습니다. 새로고침해 주세요.";
    mockAppIPC({
      notion_todo_create_row: () => ({
        state: "created",
        snapshot: null,
        notice: 안내,
      }),
    });
    render(<App />);
    await screen.findByRole("checkbox", { name: "아침 운동" });

    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    await screen.findByText(안내);
    // 목록은 마지막 스냅샷 그대로 유지된다 (비워지지 않음)
    expect(screen.getByRole("checkbox", { name: "아침 운동" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "이메일 정리" })).toBeInTheDocument();
    // 생성 자체는 성공(created) — 폼은 접혀 중복 재시도를 막는다
    expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument();
  });

  it("exists_응답이_기존_행_열기_버튼을_보여준다", async () => {
    const onCreateRow = vi.fn(
      async (): Promise<CreateRowFormResult> => ({
        state: "exists",
        page_id: "page-9",
        title: "[TODO]",
        date: "2026-08-15",
        performance: "일부",
      }),
    );
    await renderOpenForm({ onCreateRow });

    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    expect(await screen.findByText("이미 있음: [TODO]")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "기존 행 열기" })).toBeInTheDocument();
    // 폼은 유지된다
    expect(screen.getByLabelText("날짜")).toHaveValue("2026-08-09");
  });

  it("열기_클릭이_open_page를_호출한다", async () => {
    const onCreateRow = vi.fn(
      async (): Promise<CreateRowFormResult> => ({
        state: "exists",
        page_id: "page-9",
        title: "[TODO]",
        date: "2026-08-15",
        performance: "일부",
      }),
    );
    const onOpenPage = vi.fn(async (): Promise<boolean> => true);
    await renderOpenForm({ onCreateRow, onOpenPage });
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    await userEvent.click(await screen.findByRole("button", { name: "기존 행 열기" }));

    // exists가 실어 준 수행도가 열기 호출까지 그대로 흘러야 한다 (KTD1)
    expect(onOpenPage).toHaveBeenCalledWith("page-9", "[TODO]", "2026-08-15", "일부");
    // 열기 성공 → 폼 접힘
    await waitFor(() =>
      expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument(),
    );
  });
});

describe("날짜 전환", () => {
  it("오늘이_아닌_스냅샷에서_날짜와_돌아가기_버튼이_보인다", () => {
    renderCard({ snapshot: NOT_TODAY });

    expect(screen.getByText("2026-08-08")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "오늘로 돌아가기" })).toBeInTheDocument();
  });

  it("오늘이_아닌_no_page에서는_만들기_버튼이_숨겨진다", () => {
    renderCard({
      snapshot: { state: "no_page", date: "2026-08-15", is_today: false },
    });

    expect(
      screen.queryByRole("button", { name: "오늘 페이지 만들기" }),
    ).not.toBeInTheDocument();
    // 대신 날짜 표시 + 돌아가기만 보인다
    expect(screen.getByText("2026-08-15")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "오늘로 돌아가기" })).toBeInTheDocument();
  });

  it("돌아가기_클릭이_오늘_목록을_재조회한다", async () => {
    const { props } = renderCard({ snapshot: NOT_TODAY });

    await userEvent.click(screen.getByRole("button", { name: "오늘로 돌아가기" }));

    expect(props.onRefresh).toHaveBeenCalled();
  });

  it("쓰기_핸들러가_스냅샷_날짜를_전달한다", async () => {
    const calls = mockAppIPC({
      notion_todo_list: () => NOT_TODAY,
      notion_todo_add: () => outcome(NOT_TODAY),
    });
    render(<App />);

    const input = await screen.findByRole("textbox", { name: "새 할 일" });
    await userEvent.type(input, "쉬기 준비");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));

    await waitFor(() => {
      const call = calls.find((c) => c.cmd === "notion_todo_add");
      expect(call).toBeDefined();
      expect(call!.args).toMatchObject({
        pageId: "page-0",
        pageTitle: "[TODO]",
        text: "쉬기 준비",
        date: "2026-08-08",
      });
    });
  });

  it("전환_중_쓰기_실패가_오늘로_되돌리지_않는다", async () => {
    const calls = mockAppIPC({
      notion_todo_list: () => NOT_TODAY,
      notion_todo_add: () => {
        throw "일시 오류";
      },
      notion_todo_open_page: () => outcome(NOT_TODAY),
    });
    render(<App />);

    const input = await screen.findByRole("textbox", { name: "새 할 일" });
    const listCallsBefore = calls.filter((c) => c.cmd === "notion_todo_list").length;
    await userEvent.type(input, "새 항목");
    await userEvent.click(screen.getByRole("button", { name: "추가" }));

    await screen.findByText("일시 오류");
    // 실패 재조회가 오늘(getTodoList)이 아니라 전환된 날짜(openTodoPage)로 간다
    await waitFor(() => {
      const open = calls.find((c) => c.cmd === "notion_todo_open_page");
      expect(open).toBeDefined();
      expect(open!.args).toMatchObject({
        pageId: "page-0",
        pageTitle: "[TODO]",
        date: "2026-08-08",
      });
    });
    expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(
      listCallsBefore,
    );
    expect(screen.getByText("2026-08-08")).toBeInTheDocument();
  });

  it("생성_성공_시_폼이_접히고_스냅샷이_전환된다", async () => {
    const FUTURE: LoadedSnapshot = {
      state: "loaded",
      date: "2026-08-15",
      page_id: "page-7",
      title: "[TODO]",
      items: [],
      is_today: false,
      performance: null,
      range_start: null,
      range_end: null,
    };
    const calls = mockAppIPC({
      notion_todo_create_row: () => ({
        state: "created",
        snapshot: FUTURE,
        notice: null,
      }),
    });
    render(<App />);
    await screen.findByRole("checkbox", { name: "아침 운동" });

    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    fireEvent.change(screen.getByLabelText("날짜"), {
      target: { value: "2026-08-15" },
    });
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));

    const call = calls.find((c) => c.cmd === "notion_todo_create_row");
    expect(call).toBeDefined();
    expect(call!.args).toEqual({ start: "2026-08-15" });
    await waitFor(() =>
      expect(screen.queryByLabelText("날짜")).not.toBeInTheDocument(),
    );
    // 생성된 날짜로 전환됨 — 날짜 표시 + 돌아가기
    expect(await screen.findByText("2026-08-15")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "오늘로 돌아가기" })).toBeInTheDocument();
  });
});

describe("수행도", () => {
  const PERFORMANCES = ["완료", "일부", "미완", "기타"];

  it("현재_수행도가_선택된_상태로_보인다", () => {
    renderCard({ snapshot: loaded({ performance: "일부" }) });

    // 달성도 순(완료·일부·미완·기타) 4개만 있고 현재 값만 눌려 있다 (KTD2)
    const buttons = within(perfGroup()).getAllByRole("button");
    expect(buttons.map((b) => b.textContent)).toEqual(PERFORMANCES);
    for (const value of PERFORMANCES) {
      expect(perfButton(value)).toHaveAttribute(
        "aria-pressed",
        String(value === "일부"),
      );
    }
    // 값이 있으면 미지정 라벨은 없다
    expect(screen.queryByText("미지정")).not.toBeInTheDocument();
  });

  it("값이_없으면_미지정으로_보이고_아무것도_선택되지_않는다", () => {
    renderCard(); // LOADED — performance null

    expect(screen.getByText("미지정")).toBeInTheDocument();
    for (const value of PERFORMANCES) {
      expect(perfButton(value)).toHaveAttribute("aria-pressed", "false");
    }
  });

  it("버튼을_누르면_그_값으로_커맨드가_호출된다", async () => {
    const { props } = renderCard({ snapshot: loaded({ performance: "미완" }) });

    await userEvent.click(perfButton("완료"));

    expect(props.onSetPerformance).toHaveBeenCalledWith("완료");
  });

  it("같은_값을_다시_누르면_커맨드를_호출하지_않는다", async () => {
    const { props } = renderCard({ snapshot: loaded({ performance: "완료" }) });

    await userEvent.click(perfButton("완료"));

    expect(props.onSetPerformance).not.toHaveBeenCalled();
  });

  it("busy_중에는_수행도_버튼이_비활성이다", () => {
    renderCard({ isBusy: true, snapshot: loaded({ performance: "완료" }) });

    for (const value of PERFORMANCES) {
      expect(perfButton(value)).toBeDisabled();
    }
  });

  it("범위_행에서는_적용_구간이_함께_표시된다", () => {
    // 8/13을 보고 있고 그 날짜를 덮는 행이 8/12~8/14인 경우 (AE8) —
    // 끝만 적으면 이미 지난 8/12까지 함께 바뀐다는 사실이 감춰진다
    renderCard({
      snapshot: loaded({
        date: "2026-08-13",
        is_today: false,
        performance: "기타",
        range_start: "2026-08-12",
        range_end: "2026-08-14",
      }),
    });

    expect(screen.getByText("2026-08-12~2026-08-14 적용")).toBeInTheDocument();
  });

  it("하루_행에서는_적용_구간을_표시하지_않는다", () => {
    renderCard({ snapshot: loaded({ performance: "완료" }) });

    expect(screen.queryByText(/적용/)).not.toBeInTheDocument();
  });

  it("시작일과_끝이_같은_행은_적용_구간을_표시하지_않는다", () => {
    // 시작·끝이 모두 실려도 같은 날이면 하루 행이다 — 구간을 그리면 거짓 정보다
    renderCard({
      snapshot: loaded({
        date: "2026-08-13",
        is_today: false,
        performance: "완료",
        range_start: "2026-08-13",
        range_end: "2026-08-13",
      }),
    });

    expect(screen.queryByText(/적용/)).not.toBeInTheDocument();
  });

  it("not_connected와_no_page에서는_수행도_줄이_보이지_않는다", () => {
    const { view } = renderCard({ snapshot: NOT_CONNECTED });
    expect(screen.queryByRole("group", { name: "수행도" })).not.toBeInTheDocument();
    view.unmount();

    renderCard({ snapshot: NO_PAGE });
    expect(screen.queryByRole("group", { name: "수행도" })).not.toBeInTheDocument();
  });

  it("오늘이_아닌_스냅샷에서도_수행도_줄이_보이고_그_날짜로_호출한다", async () => {
    // NOT_TODAY — 8/8 행, 현재 수행도 "일부" (AE5)
    const calls = mockAppIPC({
      notion_todo_list: () => NOT_TODAY,
      notion_todo_set_performance: () =>
        outcome({ ...NOT_TODAY, performance: "미완" }),
    });
    render(<App />);

    await screen.findByRole("checkbox", { name: "쉬기" });
    expect(perfButton("일부")).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(perfButton("미완"));

    await waitFor(() => {
      const call = calls.find((c) => c.cmd === "notion_todo_set_performance");
      expect(call).toBeDefined();
      expect(call!.args).toMatchObject({
        pageId: "page-0",
        pageTitle: "[TODO]",
        date: "2026-08-08",
        performance: "미완",
        // 저장이 확인되지 않은 경로에서 되실을 직전 값 (R9)
        meta: { performance: "일부" },
      });
    });
    // 카드는 그 날짜에 머문다
    await waitFor(() => expect(perfButton("미완")).toHaveAttribute("aria-pressed", "true"));
    expect(screen.getByText("2026-08-08")).toBeInTheDocument();
  });

  it("오늘_화면의_수행도_쓰기_실패가_배너를_띄우고_재조회한다", async () => {
    // AE6 — 오늘 화면(LOADED, 미지정)에서의 실패는 오늘 목록을 1회 재조회한다
    const calls = mockAppIPC({
      notion_todo_set_performance: () => {
        throw "수행도 저장에 실패했습니다";
      },
    });
    render(<App />);

    await screen.findByRole("checkbox", { name: "아침 운동" });
    const listCallsBefore = calls.filter((c) => c.cmd === "notion_todo_list").length;
    await userEvent.click(perfButton("완료"));

    await screen.findByText("수행도 저장에 실패했습니다");
    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "notion_todo_list").length).toBe(
        listCallsBefore + 1,
      ),
    );
    // 시도값이 선택된 채로 남지 않는다 — 재조회 결과(미지정) 그대로다 (R9)
    expect(perfButton("완료")).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("미지정")).toBeInTheDocument();
  });

  it("전환된_날짜의_쓰기_실패는_직전_수행도를_유지한다", async () => {
    // AE7 — 실패 재조회(openTodoPage)가 시도값이 아니라 직전 값을 되싣는다 (R9).
    // 백엔드는 받은 값을 그대로 에코하므로, 화면에 남는 값이 곧 넘긴 값이다.
    const calls = mockAppIPC({
      notion_todo_list: () => NOT_TODAY,
      notion_todo_set_performance: () => {
        throw "일시 오류";
      },
      notion_todo_open_page: (args) =>
        outcome({
          ...NOT_TODAY,
          performance: metaPerformance(args),
        }),
    });
    render(<App />);

    await screen.findByRole("checkbox", { name: "쉬기" });
    await userEvent.click(perfButton("완료"));

    await screen.findByText("일시 오류");
    await waitFor(() => {
      const open = calls.find((c) => c.cmd === "notion_todo_open_page");
      expect(open).toBeDefined();
      expect(open!.args).toMatchObject({
        pageId: "page-0",
        date: "2026-08-08",
        // 시도값("완료")이 아니라 직전 값
        meta: { performance: "일부" },
      });
    });
    expect(perfButton("일부")).toHaveAttribute("aria-pressed", "true");
    expect(perfButton("완료")).toHaveAttribute("aria-pressed", "false");
  });

  it("목록_쓰기도_현재_수행도를_함께_보낸다", async () => {
    // children 재조회는 페이지 메타를 주지 않는다 — 토글 후에도 값이 남으려면
    // 프론트가 현재 값을 되실어 줘야 한다 (KTD1)
    const RANGE = loaded({
      performance: "일부",
      range_start: "2026-08-12",
      range_end: "2026-08-14",
    });
    const calls = mockAppIPC({
      notion_todo_list: () => RANGE,
      notion_todo_toggle: () => outcome(RANGE),
    });
    render(<App />);

    await userEvent.click(await screen.findByRole("checkbox", { name: "보고서 작성" }));

    await waitFor(() => {
      const call = calls.find((c) => c.cmd === "notion_todo_toggle");
      expect(call).toBeDefined();
      expect(call!.args).toMatchObject({
        meta: {
          performance: "일부",
          // 적용 구간은 시작·끝 둘 다 되실려야 토글 후에도 구간 표시가 남는다
          rangeStart: "2026-08-12",
          rangeEnd: "2026-08-14",
        },
      });
    });
    expect(perfButton("일부")).toHaveAttribute("aria-pressed", "true");
  });

  it("exists로_연_행의_수행도가_보인다", async () => {
    // 기존 행 열기 — exists가 실어 준 수행도가 열기 커맨드까지 흘러 화면에 보인다
    const calls = mockAppIPC({
      notion_todo_create_row: () => ({
        state: "exists",
        page_id: "page-9",
        title: "휴가",
        date: "2026-08-15",
        performance: "기타",
      }),
      notion_todo_open_page: (args) =>
        outcome({
          state: "loaded",
          date: "2026-08-15",
          page_id: "page-9",
          title: "휴가",
          items: [],
          is_today: false,
          performance: metaPerformance(args),
          range_start: null,
          range_end: null,
        }),
    });
    render(<App />);
    await screen.findByRole("checkbox", { name: "아침 운동" });

    await userEvent.click(screen.getByRole("button", { name: "행 만들기" }));
    fireEvent.change(screen.getByLabelText("날짜"), {
      target: { value: "2026-08-15" },
    });
    await userEvent.click(screen.getByRole("button", { name: "만들기" }));
    await userEvent.click(await screen.findByRole("button", { name: "기존 행 열기" }));

    await waitFor(() => {
      const open = calls.find((c) => c.cmd === "notion_todo_open_page");
      expect(open).toBeDefined();
      expect(open!.args).toMatchObject({
        pageId: "page-9",
        meta: { performance: "기타" },
      });
    });
    await waitFor(() => expect(perfButton("기타")).toHaveAttribute("aria-pressed", "true"));
  });
});
