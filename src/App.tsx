import { useCallback, useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { NotionCard } from "./components/NotionCard";
import { SettingsCard } from "./components/SettingsCard";
import { TimerCard } from "./components/TimerCard";
import { ensureNotificationPermission } from "./lib/notification";
import {
  deleteNotionToken,
  getNotionStatus,
  saveNotionToken,
  setNotionDatabase,
  testNotionConnection,
  type ConnectionState,
} from "./lib/notion";
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
  const [notifGranted, setNotifGranted] = useState(true);
  const [saveFailed, setSaveFailed] = useState(false);
  const [notionStatus, setNotionStatus] = useState<ConnectionState>({
    state: "not_configured",
    missing: "both",
  });
  const [notionVerifying, setNotionVerifying] = useState(false);
  const [notionError, setNotionError] = useState<string | null>(null);

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
      // Notion 연결 상태 로드 (네트워크 없이 저장된 설정만 본다) —
      // 첫 실행의 알림 권한 프롬프트 대기에 막히지 않게 먼저 로드한다
      const notion = await getNotionStatus().catch(() => null);
      if (!cancelled && notion) setNotionStatus(notion);
      // 알림 권한: 거부돼도 앱은 계속 동작하고 카드 내 표시로 대체한다 (R8)
      const granted = await ensureNotificationPermission();
      if (!cancelled) setNotifGranted(granted);
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
    let applied: TimerConfig;
    try {
      applied = await setTimerConfig(next);
    } catch {
      // 검증 실패(0분 등)는 무시 — 입력단에서 이미 걸러진다
      return;
    }
    setConfig(applied);
    try {
      await saveSettings(applied);
      setSaveFailed(false);
    } catch (err) {
      // 저장 실패는 Rust 코어 상태와 별개 — 사용자에게 알리고 이번 실행에서만 유지됨을 알린다
      console.error("설정 저장 실패:", err);
      setSaveFailed(true);
    }
  }, []);

  /** Notion 커맨드 공통 실행 — 진행 플래그 관리, reject는 failed 상태로 렌더한다. */
  const runNotionCommand = useCallback(
    async (command: () => Promise<ConnectionState>): Promise<ConnectionState> => {
      setNotionVerifying(true);
      try {
        const next = await command();
        setNotionStatus(next);
        setNotionError(null);
        return next;
      } catch (err) {
        // 커맨드 reject 시 실제 상태를 재조회해 파생 UI(저장됨 배지 등)가
        // 어긋나지 않게 하고, 오류 메시지는 별도 배너로 알린다.
        // 카드에는 failed를 돌려줘 입력값 유지(실패 시 비우지 않음)를 지킨다.
        const message = typeof err === "string" ? err : String(err);
        const failed: ConnectionState = { state: "failed", message };
        const actual = await getNotionStatus().catch(() => null);
        setNotionStatus(actual ?? failed);
        setNotionError(message);
        return failed;
      } finally {
        setNotionVerifying(false);
      }
    },
    [],
  );

  const handleSaveToken = useCallback(
    (token: string) => runNotionCommand(() => saveNotionToken(token)),
    [runNotionCommand],
  );
  const handleDeleteToken = useCallback(() => {
    void runNotionCommand(deleteNotionToken);
  }, [runNotionCommand]);
  const handleSetDatabase = useCallback(
    (input: string) => {
      void runNotionCommand(() => setNotionDatabase(input));
    },
    [runNotionCommand],
  );
  const handleTestConnection = useCallback(() => {
    void runNotionCommand(testNotionConnection);
  }, [runNotionCommand]);

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
      <NotionCard
        status={notionStatus}
        isVerifying={notionVerifying}
        onSaveToken={handleSaveToken}
        onDeleteToken={handleDeleteToken}
        onSetDatabase={handleSetDatabase}
        onTestConnection={handleTestConnection}
      />
      {!notifGranted && (
        <p className="notif-hint" role="status">
          알림 권한이 꺼져 있어요 — 세션 종료는 이 카드에서 확인돼요
        </p>
      )}
      {saveFailed && (
        <p className="notif-hint" role="status">
          설정 저장에 실패했어요 — 변경은 이번 실행에만 적용돼요
        </p>
      )}
      {notionError && (
        <p className="notif-hint" role="status">
          {notionError}
        </p>
      )}
    </main>
  );
}

export default App;
