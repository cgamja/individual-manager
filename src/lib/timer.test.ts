import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { formatMmss, nextPhase, setTimerConfig, startTimer } from "./timer";

afterEach(() => {
  clearMocks();
});

describe("타이머 커맨드 래퍼", () => {
  it("start는_올바른_command와_phase_인자로_invoke된다", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return { state: "running", phase: "focus", remaining_ms: 1_500_000 };
    });

    const snapshot = await startTimer("focus");

    expect(calls).toHaveLength(1);
    expect(calls[0].cmd).toBe("timer_start");
    expect(calls[0].args).toMatchObject({ phase: "focus" });
    expect(snapshot).toEqual({
      state: "running",
      phase: "focus",
      remaining_ms: 1_500_000,
    });
  });

  it("set_config는_분_단위_인자를_camelCase로_전달한다", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return { focus_minutes: 50, break_minutes: 10 };
    });

    await setTimerConfig({ focus_minutes: 50, break_minutes: 10 });

    expect(calls[0].cmd).toBe("timer_set_config");
    expect(calls[0].args).toMatchObject({ focusMinutes: 50, breakMinutes: 10 });
  });
});

describe("교대 규칙", () => {
  it("집중이_끝나면_다음은_휴식이고_휴식이_끝나면_다음은_집중이다", () => {
    expect(nextPhase("focus")).toBe("break");
    expect(nextPhase("break")).toBe("focus");
  });
});

describe("남은 시간 포맷", () => {
  it("mmss_포맷과_한_자리_초_패딩을_지킨다", () => {
    expect(formatMmss(1_500_000)).toBe("25:00");
    expect(formatMmss(61_000)).toBe("01:01");
    expect(formatMmss(9_000)).toBe("00:09");
    expect(formatMmss(0)).toBe("00:00");
    expect(formatMmss(5_400_000)).toBe("90:00");
  });
});
