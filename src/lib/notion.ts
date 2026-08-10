import { invoke } from "@tauri-apps/api/core";

/** Rust ConnectionState(serde tagged)를 그대로 미러링한 판별 유니언. */
export type ConnectionState =
  | { state: "not_configured"; missing: "token" | "database" | "both" }
  | { state: "connected"; title: string }
  | { state: "failed"; message: string };

export const saveNotionToken = (token: string): Promise<ConnectionState> =>
  invoke("notion_save_token", { token });

export const deleteNotionToken = (): Promise<ConnectionState> =>
  invoke("notion_delete_token");

export const setNotionDatabase = (input: string): Promise<ConnectionState> =>
  invoke("notion_set_database", { input });

export const getNotionStatus = (): Promise<ConnectionState> =>
  invoke("notion_get_status");

export const testNotionConnection = (): Promise<ConnectionState> =>
  invoke("notion_test_connection");

/** Rust TodoItem을 그대로 미러링 — 페이지 본문의 to_do 블록 한 개. */
export interface TodoItem {
  id: string;
  text: string;
  checked: boolean;
  /** 직전 heading_1/2/3의 텍스트 (공부/기타 등) — 첫 헤딩 전 블록이면 null. */
  category: string | null;
}

/** Rust TodoSnapshot(serde tagged)을 그대로 미러링한 판별 유니언. */
export type TodoSnapshot =
  | {
      state: "not_connected";
      missing: Array<"token" | "database" | "data_source">;
    }
  | { state: "no_page"; date: string; is_today: boolean }
  | {
      state: "loaded";
      date: string;
      page_id: string;
      title: string;
      items: TodoItem[];
      is_today: boolean;
      /** 이 행의 `수행도` — null이면 "미지정" (R1). */
      performance: string | null;
      /** 이 행의 시작 날짜 — 모르는 경로(생성 직후·확인되지 않은 에코)면 null (R10). */
      range_start: string | null;
      /** 행이 날짜 범위를 덮을 때의 끝 날짜 — 하루 행이면 null (R10). */
      range_end: string | null;
    };

/** 쓰기 커맨드의 반환 — 재조회 스냅샷(R6)과 블록 소실·충돌 안내 문구(R8).
 * snapshot이 null이면 쓰기는 반영됐지만 재조회가 실패한 경우다 — 기존 목록을 유지하고 notice만 표시한다. */
export interface TodoOutcome {
  snapshot: TodoSnapshot | null;
  notice: string | null;
}

/** Rust CreateRowOutcome(serde tagged)을 그대로 미러링 — 생성 성공(created)과
 * 같은 제목 행이 기간과 겹쳐 생성하지 않은 경우(exists)를 state로 구분한다. */
export type CreateRowOutcome =
  | { state: "created"; snapshot: TodoSnapshot | null; notice: string | null }
  | {
      state: "exists";
      page_id: string;
      title: string;
      date: string;
      /** 겹친 행의 현재 수행도 — "기존 행 열기"가 그대로 넘긴다. */
      performance: string | null;
    };

export const getTodoList = (): Promise<TodoSnapshot> =>
  invoke("notion_todo_list");

export const createTodoPage = (): Promise<TodoOutcome> =>
  invoke("notion_todo_create_page");

/** 미래 [TODO] 행 생성 — 날짜 하나만 받는다 (제목·아이콘은 백엔드가 채운다). */
export const createTodoRow = (params: {
  start: string;
}): Promise<CreateRowOutcome> => invoke("notion_todo_create_row", { ...params });

/** 쓰기 커맨드는 재조회가 주지 않는 페이지 메타를 프론트가 되실어 준다 —
 * `pageTitle`과 같은 방식이다. 모르면 생략하고, 그러면 스냅샷도 null로 돌아온다. */
export const openTodoPage = (
  pageId: string,
  pageTitle: string,
  date: string,
  performance?: string,
  rangeStart?: string,
  rangeEnd?: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_open_page", {
    pageId,
    pageTitle,
    date,
    performance,
    rangeStart,
    rangeEnd,
  });

export const addTodo = (
  pageId: string,
  text: string,
  pageTitle: string,
  date?: string,
  /** 삽입할 헤딩 텍스트 (공부/기타) — 미지정이면 백엔드가 끝에 append. */
  category?: string,
  performance?: string,
  rangeStart?: string,
  rangeEnd?: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_add", {
    pageId,
    text,
    pageTitle,
    date,
    category,
    performance,
    rangeStart,
    rangeEnd,
  });

export const toggleTodo = (
  pageId: string,
  blockId: string,
  checked: boolean,
  pageTitle: string,
  date?: string,
  performance?: string,
  rangeStart?: string,
  rangeEnd?: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_toggle", {
    pageId,
    blockId,
    checked,
    pageTitle,
    date,
    performance,
    rangeStart,
    rangeEnd,
  });

export const editTodo = (
  pageId: string,
  blockId: string,
  text: string,
  pageTitle: string,
  date?: string,
  performance?: string,
  rangeStart?: string,
  rangeEnd?: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_edit", {
    pageId,
    blockId,
    text,
    pageTitle,
    date,
    performance,
    rangeStart,
    rangeEnd,
  });

/** 보고 있는 행의 `수행도`를 바꾼다 (R3). 옵션 4개(완료/일부/미완/기타) 가드는
 * Rust가 갖고 있다(KTD2) — 여기서는 값을 그대로 넘긴다.
 * `previousPerformance`는 화면에 지금 보이는 값 — 저장이 확인되지 않은 경로에서
 * 시도값 대신 이 값이 되실린다 (R9). */
export const setTodoPerformance = (
  pageId: string,
  pageTitle: string,
  date: string | undefined,
  performance: string,
  previousPerformance?: string,
  rangeStart?: string,
  rangeEnd?: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_set_performance", {
    pageId,
    pageTitle,
    date,
    performance,
    previousPerformance,
    rangeStart,
    rangeEnd,
  });
