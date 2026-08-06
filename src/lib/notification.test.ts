import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ensureNotificationPermission } from "./notification";

// Tauri 런타임은 window.Notification 심을 주입한다 — 플러그인 JS는
// permission을 먼저 보고, "default"일 때만 IPC로 조회한다.
function stubNotification(permission: string, requestResult = permission) {
  const requestPermission = vi.fn(async () => requestResult);
  Object.defineProperty(window, "Notification", {
    configurable: true,
    value: { permission, requestPermission },
  });
  return { requestPermission };
}

afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(window, "Notification");
});

describe("알림 권한 플로", () => {
  it("이미_허용된_경우_요청_없이_true를_반환한다", async () => {
    const { requestPermission } = stubNotification("granted");

    expect(await ensureNotificationPermission()).toBe(true);
    expect(requestPermission).not.toHaveBeenCalled();
  });

  it("미결정이면_요청하고_거부돼도_에러_없이_false를_반환한다", async () => {
    const { requestPermission } = stubNotification("default", "denied");
    mockIPC((cmd) => {
      if (cmd === "plugin:notification|is_permission_granted") return false;
      return undefined;
    });

    expect(await ensureNotificationPermission()).toBe(false);
    expect(requestPermission).toHaveBeenCalledOnce();
  });

  it("권한_API가_없어도_에러_없이_false로_처리한다", async () => {
    // window.Notification 자체가 없는 환경 — 예외를 삼키고 false
    expect(await ensureNotificationPermission()).toBe(false);
  });
});
