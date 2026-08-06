import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "focus" | "break";

export type TimerSnapshot =
  | { state: "idle" }
  | { state: "running"; phase: Phase; remaining_ms: number }
  | { state: "paused"; phase: Phase; remaining_ms: number }
  | { state: "finished"; phase: Phase };

export interface TimerConfig {
  focus_minutes: number;
  break_minutes: number;
}

export const EVENT_TICK = "pomodoro://tick";
export const EVENT_FINISHED = "pomodoro://finished";

export const startTimer = (phase: Phase): Promise<TimerSnapshot> =>
  invoke("timer_start", { phase });

export const pauseTimer = (): Promise<TimerSnapshot> => invoke("timer_pause");

export const resumeTimer = (): Promise<TimerSnapshot> => invoke("timer_resume");

export const resetTimer = (): Promise<TimerSnapshot> => invoke("timer_reset");

export const getTimerState = (): Promise<TimerSnapshot> => invoke("timer_get_state");

export const getTimerConfig = (): Promise<TimerConfig> => invoke("timer_get_config");

export const setTimerConfig = (config: TimerConfig): Promise<TimerConfig> =>
  invoke("timer_set_config", {
    focusMinutes: config.focus_minutes,
    breakMinutes: config.break_minutes,
  });

/** 교대 규칙: 집중이 끝나면 휴식, 휴식이 끝나면 집중. */
export const nextPhase = (finished: Phase): Phase =>
  finished === "focus" ? "break" : "focus";

/** 남은 ms → "MM:SS" (초 올림 — Rust format_mmss와 동일 규칙). */
export const formatMmss = (ms: number): string => {
  const totalSecs = Math.ceil(ms / 1000);
  const mm = String(Math.floor(totalSecs / 60)).padStart(2, "0");
  const ss = String(totalSecs % 60).padStart(2, "0");
  return `${mm}:${ss}`;
};

export const onTick = (cb: (snapshot: TimerSnapshot) => void): Promise<UnlistenFn> =>
  listen<TimerSnapshot>(EVENT_TICK, (event) => cb(event.payload));

export const onFinished = (cb: (phase: Phase) => void): Promise<UnlistenFn> =>
  listen<Phase>(EVENT_FINISHED, (event) => cb(event.payload));
