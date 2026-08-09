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
}

/** Rust TodoSnapshot(serde tagged)을 그대로 미러링한 판별 유니언. */
export type TodoSnapshot =
  | {
      state: "not_connected";
      missing: Array<"token" | "database" | "data_source">;
    }
  | { state: "no_page"; date: string }
  | {
      state: "loaded";
      date: string;
      page_id: string;
      title: string;
      items: TodoItem[];
    };

/** 쓰기 커맨드의 반환 — 재조회 스냅샷(R6)과 블록 소실·충돌 안내 문구(R8). */
export interface TodoOutcome {
  snapshot: TodoSnapshot;
  notice: string | null;
}

export const getTodoList = (): Promise<TodoSnapshot> =>
  invoke("notion_todo_list");

export const createTodoPage = (): Promise<TodoOutcome> =>
  invoke("notion_todo_create_page");

export const addTodo = (
  pageId: string,
  text: string,
  pageTitle: string,
): Promise<TodoOutcome> => invoke("notion_todo_add", { pageId, text, pageTitle });

export const toggleTodo = (
  pageId: string,
  blockId: string,
  checked: boolean,
  pageTitle: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_toggle", { pageId, blockId, checked, pageTitle });

export const editTodo = (
  pageId: string,
  blockId: string,
  text: string,
  pageTitle: string,
): Promise<TodoOutcome> =>
  invoke("notion_todo_edit", { pageId, blockId, text, pageTitle });
