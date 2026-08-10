import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { NotionCard } from "./components/NotionCard";
import { SettingsCard } from "./components/SettingsCard";
import { TimerCard } from "./components/TimerCard";
import {
  TodoCard,
  type CreateRowFormParams,
  type CreateRowFormResult,
} from "./components/TodoCard";
import { ensureNotificationPermission } from "./lib/notification";
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
  setTodoPerformance,
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
  // 현재 스냅샷 — runTodoCommand의 실패 재조회가 (낡은 클로저가 아닌) 최신
  // 날짜를 보고 재조회 경로를 고르도록 state와 함께 항상 갱신한다
  const todoSnapshotRef = useRef<TodoSnapshot | null>(null);

  /** 스냅샷 반영 — ref를 state와 같은 시점에 갱신한다 (실패 재조회 경로 판단용). */
  const applyTodoSnapshot = useCallback((next: TodoSnapshot) => {
    todoSnapshotRef.current = next;
    setTodoSnapshot(next);
  }, []);

  /** todo 작업 시작 부기 — 새 순번 발급, in-flight·busy 표시, 오류 초기화. */
  const beginTodoTurn = useCallback((): number => {
    const seq = ++todoSeqRef.current;
    todoInFlightRef.current = true;
    setTodoBusy(true);
    setTodoError(null);
    return seq;
  }, []);

  /** todo 작업 종료 부기 — 더 새로운 작업이 시작됐다면 busy 해제는 그 작업의 몫이다. */
  const endTodoTurnIfCurrent = useCallback((seq: number) => {
    if (seq === todoSeqRef.current) {
      todoInFlightRef.current = false;
      setTodoBusy(false);
    }
  }, []);

  /** 목록 재조회 — 스냅샷이 이미 있으면 목록을 유지한 채 busy만 건다 (R2). */
  const refreshTodos = useCallback(async () => {
    const seq = ++todoSeqRef.current;
    todoInFlightRef.current = true;
    setTodoBusy(true);
    setTodoError(null);
    try {
      const todos = await getTodoList();
      if (seq === todoSeqRef.current) applyTodoSnapshot(todos);
    } catch (err) {
      if (seq === todoSeqRef.current) setTodoError(errorMessage(err));
    } finally {
      // 더 새로운 작업이 시작됐다면 busy 해제는 그 작업의 몫이다
      if (seq === todoSeqRef.current) {
        todoInFlightRef.current = false;
        setTodoBusy(false);
      }
    }
  }, [applyTodoSnapshot]);

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
          if (!cancelled && mountSeq === todoSeqRef.current) applyTodoSnapshot(todos);
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
  }, [refreshTodos, applyTodoSnapshot]);

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
   * 쓰기 실패 후 1회 재조회 — 타임아웃 뒤 실제로 반영됐다면 재조회가 그 결과를
   * 드러낸다. 날짜 전환 중(loaded && !is_today)이면 그 날짜(openTodoPage)로
   * 재조회해 오늘로 튕기지 않게 하고, 오늘이면 getTodoList. 결과는 순번이
   * 여전히 최신일 때만 반영하고 재조회 자체의 실패는 삼킨다.
   * 열기 경로에 되싣는 수행도는 방금 시도한 값이 아니라 화면에 남아 있는
   * 직전 값이다 — 저장이 확인되지 않은 값을 선택된 것처럼 보이게 하지 않는다 (R9).
   */
  const refetchAfterTodoFailure = useCallback(
    async (seq: number) => {
      const current = todoSnapshotRef.current;
      const actual =
        current?.state === "loaded" && !current.is_today
          ? await openTodoPage(
              current.page_id,
              current.title,
              current.date,
              current.performance ?? undefined,
              current.range_start ?? undefined,
              current.range_end ?? undefined,
            )
              .then((o) => o.snapshot)
              .catch(() => null)
          : await getTodoList().catch(() => null);
      if (actual && seq === todoSeqRef.current) applyTodoSnapshot(actual);
    },
    [applyTodoSnapshot],
  );

  /**
   * todo 쓰기 커맨드 공통 실행 (runNotionCommand 전례) — busy 플래그 관리,
   * 성공 시 재조회 스냅샷 반영. notice(블록 소실·충돌)는 쓰기가 반영되지 않은
   * 것이므로 배너로 알리고 false를 돌려줘 입력을 유지시킨다 (R8).
   * reject 시 배너 표시 + 목록 1회 재조회로 실제 반영 여부를 보여준다.
   */
  const runTodoCommand = useCallback(
    async (command: () => Promise<TodoOutcome>): Promise<boolean> => {
      const seq = beginTodoTurn();
      try {
        const outcome = await command();
        // 더 새로운 작업이 시작됐으면 낡은 결과를 버린다 — 입력은 유지시킨다
        if (seq !== todoSeqRef.current) return false;
        if (outcome.notice) setTodoError(outcome.notice);
        // snapshot이 null이면 쓰기는 반영됐지만 재조회만 실패한 것 —
        // 기존 목록을 유지하고, 입력은 비워(true) 중복 재시도를 막는다
        if (outcome.snapshot === null) return true;
        applyTodoSnapshot(outcome.snapshot);
        // 안내가 있는 스냅샷(블록 소실·충돌)은 쓰기가 반영되지 않은 것 — 입력 유지
        return !outcome.notice;
      } catch (err) {
        if (seq !== todoSeqRef.current) return false;
        setTodoError(errorMessage(err));
        // 실패 시에도 목록을 1회 재조회한다 — 입력값은 카드가 유지한다 (R8)
        await refetchAfterTodoFailure(seq);
        return false;
      } finally {
        endTodoTurnIfCurrent(seq);
      }
    },
    [applyTodoSnapshot, beginTodoTurn, endTodoTurnIfCurrent, refetchAfterTodoFailure],
  );

  const handleTodoRefresh = useCallback(() => {
    void refreshTodos();
  }, [refreshTodos]);
  const handleTodoCreatePage = useCallback(() => {
    void runTodoCommand(createTodoPage);
  }, [runTodoCommand]);
  const handleTodoAdd = useCallback(
    (text: string, category: string): Promise<boolean> => {
      if (todoSnapshot?.state !== "loaded") return Promise.resolve(false);
      const { page_id, title, date, performance, range_start, range_end } =
        todoSnapshot;
      // category = 선택된 헤딩 텍스트 — 백엔드가 해당 헤딩 아래에 삽입한다.
      // 수행도·적용 구간은 children 재조회가 주지 않는 페이지 메타라 되실어 준다 (KTD1)
      return runTodoCommand(() =>
        addTodo(
          page_id,
          text,
          title,
          date,
          category,
          performance ?? undefined,
          range_start ?? undefined,
          range_end ?? undefined,
        ),
      );
    },
    [todoSnapshot, runTodoCommand],
  );
  const handleTodoToggle = useCallback(
    (blockId: string, checked: boolean) => {
      if (todoSnapshot?.state !== "loaded") return;
      const { page_id, title, date, performance, range_start, range_end } =
        todoSnapshot;
      void runTodoCommand(() =>
        toggleTodo(
          page_id,
          blockId,
          checked,
          title,
          date,
          performance ?? undefined,
          range_start ?? undefined,
          range_end ?? undefined,
        ),
      );
    },
    [todoSnapshot, runTodoCommand],
  );
  const handleTodoEdit = useCallback(
    (blockId: string, text: string): Promise<boolean> => {
      if (todoSnapshot?.state !== "loaded") return Promise.resolve(false);
      const { page_id, title, date, performance, range_start, range_end } =
        todoSnapshot;
      return runTodoCommand(() =>
        editTodo(
          page_id,
          blockId,
          text,
          title,
          date,
          performance ?? undefined,
          range_start ?? undefined,
          range_end ?? undefined,
        ),
      );
    },
    [todoSnapshot, runTodoCommand],
  );
  /** 수행도 즉시 저장 (R3) — `handleTodoToggle` 전례로 busy·seq·실패 재조회를
   * 공통 경로에 맡긴다. 직전 값을 함께 넘겨 확인되지 않은 값이 되실리지 않게 한다 (R9). */
  const handleTodoSetPerformance = useCallback(
    (performance: string) => {
      if (todoSnapshot?.state !== "loaded") return;
      const { page_id, title, date, performance: current, range_start, range_end } =
        todoSnapshot;
      void runTodoCommand(() =>
        setTodoPerformance(
          page_id,
          title,
          date,
          performance,
          current ?? undefined,
          range_start ?? undefined,
          range_end ?? undefined,
        ),
      );
    },
    [todoSnapshot, runTodoCommand],
  );
  /**
   * 행 만들기(미래 [TODO] 전용 — start만 받는다) — CreateRowOutcome은
   * TodoOutcome이 아니라 runTodoCommand를 못 탄다. busy·seq 관리는 동일하게
   * 하되, exists는 스냅샷을 건드리지 않고 카드에 그대로 돌려줘
   * "기존 행 열기" 안내를 띄우게 한다 (스펙 5).
   */
  const handleTodoCreateRow = useCallback(
    async (params: CreateRowFormParams): Promise<CreateRowFormResult> => {
      const seq = beginTodoTurn();
      try {
        const created = await createTodoRow(params);
        // 더 새로운 작업이 시작됐으면 낡은 결과를 버린다 — 폼·입력은 유지시킨다
        if (seq !== todoSeqRef.current) return { state: "failed" };
        if (created.state === "exists") {
          return {
            state: "exists",
            page_id: created.page_id,
            title: created.title,
            date: created.date,
            performance: created.performance,
          };
        }
        // created — snapshot이 null이면 생성은 됐지만 재조회만 실패한 것 (notice로 안내)
        if (created.notice) setTodoError(created.notice);
        if (created.snapshot !== null) applyTodoSnapshot(created.snapshot);
        return { state: "created" };
      } catch (err) {
        // 실패 — 배너를 띄우고 폼 입력은 카드가 유지한다 (R10).
        // runTodoCommand와 동일하게 1회 재조회한다 — 타임아웃 뒤 실제로
        // 생성됐다면 재조회가 그 행/exists 상태를 드러낸다
        if (seq !== todoSeqRef.current) return { state: "failed" };
        setTodoError(errorMessage(err));
        await refetchAfterTodoFailure(seq);
        return { state: "failed" };
      } finally {
        endTodoTurnIfCurrent(seq);
      }
    },
    [applyTodoSnapshot, beginTodoTurn, endTodoTurnIfCurrent, refetchAfterTodoFailure],
  );
  /** exists의 기존 행 열기 — openTodoPage는 TodoOutcome을 돌려줘 공통 경로를 탄다.
   * 적용 구간은 exists가 알려주지 않아 생략한다 — 열기 스냅샷도 그만큼만 안다. */
  const handleTodoOpenPage = useCallback(
    (
      pageId: string,
      title: string,
      date: string,
      performance: string | null,
    ): Promise<boolean> =>
      runTodoCommand(() => openTodoPage(pageId, title, date, performance ?? undefined)),
    [runTodoCommand],
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
        onCreateRow={handleTodoCreateRow}
        onOpenPage={handleTodoOpenPage}
        onSetPerformance={handleTodoSetPerformance}
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
