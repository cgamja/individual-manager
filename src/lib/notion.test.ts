import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import {
  addTodo,
  createTodoPage,
  createTodoRow,
  deleteNotionToken,
  editTodo,
  getNotionStatus,
  getTodoList,
  openTodoPage,
  saveNotionToken,
  setNotionDatabase,
  testNotionConnection,
  toggleTodo,
  type CreateRowOutcome,
  type TodoOutcome,
  type TodoSnapshot,
} from "./notion";

afterEach(() => {
  clearMocks();
});

/** IPC 호출을 기록하고 고정 응답을 돌려준다. */
function captureIPC<T>(response: T) {
  const calls: Array<{ cmd: string; args: unknown }> = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return response;
  });
  return calls;
}

describe("Notion 커맨드 래퍼", () => {
  it("save_token은_notion_save_token에_token_인자를_전달한다", async () => {
    const calls = captureIPC({ state: "not_configured", missing: "database" });

    const result = await saveNotionToken("ntn_fake_token_123");

    expect(calls).toHaveLength(1);
    expect(calls[0].cmd).toBe("notion_save_token");
    expect(calls[0].args).toMatchObject({ token: "ntn_fake_token_123" });
    expect(result).toEqual({ state: "not_configured", missing: "database" });
  });

  it("set_database는_notion_set_database에_input_인자를_전달한다", async () => {
    const calls = captureIPC({ state: "connected", title: "가짜 DB" });

    const result = await setNotionDatabase(
      "https://www.notion.so/fakeworkspace/00000000000000000000000000000abc",
    );

    expect(calls[0].cmd).toBe("notion_set_database");
    expect(calls[0].args).toMatchObject({
      input: "https://www.notion.so/fakeworkspace/00000000000000000000000000000abc",
    });
    expect(result).toEqual({ state: "connected", title: "가짜 DB" });
  });

  it("delete_token은_notion_delete_token을_invoke한다", async () => {
    const calls = captureIPC({ state: "not_configured", missing: "both" });

    await deleteNotionToken();

    expect(calls[0].cmd).toBe("notion_delete_token");
  });

  it("get_status는_notion_get_status를_invoke하고_상태를_반환한다", async () => {
    const calls = captureIPC({ state: "connected", title: "가짜 DB" });

    const result = await getNotionStatus();

    expect(calls[0].cmd).toBe("notion_get_status");
    expect(result).toEqual({ state: "connected", title: "가짜 DB" });
  });

  it("test_connection은_notion_test_connection을_invoke한다", async () => {
    const calls = captureIPC({ state: "failed", message: "인증에 실패했어요" });

    const result = await testNotionConnection();

    expect(calls[0].cmd).toBe("notion_test_connection");
    expect(result).toEqual({ state: "failed", message: "인증에 실패했어요" });
  });
});

describe("Todo 커맨드 래퍼", () => {
  const SNAPSHOT: TodoSnapshot = {
    state: "loaded",
    date: "2026-08-09",
    page_id: "page-1",
    title: "[TODO]",
    items: [{ id: "b1", text: "가짜 항목", checked: false, category: null }],
    is_today: true,
  };
  const OUTCOME: TodoOutcome = { snapshot: SNAPSHOT, notice: null };

  it("todo_list는_notion_todo_list를_invoke하고_스냅샷을_반환한다", async () => {
    const calls = captureIPC(SNAPSHOT);

    const result = await getTodoList();

    expect(calls[0].cmd).toBe("notion_todo_list");
    expect(result).toEqual(SNAPSHOT);
  });

  it("create_page는_notion_todo_create_page를_invoke하고_결과를_반환한다", async () => {
    const calls = captureIPC(OUTCOME);

    const result = await createTodoPage();

    expect(calls[0].cmd).toBe("notion_todo_create_page");
    expect(result).toEqual(OUTCOME);
  });

  it("add는_notion_todo_add에_camelCase_인자를_전달한다", async () => {
    const calls = captureIPC(OUTCOME);

    await addTodo("page-1", "새 할 일", "[TODO]");

    expect(calls[0].cmd).toBe("notion_todo_add");
    expect(calls[0].args).toMatchObject({
      pageId: "page-1",
      text: "새 할 일",
      pageTitle: "[TODO]",
    });
  });

  it("toggle은_notion_todo_toggle에_blockId와_checked를_전달한다", async () => {
    const calls = captureIPC(OUTCOME);

    await toggleTodo("page-1", "b1", true, "[TODO]");

    expect(calls[0].cmd).toBe("notion_todo_toggle");
    expect(calls[0].args).toMatchObject({
      pageId: "page-1",
      blockId: "b1",
      checked: true,
      pageTitle: "[TODO]",
    });
  });

  it("edit은_notion_todo_edit에_blockId와_text를_전달한다", async () => {
    const calls = captureIPC(OUTCOME);

    await editTodo("page-1", "b1", "수정된 텍스트", "[TODO]");

    expect(calls[0].cmd).toBe("notion_todo_edit");
    expect(calls[0].args).toMatchObject({
      pageId: "page-1",
      blockId: "b1",
      text: "수정된 텍스트",
      pageTitle: "[TODO]",
    });
  });

  it("create_row는_start만_전달한다", async () => {
    const CREATED: CreateRowOutcome = {
      state: "created",
      snapshot: SNAPSHOT,
      notice: null,
    };
    const calls = captureIPC(CREATED);

    const result = await createTodoRow({ start: "2026-08-10" });

    expect(calls[0].cmd).toBe("notion_todo_create_row");
    // 행 생성은 미래 [TODO] 전용 — 날짜 외 인자는 실리지 않는다
    expect(calls[0].args).toEqual({ start: "2026-08-10" });
    expect(result).toEqual(CREATED);
  });

  it("add는_category를_전달하거나_생략한다", async () => {
    const calls = captureIPC(OUTCOME);

    // category 전달 → args에 포함
    await addTodo("page-1", "새 할 일", "[TODO]", "2026-08-10", "기타");
    expect(calls[0].args).toMatchObject({ category: "기타" });

    // 미전달 → undefined (백엔드는 끝에 append)
    await addTodo("page-1", "새 할 일", "[TODO]");
    expect(calls[1].args).toMatchObject({ category: undefined });
  });

  it("open_page_인자가_커맨드에_그대로_전달된다", async () => {
    const calls = captureIPC(OUTCOME);

    const result = await openTodoPage("page-9", "휴가", "2026-08-10");

    expect(calls[0].cmd).toBe("notion_todo_open_page");
    expect(calls[0].args).toMatchObject({
      pageId: "page-9",
      pageTitle: "휴가",
      date: "2026-08-10",
    });
    expect(result).toEqual(OUTCOME);
  });

  it("쓰기_래퍼가_날짜를_전달한다", async () => {
    const calls = captureIPC(OUTCOME);

    // date 전달 → args에 date 포함
    await addTodo("page-1", "새 할 일", "[TODO]", "2026-08-10");
    expect(calls[0].args).toMatchObject({ date: "2026-08-10" });

    // 미전달 → undefined
    await addTodo("page-1", "새 할 일", "[TODO]");
    expect(calls[1].args).toMatchObject({ date: undefined });

    await toggleTodo("page-1", "b1", true, "[TODO]", "2026-08-11");
    expect(calls[2].args).toMatchObject({ date: "2026-08-11" });

    await editTodo("page-1", "b1", "수정", "[TODO]", "2026-08-12");
    expect(calls[3].args).toMatchObject({ date: "2026-08-12" });
  });
});
