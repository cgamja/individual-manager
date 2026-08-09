import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import {
  deleteNotionToken,
  getNotionStatus,
  saveNotionToken,
  setNotionDatabase,
  testNotionConnection,
  type ConnectionState,
} from "./notion";

afterEach(() => {
  clearMocks();
});

/** IPC 호출을 기록하고 고정 응답을 돌려준다. */
function captureIPC(response: ConnectionState) {
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
