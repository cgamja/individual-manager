import { useCallback, useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { SettingsCard } from "./components/SettingsCard";
import { TimerCard } from "./components/TimerCard";
import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "./lib/settings";
import {
  getTimerState,
  onTick,
  pauseTimer,
  resetTimer,
  resumeTimer,
  setTimerConfig,
  startTimer,
  type Phase,
  type TimerConfig,
  type TimerSnapshot,
} from "./lib/timer";
import "./App.css";

function App() {
  const [snapshot, setSnapshot] = useState<TimerSnapshot>({ state: "idle" });
  const [config, setConfig] = useState<TimerConfig>(DEFAULT_SETTINGS);

  useEffect(() => {
    let unlistenTick: UnlistenFn | undefined;
    let cancelled = false;

    (async () => {
      // 저장된 설정을 Rust 코어에 반영한 뒤 상태를 동기화한다
      const saved = await loadSettings().catch(() => DEFAULT_SETTINGS);
      const applied = await setTimerConfig(saved).catch(() => DEFAULT_SETTINGS);
      if (cancelled) return;
      setConfig(applied);
      setSnapshot(await getTimerState());
      unlistenTick = await onTick((s) => setSnapshot(s));
    })();

    // 팝오버가 다시 보일 때 즉시 재동기화 (틱 대기 없이)
    const onVisibility = () => {
      if (!document.hidden) {
        getTimerState().then(setSnapshot).catch(() => {});
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      cancelled = true;
      unlistenTick?.();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  const handleStart = useCallback((phase: Phase) => {
    startTimer(phase).then(setSnapshot).catch(() => {});
  }, []);
  const handlePause = useCallback(() => {
    pauseTimer().then(setSnapshot).catch(() => {});
  }, []);
  const handleResume = useCallback(() => {
    resumeTimer().then(setSnapshot).catch(() => {});
  }, []);
  const handleReset = useCallback(() => {
    resetTimer().then(setSnapshot).catch(() => {});
  }, []);

  const handleConfigChange = useCallback(async (next: TimerConfig) => {
    try {
      const applied = await setTimerConfig(next);
      setConfig(applied);
      await saveSettings(applied);
    } catch {
      // 검증 실패(0분 등)는 무시 — 입력단에서 이미 걸러진다
    }
  }, []);

  return (
    <main className="popover">
      <TimerCard
        snapshot={snapshot}
        onStart={handleStart}
        onPause={handlePause}
        onResume={handleResume}
        onReset={handleReset}
      />
      <SettingsCard
        config={config}
        disabled={snapshot.state !== "idle"}
        onChange={handleConfigChange}
      />
    </main>
  );
}

export default App;
