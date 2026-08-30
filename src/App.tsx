import { useCallback, useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LauncherFan } from "./components/LauncherFan";
import { SettingsCard } from "./components/SettingsCard";
import { TauntCard } from "./components/TauntCard";
import { TimerCard } from "./components/TimerCard";
import { DEFAULT_LAUNCHER, openInBrowser, type LauncherService } from "./lib/launcher";
import { ensureNotificationPermission } from "./lib/notification";
import { setPetEnabled } from "./lib/pet";
import {
  DEFAULT_PET_SETTINGS,
  DEFAULT_SETTINGS,
  loadLauncher,
  loadPetSettings,
  loadSettings,
  loadTaunts,
  savePetSettings,
  saveSettings,
  saveTaunts,
} from "./lib/settings";
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

/**
 * 팝오버 — 최상위는 런처다 (PRD §5.2). 앱이 직접 소유하는 것은 타이머와 설정뿐이고,
 * 서비스 조회·기록은 Claude Code가, 깊은 작업은 Chrome으로 연 원래 웹 UI가 맡는다
 * (PRINCIPLE 1·2).
 */
function App() {
  const [snapshot, setSnapshot] = useState<TimerSnapshot>({ state: "idle" });
  const [config, setConfig] = useState<TimerConfig>(DEFAULT_SETTINGS);
  const [notifGranted, setNotifGranted] = useState(true);
  const [saveFailed, setSaveFailed] = useState(false);
  const [petEnabled, setPetEnabledState] = useState(DEFAULT_PET_SETTINGS.enabled);
  const [taunts, setTaunts] = useState<readonly string[]>([]);
  const [launcher, setLauncher] = useState<readonly LauncherService[]>(DEFAULT_LAUNCHER);
  /** 펭귄 on/off와 대사 편집은 다섯 번째 카드가 아니라 여기 들어간다 (A4). */
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    let unlistenTick: UnlistenFn | undefined;
    let cancelled = false;

    (async () => {
      const savedPet = await loadPetSettings().catch(() => DEFAULT_PET_SETTINGS);
      if (!cancelled) setPetEnabledState(savedPet.enabled);
      const savedTaunts = await loadTaunts().catch(() => []);
      if (!cancelled) setTaunts(savedTaunts);
      // 저장된 게 없으면 기본 목록으로 수렴한다 — 편집 UI는 후속 작업이다
      const savedLauncher = await loadLauncher().catch(() => [...DEFAULT_LAUNCHER]);
      if (!cancelled) setLauncher(savedLauncher);
      // 저장된 설정을 Rust 코어에 반영한 뒤 상태를 동기화한다
      const saved = await loadSettings().catch(() => DEFAULT_SETTINGS);
      const applied = await setTimerConfig(saved).catch(() => DEFAULT_SETTINGS);
      if (cancelled) return;
      setConfig(applied);
      setSnapshot(await getTimerState());
      unlistenTick = await onTick((s) => setSnapshot(s));
      // 알림 권한: 거부돼도 앱은 계속 동작하고 카드 내 표시로 대체한다
      const granted = await ensureNotificationPermission();
      if (!cancelled) setNotifGranted(granted);
    })();

    // 팝오버가 다시 보일 때 즉시 재동기화 (틱 대기 없이, 주기 폴링 없음)
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

  /** 펭귄 on/off — Rust가 창을 만들거나 닫고, 저장은 여기서 한다.
   * 커맨드가 실패하면 화면 표시를 되돌린다 — 켜지지 않았는데 켜진 것처럼 보이지 않게. */
  const handlePetEnabledChange = useCallback(async (next: boolean) => {
    setPetEnabledState(next);
    try {
      await setPetEnabled(next);
    } catch (err) {
      // 창을 못 만들거나 못 닫았다 — 표시만 되돌린다
      console.error("펭귄 표시 변경 실패:", err);
      setPetEnabledState(!next);
      return;
    }
    try {
      await savePetSettings({ enabled: next });
    } catch (err) {
      // 창은 바뀌었는데 저장만 실패했다. 표시만 되돌리면 "꺼짐인데 떠 있는"
      // 상태가 되므로 창도 함께 원복해 화면과 실제를 맞춘다
      console.error("펭귄 설정 저장 실패:", err);
      await setPetEnabled(!next).catch(() => {});
      setPetEnabledState(!next);
    }
  }, []);

  /**
   * 런처가 다 접힌 뒤에 온 Esc — 설정이 열려 있으면 그걸 접고, 아니면 팝오버를 닫는다 (R6).
   *
   * `app.hide()`가 아니라 창만 숨긴다. macOS 26에서 `app.hide()`는 트레이 아이콘까지
   * 지운다 (docs/solutions/ui-bugs/macos-tahoe-app-hide-removes-tray-icon.md).
   */
  const handleEscape = useCallback(() => {
    if (settingsOpen) {
      setSettingsOpen(false);
      return;
    }
    getCurrentWindow().hide().catch(() => {});
  }, [settingsOpen]);

  /** 대사 편집 — 화면을 먼저 바꾸고 저장한다. 실패하면 되돌린다. */
  const handleTauntsChange = useCallback(
    async (next: string[]) => {
      const before = taunts;
      setTaunts(next);
      try {
        await saveTaunts(next);
        setSaveFailed(false);
      } catch (err) {
        console.error("대사 저장 실패:", err);
        setTaunts(before);
        setSaveFailed(true);
      }
    },
    [taunts],
  );

  return (
    <main className="popover">
      <LauncherFan
        services={launcher}
        onOpen={openInBrowser}
        onEscape={handleEscape}
        timerPanel={
          <TimerCard
            snapshot={snapshot}
            onStart={handleStart}
            onPause={handlePause}
            onResume={handleResume}
            onReset={handleReset}
          />
        }
      />
      <div className="launcher-foot">
        <button
          type="button"
          className="launcher-gear"
          aria-expanded={settingsOpen}
          onClick={() => setSettingsOpen((open) => !open)}
        >
          설정
        </button>
        <span className="launcher-hint">Esc · 접기 / 닫기</span>
      </div>
      {settingsOpen && (
        <>
          <SettingsCard
            config={config}
            disabled={snapshot.state !== "idle"}
            onChange={handleConfigChange}
            petEnabled={petEnabled}
            onPetEnabledChange={(next) => void handlePetEnabledChange(next)}
          />
          <TauntCard lines={taunts} onChange={(next) => void handleTauntsChange(next)} />
        </>
      )}
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
    </main>
  );
}

export default App;
