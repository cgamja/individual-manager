import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { ConnectionState } from "../lib/notion";
import { NotionCard } from "./NotionCard";

afterEach(() => {
  cleanup();
  clearMocks();
  Reflect.deleteProperty(window, "Notification");
});

const NOT_CONFIGURED_BOTH: ConnectionState = { state: "not_configured", missing: "both" };
const CONNECTED: ConnectionState = { state: "connected", title: "가짜 DB" };

/** NotionCard를 기본 props + 오버라이드로 렌더한다. */
function renderCard(overrides: Partial<Parameters<typeof NotionCard>[0]> = {}) {
  const props = {
    status: NOT_CONFIGURED_BOTH,
    isVerifying: false,
    onSaveToken: vi.fn(async (): Promise<ConnectionState> => CONNECTED),
    onDeleteToken: vi.fn(),
    onSetDatabase: vi.fn(),
    onTestConnection: vi.fn(),
    ...overrides,
  };
  const view = render(<NotionCard {...props} />);
  return { props, view };
}

/**
 * App 통합 테스트용 IPC mock — 타이머/스토어/이벤트 커맨드는 기본 응답을 주고,
 * notion 커맨드는 handlers로 오버라이드한다. 알림 권한은 Notification 심으로 통과시킨다.
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
        return NOT_CONFIGURED_BOTH;
      default:
        return undefined;
    }
  });
  return calls;
}

describe("토큰 저장 플로", () => {
  it("토큰_저장은_올바른_커맨드와_인자로_invoke된다", async () => {
    const calls = mockAppIPC({
      notion_save_token: () => ({ state: "not_configured", missing: "database" }),
    });
    render(<App />);

    const tokenInput = await screen.findByLabelText("Integration 토큰");
    await userEvent.type(tokenInput, "ntn_fake_token_123");
    await userEvent.click(screen.getByRole("button", { name: "토큰 저장" }));

    const call = calls.find((c) => c.cmd === "notion_save_token");
    expect(call).toBeDefined();
    expect(call!.args).toMatchObject({ token: "ntn_fake_token_123" });
  });

  it("검증_성공_시에만_입력_필드가_비워진다", async () => {
    const { props } = renderCard({
      onSaveToken: vi.fn(async (): Promise<ConnectionState> => CONNECTED),
    });

    const tokenInput = screen.getByLabelText("Integration 토큰");
    await userEvent.type(tokenInput, "ntn_fake_token_123");
    await userEvent.click(screen.getByRole("button", { name: "토큰 저장" }));

    expect(props.onSaveToken).toHaveBeenCalledWith("ntn_fake_token_123");
    await waitFor(() => expect(tokenInput).toHaveValue(""));
  });

  it("실패_시_입력값이_유지된다", async () => {
    const failed: ConnectionState = { state: "failed", message: "인증에 실패했어요" };
    const { props } = renderCard({
      onSaveToken: vi.fn(async (): Promise<ConnectionState> => failed),
    });

    const tokenInput = screen.getByLabelText("Integration 토큰");
    await userEvent.type(tokenInput, "ntn_fake_token_123");
    await userEvent.click(screen.getByRole("button", { name: "토큰 저장" }));

    await waitFor(() => expect(props.onSaveToken).toHaveBeenCalled());
    expect(tokenInput).toHaveValue("ntn_fake_token_123");
  });

  it("커맨드가_reject돼도_입력값이_유지된다", async () => {
    const { props } = renderCard({
      onSaveToken: vi.fn(async (): Promise<ConnectionState> => {
        throw "Keychain 저장에 실패했어요";
      }),
    });

    const tokenInput = screen.getByLabelText("Integration 토큰");
    await userEvent.type(tokenInput, "ntn_fake_token_123");
    await userEvent.click(screen.getByRole("button", { name: "토큰 저장" }));

    await waitFor(() => expect(props.onSaveToken).toHaveBeenCalled());
    expect(tokenInput).toHaveValue("ntn_fake_token_123");
  });
});

describe("DB 지정 플로", () => {
  it("DB_입력_저장이_상태를_갱신한다", async () => {
    mockAppIPC({
      notion_set_database: () => CONNECTED,
    });
    render(<App />);

    const dbInput = await screen.findByLabelText("Database URL/ID");
    await userEvent.type(dbInput, "00000000000000000000000000000abc");
    await userEvent.click(screen.getByRole("button", { name: "Database 저장" }));

    expect(await screen.findByText(/가짜 DB/)).toBeInTheDocument();
  });
});

describe("상태 표시", () => {
  it("실패_상태는_원인_메시지를_표시한다", () => {
    renderCard({
      status: {
        state: "failed",
        message: "Database를 찾을 수 없어요 (404) — 커넥터 연결을 확인해 주세요",
      },
    });

    expect(screen.getByText(/404/)).toBeInTheDocument();
    expect(screen.getByText(/커넥터 연결을 확인해 주세요/)).toBeInTheDocument();
  });

  it("미설정_상태는_안내_문구를_표시한다", () => {
    const { view: v1 } = renderCard({
      status: { state: "not_configured", missing: "token" },
    });
    expect(screen.getByText("토큰을 입력해 주세요")).toBeInTheDocument();
    v1.unmount();

    const { view: v2 } = renderCard({
      status: { state: "not_configured", missing: "database" },
    });
    expect(screen.getByText("Database를 지정해 주세요")).toBeInTheDocument();
    v2.unmount();

    renderCard({ status: { state: "not_configured", missing: "both" } });
    expect(screen.getByText("토큰과 Database를 입력해 주세요")).toBeInTheDocument();
  });

  it("연결됨_상태는_DB_제목을_표시한다", () => {
    renderCard({ status: CONNECTED });
    expect(screen.getByText(/가짜 DB/)).toBeInTheDocument();
  });
});

describe("상태별 컨트롤 노출", () => {
  it("상태별_컨트롤_노출이_올바르다", () => {
    // 토큰 미저장 → 삭제 버튼·저장됨 배지 없음
    const { view: v1 } = renderCard({
      status: { state: "not_configured", missing: "token" },
    });
    expect(screen.queryByRole("button", { name: "삭제" })).not.toBeInTheDocument();
    expect(screen.queryByText("저장됨")).not.toBeInTheDocument();
    v1.unmount();

    // 토큰 저장됨(연결됨) → 저장됨 배지 + 삭제 버튼
    const { view: v2 } = renderCard({ status: CONNECTED });
    expect(screen.getByText("저장됨")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "삭제" })).toBeInTheDocument();
    v2.unmount();

    // isVerifying → 세 트리거 모두 disabled + "확인 중..." 표시
    renderCard({ isVerifying: true });
    expect(screen.getByRole("button", { name: "토큰 저장" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Database 저장" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "연결 테스트" })).toBeDisabled();
    expect(screen.getByText("확인 중...")).toBeInTheDocument();
  });
});
