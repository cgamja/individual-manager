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
