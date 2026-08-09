import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { NotionCard } from "./components/NotionCard";
import { SettingsCard } from "./components/SettingsCard";
import { TimerCard } from "./components/TimerCard";
import { TodoCard } from "./components/TodoCard";
import { ensureNotificationPermission } from "./lib/notification";
import {
  addTodo,
  createTodoPage,
  deleteNotionToken,
  editTodo,
  getNotionStatus,
  getTodoList,
  saveNotionToken,
  setNotionDatabase,
  testNotionConnection,
  toggleTodo,
  type ConnectionState,
  type TodoOutcome,
  type TodoSnapshot,
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

/** IPC 계층은 Error 객체로 reject할 수 있어 방어적으로 메시지를 추출한다. */
function errorMessage(err: unknown): string {
  return err instanceof Error
    ? err.message
    : typeof err === "string"
      ? err
      : String(err);
}

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
  const [todoSnapshot, setTodoSnapshot] = useState<TodoSnapshot | null>(null);
  const [todoBusy, setTodoBusy] = useState(false);
  // 오류와 성공 응답의 notice가 같은 배너 슬롯을 공유한다
  const [todoError, setTodoError] = useState<string | null>(null);
  // todo 요청 순번 — 뒤늦게 도착한 오래된 응답(재조회 vs 쓰기 경쟁)이
  // 최신 스냅샷·배너·busy를 덮어쓰지 않게 최신 순번만 결과를 반영한다
  const todoSeqRef = useRef(0);
  // 진행 중 여부 — visibilitychange 리스너 클로저에서는 todoBusy state가
  // 낡은 값이라 ref로 판단한다
  const todoInFlightRef = useRef(false);

  /** 목록 재조회 — 스냅샷이 이미 있으면 목록을 유지한 채 busy만 건다 (R2). */
  const refreshTodos = useCallback(async () => {
    const seq = ++todoSeqRef.current;
    todoInFlightRef.current = true;
    setTodoBusy(true);
    setTodoError(null);
    try {
      const todos = await getTodoList();
      if (seq === todoSeqRef.current) setTodoSnapshot(todos);
    } catch (err) {
      if (seq === todoSeqRef.current) setTodoError(errorMessage(err));
    } finally {
      // 더 새로운 작업이 시작됐다면 busy 해제는 그 작업의 몫이다
      if (seq === todoSeqRef.current) {
        todoInFlightRef.current = false;
        setTodoBusy(false);
      }
    }
  }, []);

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
      // 오늘 할 일 로드 — Notion 상태 로드 뒤에 시작하되, 네트워크 대기가
      // 알림 권한 확인을 막지 않게 결과만 비동기로 반영한다.
      // 로드 중 사용자가 새로고침·쓰기를 시작했다면(순번 증가) 결과를 버린다
      const mountSeq = todoSeqRef.current;
      getTodoList()
        .then((todos) => {
          if (!cancelled && mountSeq === todoSeqRef.current) setTodoSnapshot(todos);
        })
        .catch((err) => {
          if (!cancelled && mountSeq === todoSeqRef.current) setTodoError(errorMessage(err));
        });
      // 알림 권한: 거부돼도 앱은 계속 동작하고 카드 내 표시로 대체한다 (R8)
      const granted = await ensureNotificationPermission();
      if (!cancelled) setNotifGranted(granted);
    })();

    // 팝오버가 다시 보일 때 즉시 재동기화 (틱 대기 없이, 주기 폴링 없음)
    const onVisibility = () => {
      if (!document.hidden) {
        getTimerState().then(setSnapshot).catch(() => {});
        // 쓰기·재조회가 진행 중이면 스킵 — 재조회 응답이 쓰기 결과를 덮어쓰지 않게
        if (!todoInFlightRef.current) void refreshTodos();
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      cancelled = true;
      unlistenTick?.();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [refreshTodos]);

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
      // 이전 오류 배너는 새 시도 시작 시 지운다 — "확인 중..."과 겹쳐 보이지 않게
      setNotionError(null);
      try {
        const next = await command();
        setNotionStatus(next);
        return next;
      } catch (err) {
        // 커맨드 reject 시 실제 상태를 재조회해 파생 UI(저장됨 배지 등)가
        // 어긋나지 않게 하고, 오류 메시지는 별도 배너로 알린다.
        // 카드에는 failed를 돌려줘 입력값 유지(실패 시 비우지 않음)를 지킨다.
        const message = errorMessage(err);
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

  /**
   * todo 쓰기 커맨드 공통 실행 (runNotionCommand 전례) — busy 플래그 관리,
   * 성공 시 재조회 스냅샷 반영. notice(블록 소실·충돌)는 쓰기가 반영되지 않은
   * 것이므로 배너로 알리고 false를 돌려줘 입력을 유지시킨다 (R8).
   * reject 시 배너 표시 + 목록 1회 재조회로 실제 반영 여부를 보여준다.
   */
  const runTodoCommand = useCallback(
    async (command: () => Promise<TodoOutcome>): Promise<boolean> => {
      const seq = ++todoSeqRef.current;
      todoInFlightRef.current = true;
      setTodoBusy(true);
      setTodoError(null);
      try {
        const outcome = await command();
        // 더 새로운 작업이 시작됐으면 낡은 결과를 버린다 — 입력은 유지시킨다
        if (seq !== todoSeqRef.current) return false;
        if (outcome.notice) setTodoError(outcome.notice);
        // snapshot이 null이면 쓰기는 반영됐지만 재조회만 실패한 것 —
        // 기존 목록을 유지하고, 입력은 비워(true) 중복 재시도를 막는다
        if (outcome.snapshot === null) return true;
        setTodoSnapshot(outcome.snapshot);
        // 안내가 있는 스냅샷(블록 소실·충돌)은 쓰기가 반영되지 않은 것 — 입력 유지
        return !outcome.notice;
      } catch (err) {
        if (seq !== todoSeqRef.current) return false;
        setTodoError(errorMessage(err));
        // 실패 시에도 목록을 1회 재조회한다 — 입력값은 카드가 유지한다 (R8)
        const actual = await getTodoList().catch(() => null);
        if (actual && seq === todoSeqRef.current) setTodoSnapshot(actual);
        return false;
      } finally {
        // 더 새로운 작업이 시작됐다면 busy 해제는 그 작업의 몫이다
        if (seq === todoSeqRef.current) {
          todoInFlightRef.current = false;
          setTodoBusy(false);
        }
      }
    },
    [],
  );

  const handleTodoRefresh = useCallback(() => {
    void refreshTodos();
  }, [refreshTodos]);
  const handleTodoCreatePage = useCallback(() => {
    void runTodoCommand(createTodoPage);
  }, [runTodoCommand]);
  const handleTodoAdd = useCallback(
    (text: string): Promise<boolean> => {
      if (todoSnapshot?.state !== "loaded") return Promise.resolve(false);
      const { page_id, title } = todoSnapshot;
      return runTodoCommand(() => addTodo(page_id, text, title));
    },
    [todoSnapshot, runTodoCommand],
  );
  const handleTodoToggle = useCallback(
    (blockId: string, checked: boolean) => {
      if (todoSnapshot?.state !== "loaded") return;
      const { page_id, title } = todoSnapshot;
      void runTodoCommand(() => toggleTodo(page_id, blockId, checked, title));
    },
    [todoSnapshot, runTodoCommand],
  );
  const handleTodoEdit = useCallback(
    (blockId: string, text: string): Promise<boolean> => {
      if (todoSnapshot?.state !== "loaded") return Promise.resolve(false);
      const { page_id, title } = todoSnapshot;
      return runTodoCommand(() => editTodo(page_id, blockId, text, title));
    },
    [todoSnapshot, runTodoCommand],
  );

  return (
    <main className="popover">
      <TimerCard
        snapshot={snapshot}
        onStart={handleStart}
        onPause={handlePause}
        onResume={handleResume}
        onReset={handleReset}
      />
      <TodoCard
        snapshot={todoSnapshot}
        isBusy={todoBusy}
        onRefresh={handleTodoRefresh}
        onCreatePage={handleTodoCreatePage}
        onAdd={handleTodoAdd}
        onToggle={handleTodoToggle}
        onEdit={handleTodoEdit}
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
      {todoError && (
        <p className="notif-hint" role="status">
          {todoError}
        </p>
      )}
    </main>
  );
}

export default App;
