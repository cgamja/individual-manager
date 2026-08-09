//! Notion 브릿지 — 코어(notion)를 앱에 연결한다: Keychain 토큰 보관,
//! store(`settings.json`) 설정 영속, 연결 커맨드 5종.
//! 토큰 값은 Keychain에만 존재한다 — 응답·로그·store·에러 어디에도 넣지 않는다.

use serde::Serialize;
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::notion::{
    determine_connection_state, parse_database_id, ConnectError, ConnectionState, NotionClient,
    TodoItem, NOTION_API_BASE,
};

/// Keychain service 이름 — 번들 ID(com.kangr.penguin) 기반.
const KEYCHAIN_SERVICE: &str = "com.kangr.penguin.notion";
const KEYCHAIN_ACCOUNT: &str = "notion-token";

/// 프론트(src/lib/settings.ts)의 `timer` 키와 같은 파일을 공유한다 — 경로 문자열 동일 필수.
const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "notion";

/// Keychain·store 오류의 사용자 메시지. 내부 오류 문자열(Display/Debug)은 이어붙이지 않는다.
const KEYCHAIN_ERROR: &str = "Keychain 접근에 실패했습니다";
const STORE_ERROR: &str = "설정 저장소 접근에 실패했습니다";

/// store의 `notion` 키에 영속되는 설정.
/// data_source_id·title은 검증 성공 시에만 채워지는 캐시다.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct NotionSettings {
    pub database_id: Option<String>,
    pub data_source_id: Option<String>,
    pub title: Option<String>,
}

/// store JSON → 설정. 없는 키·타입 불일치는 None으로 관대하게 읽는다.
pub fn settings_from_json(value: Option<&Value>) -> NotionSettings {
    let get = |key: &str| {
        value
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    NotionSettings {
        database_id: get("database_id"),
        data_source_id: get("data_source_id"),
        title: get("title"),
    }
}

/// 설정 → store JSON. None 필드는 null로 남겨 형태를 고정한다.
pub fn settings_to_json(settings: &NotionSettings) -> Value {
    json!({
        "database_id": settings.database_id,
        "data_source_id": settings.data_source_id,
        "title": settings.title,
    })
}

// ---------------------------------------------------------------------------
// Keychain — 블로킹 호출이므로 async 커맨드에서는 spawn_blocking으로 감싼다
// ---------------------------------------------------------------------------

fn keychain_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).map_err(|_| KEYCHAIN_ERROR.to_string())
}

/// 토큰 존재 여부만 확인한다 (NoEntry = 미저장, 오류 아님).
fn token_saved_blocking() -> Result<bool, String> {
    Ok(load_token_blocking()?.is_some())
}

fn load_token_blocking() -> Result<Option<String>, String> {
    match keychain_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(KEYCHAIN_ERROR.to_string()),
    }
}

fn save_token_blocking(token: &str) -> Result<(), String> {
    keychain_entry()?
        .set_password(token)
        .map_err(|_| KEYCHAIN_ERROR.to_string())
}

fn delete_token_blocking() -> Result<(), String> {
    match keychain_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(KEYCHAIN_ERROR.to_string()),
    }
}

/// 블로킹 Keychain 함수를 async 컨텍스트에서 실행한다.
async fn on_keychain<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| KEYCHAIN_ERROR.to_string())?
}

// ---------------------------------------------------------------------------
// store — 설정 읽기/쓰기 (쓰기는 항상 save()로 디스크 플러시)
// ---------------------------------------------------------------------------

fn read_settings(app: &AppHandle) -> Result<NotionSettings, String> {
    let store = app.store(STORE_FILE).map_err(|_| STORE_ERROR.to_string())?;
    Ok(settings_from_json(store.get(STORE_KEY).as_ref()))
}

fn write_settings(app: &AppHandle, settings: &NotionSettings) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|_| STORE_ERROR.to_string())?;
    store.set(STORE_KEY, settings_to_json(settings));
    store.save().map_err(|_| STORE_ERROR.to_string())
}

// ---------------------------------------------------------------------------
// 검증 공통 경로
// ---------------------------------------------------------------------------

/// 토큰·DB가 모두 있는 상태에서 실제 검증을 수행하고 상태로 변환한다.
/// 성공 시에만 data_source_id·title을 store에 채우고 저장한다.
async fn verify_and_cache(
    app: &AppHandle,
    token: &str,
    database_id: &str,
    base_url: &str,
) -> Result<ConnectionState, String> {
    let client = NotionClient::new(base_url);
    let verified = match client.verify_connection(token, database_id).await {
        Ok(verified) => {
            write_settings(
                app,
                &NotionSettings {
                    database_id: Some(database_id.to_string()),
                    data_source_id: Some(verified.data_source_id.clone()),
                    title: Some(verified.title.clone()),
                },
            )?;
            Some(Ok(verified.title))
        }
        Err(err) => {
            // 확정적 실패(토큰 무효·미공유·스키마 불일치 등)는 검증 캐시를 비워
            // 재시작 후 stale '연결됨' 표시를 막는다. 일시 오류(네트워크·429)는
            // 캐시를 유지해 오프라인 재시작이 연결 상태를 잃지 않게 한다.
            let transient = matches!(err, ConnectError::Network(_) | ConnectError::RateLimited);
            if !transient {
                // 캐시 비우기는 부수 효과 — store 쓰기가 실패해도 실제 검증 실패
                // 원인(토큰 무효·미공유·스키마 불일치)을 가리지 않고 그대로 전달한다
                let _ = write_settings(
                    app,
                    &NotionSettings {
                        database_id: Some(database_id.to_string()),
                        data_source_id: None,
                        title: None,
                    },
                );
            }
            Some(Err(err))
        }
    };
    Ok(determine_connection_state(true, true, verified))
}

// ---------------------------------------------------------------------------
// 커맨드 5종
// ---------------------------------------------------------------------------

/// 토큰을 Keychain에 저장한다. DB가 이미 지정돼 있으면 곧바로 검증까지 수행한다.
#[tauri::command]
pub async fn notion_save_token(token: String, app: AppHandle) -> Result<ConnectionState, String> {
    let saved = token.clone();
    on_keychain(move || save_token_blocking(&saved)).await?;
    let settings = read_settings(&app)?;
    match settings.database_id {
        Some(database_id) => verify_and_cache(&app, &token, &database_id, NOTION_API_BASE).await,
        None => Ok(determine_connection_state(true, false, None)),
    }
}

/// Keychain에서 토큰을 삭제한다 (미저장이면 무시).
/// 검증 캐시(data_source_id·title)는 지워진 토큰의 결과이므로 함께 비운다.
#[tauri::command]
pub async fn notion_delete_token(app: AppHandle) -> Result<ConnectionState, String> {
    on_keychain(delete_token_blocking).await?;
    let settings = read_settings(&app)?;
    if settings.data_source_id.is_some() || settings.title.is_some() {
        let _ = write_settings(
            &app,
            &NotionSettings {
                database_id: settings.database_id.clone(),
                data_source_id: None,
                title: None,
            },
        );
    }
    Ok(determine_connection_state(
        false,
        settings.database_id.is_some(),
        None,
    ))
}

/// DB 입력(URL 또는 ID)을 파싱해 저장한다. 새 DB이므로 기존 검증 캐시는 비운다.
/// 토큰이 있으면 곧바로 검증까지 수행한다.
#[tauri::command]
pub async fn notion_set_database(input: String, app: AppHandle) -> Result<ConnectionState, String> {
    let database_id = parse_database_id(&input).map_err(|e| e.message())?;
    write_settings(
        &app,
        &NotionSettings {
            database_id: Some(database_id.clone()),
            data_source_id: None,
            title: None,
        },
    )?;
    match on_keychain(load_token_blocking).await? {
        Some(token) => verify_and_cache(&app, &token, &database_id, NOTION_API_BASE).await,
        None => Ok(determine_connection_state(false, true, None)),
    }
}

/// 네트워크 호출 없이 캐시 기반으로 현재 상태를 돌려준다.
#[tauri::command]
pub async fn notion_get_status(app: AppHandle) -> Result<ConnectionState, String> {
    let token_saved = on_keychain(token_saved_blocking).await?;
    let settings = read_settings(&app)?;
    let verified = settings.title.map(Ok);
    Ok(determine_connection_state(
        token_saved,
        settings.database_id.is_some(),
        verified,
    ))
}

/// 실제 Notion API로 연결을 재검증한다. 성공 시 data_source_id·title을 갱신한다.
#[tauri::command]
pub async fn notion_test_connection(app: AppHandle) -> Result<ConnectionState, String> {
    let token = on_keychain(load_token_blocking).await?;
    let settings = read_settings(&app)?;
    match (token, settings.database_id) {
        (Some(token), Some(database_id)) => {
            verify_and_cache(&app, &token, &database_id, NOTION_API_BASE).await
        }
        (token, database_id) => Ok(determine_connection_state(
            token.is_some(),
            database_id.is_some(),
            None,
        )),
    }
}

// ---------------------------------------------------------------------------
// todo 커맨드 5종 — 오늘 페이지의 to_do 블록 조회·생성·수정 (U4)
// ---------------------------------------------------------------------------

/// todo 커맨드가 웹뷰에 돌려주는 스냅샷 (`ConnectionState` 전례 — KTD5).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TodoSnapshot {
    /// 토큰·DB·data source 중 무엇이 없는지 snake_case 문자열로 명시한다.
    /// 기존 `Missing` enum은 data_source 부재를 표현하지 못해 재사용하지 않는다.
    NotConnected {
        missing: Vec<String>,
    },
    NoPage {
        date: String,
    },
    Loaded {
        date: String,
        page_id: String,
        title: String,
        items: Vec<TodoItem>,
    },
}

/// 쓰기 커맨드의 반환 — 재조회한 스냅샷(R6)과, 블록 소실·충돌을 오류 대신
/// 안내로 처리할 때(R8)의 문구.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct TodoOutcome {
    pub snapshot: TodoSnapshot,
    pub notice: Option<String>,
}

/// 블록 소실(404)·편집 충돌(409) 시 자동 재조회와 함께 싣는 안내 문구.
/// DB용 NotFound 메시지와 별개다 — 블록 수준 문제를 DB 연결 문제로 오독하게 하지 않는다.
const TODO_STALE_NOTICE: &str =
    "할 일이 원격에서 바뀌어 목록을 새로 불러왔습니다. 다시 시도해 주세요.";

/// 쓰기 커맨드가 미연결 상태에서 호출됐을 때의 오류 메시지 (스냅샷 없는 오류 경로).
const TODO_NOT_CONNECTED_ERROR: &str = "Notion 연결이 필요합니다. 설정을 확인해 주세요";

/// 미연결 판정 — 무엇이 없는지 snake_case 문자열 목록으로 돌려준다 (R7).
/// `notion_get_status`의 캐시 기반 표시와 독립인 순수 판정.
pub fn todo_missing(
    token_present: bool,
    database_present: bool,
    data_source_present: bool,
) -> Vec<String> {
    let mut missing = Vec::new();
    if !token_present {
        missing.push("token".to_string());
    }
    if !database_present {
        missing.push("database".to_string());
    }
    if !data_source_present {
        missing.push("data_source".to_string());
    }
    missing
}

/// "오늘"은 Rust가 소유한다 (KTD3) — 로컬 날짜 `YYYY-MM-DD`.
/// 순수 로직은 전부 이 결과를 파라미터로 받는다.
fn today_local() -> String {
    chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// todo 커맨드 공통 자격 — 토큰과 data source ID.
struct TodoAccess {
    token: String,
    data_source_id: String,
}

/// 토큰(Keychain)·설정(store)을 읽어 자격을 만든다.
/// 바깥 Err = Keychain·store 접근 실패, 안쪽 Err = 미연결(missing 목록).
async fn todo_access(app: &AppHandle) -> Result<Result<TodoAccess, Vec<String>>, String> {
    let token = on_keychain(load_token_blocking).await?;
    let settings = read_settings(app)?;
    let missing = todo_missing(
        token.is_some(),
        settings.database_id.is_some(),
        settings.data_source_id.is_some(),
    );
    match (token, settings.data_source_id) {
        (Some(token), Some(data_source_id)) if missing.is_empty() => Ok(Ok(TodoAccess {
            token,
            data_source_id,
        })),
        _ => Ok(Err(missing)),
    }
}

/// 날짜 쿼리 경로로 스냅샷을 만든다 — 행 없음 → no_page, 있음 → children 조회 → loaded.
async fn snapshot_by_date(
    base_url: &str,
    access: &TodoAccess,
    date: &str,
) -> Result<TodoSnapshot, ConnectError> {
    let client = NotionClient::new(base_url);
    match client
        .find_page_by_date(&access.token, &access.data_source_id, date)
        .await?
    {
        None => Ok(TodoSnapshot::NoPage {
            date: date.to_string(),
        }),
        Some((page_id, title)) => {
            let items = client.fetch_todos(&access.token, &page_id).await?;
            Ok(TodoSnapshot::Loaded {
                date: date.to_string(),
                page_id,
                title,
                items,
            })
        }
    }
}

/// 쓰기 후 스냅샷 — 이미 아는 page_id의 children 재조회로 시작한다 (KTD5,
/// 날짜 재쿼리는 페이지 자체가 사라진 404일 때의 폴백으로만).
async fn snapshot_after_write(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
) -> Result<TodoSnapshot, ConnectError> {
    let client = NotionClient::new(base_url);
    match client.fetch_todos(&access.token, page_id).await {
        Ok(items) => Ok(TodoSnapshot::Loaded {
            date: date.to_string(),
            page_id: page_id.to_string(),
            title: page_title.to_string(),
            items,
        }),
        Err(ConnectError::NotFound) => snapshot_by_date(base_url, access, date).await,
        Err(e) => Err(e),
    }
}

/// 블록 쓰기 결과의 공통 종단 (R6·R8): 성공 → 재조회 스냅샷,
/// 블록 소실(404)·충돌(409) → Err 대신 재조회 스냅샷 + 안내 문구,
/// 그 외(네트워크·인증·한도·형식) → 스냅샷 없는 오류로 전달.
async fn finish_write(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
    write: Result<(), ConnectError>,
) -> Result<TodoOutcome, String> {
    let notice = match write {
        Ok(()) => None,
        Err(ConnectError::NotFound | ConnectError::Conflict) => Some(TODO_STALE_NOTICE.to_string()),
        Err(e) => return Err(e.message()),
    };
    let snapshot = snapshot_after_write(base_url, access, page_id, page_title, date)
        .await
        .map_err(|e| e.message())?;
    Ok(TodoOutcome { snapshot, notice })
}

/// 오늘 페이지의 to_do 목록 스냅샷을 돌려준다 (R1·R7).
#[tauri::command]
pub async fn notion_todo_list(app: AppHandle) -> Result<TodoSnapshot, String> {
    match todo_access(&app).await? {
        Err(missing) => Ok(TodoSnapshot::NotConnected { missing }),
        Ok(access) => snapshot_by_date(NOTION_API_BASE, &access, &today_local())
            .await
            .map_err(|e| e.message()),
    }
}

/// 오늘 행이 없을 때 `[TODO]` 골격 페이지를 만들고 그 페이지의 스냅샷을 돌려준다 (R3).
/// 날짜 재쿼리 없이 POST 응답의 page ID로 곧장 children을 조회한다 (KTD5).
#[tauri::command]
pub async fn notion_todo_create_page(app: AppHandle) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let date = today_local();
    let client = NotionClient::new(NOTION_API_BASE);
    let page_id = client
        .create_day_page(&access.token, &access.data_source_id, &date)
        .await
        .map_err(|e| e.message())?;
    let items = client
        .fetch_todos(&access.token, &page_id)
        .await
        .map_err(|e| e.message())?;
    Ok(TodoOutcome {
        snapshot: TodoSnapshot::Loaded {
            date,
            page_id,
            title: "[TODO]".to_string(),
            items,
        },
        notice: None,
    })
}

/// 페이지 본문 끝에 to_do를 추가하고 재조회 스냅샷을 돌려준다 (R4·R6).
/// `page_title`은 프론트가 현재 스냅샷에서 넘긴다 — children 재조회는 페이지
/// 제목을 주지 않고, 날짜 재쿼리는 KTD5가 금지한다.
#[tauri::command]
pub async fn notion_todo_add(
    page_id: String,
    text: String,
    page_title: String,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let client = NotionClient::new(NOTION_API_BASE);
    let write = client.append_todo(&access.token, &page_id, &text).await;
    finish_write(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &today_local(),
        write,
    )
    .await
}

/// to_do 체크 상태를 토글하고 재조회 스냅샷을 돌려준다 (R5·R6·R8).
#[tauri::command]
pub async fn notion_todo_toggle(
    page_id: String,
    block_id: String,
    checked: bool,
    page_title: String,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let client = NotionClient::new(NOTION_API_BASE);
    let write = client
        .set_todo_checked(&access.token, &block_id, checked)
        .await;
    finish_write(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &today_local(),
        write,
    )
    .await
}

/// to_do 텍스트를 교체하고 재조회 스냅샷을 돌려준다 (R5·R6·R8).
#[tauri::command]
pub async fn notion_todo_edit(
    page_id: String,
    block_id: String,
    text: String,
    page_title: String,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let client = NotionClient::new(NOTION_API_BASE);
    let write = client.set_todo_text(&access.token, &block_id, &text).await;
    finish_write(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &today_local(),
        write,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_json을_설정으로_관대하게_읽는다() {
        // 정상 형태
        let v = json!({
            "database_id": "25b8f0d4-c2f9-8080-abcd-1234567890ab",
            "data_source_id": "ffffeeee-dddd-cccc-bbbb-aaaa00001111",
            "title": "계획표"
        });
        let s = settings_from_json(Some(&v));
        assert_eq!(
            s,
            NotionSettings {
                database_id: Some("25b8f0d4-c2f9-8080-abcd-1234567890ab".to_string()),
                data_source_id: Some("ffffeeee-dddd-cccc-bbbb-aaaa00001111".to_string()),
                title: Some("계획표".to_string()),
            }
        );

        // 키 없음(None)·부분 형태·타입 불일치는 전부 None으로 읽는다
        assert_eq!(settings_from_json(None), NotionSettings::default());
        let partial = json!({ "database_id": "abc" });
        let s = settings_from_json(Some(&partial));
        assert_eq!(s.database_id, Some("abc".to_string()));
        assert_eq!(s.data_source_id, None);
        assert_eq!(s.title, None);
        let wrong_type = json!({ "database_id": 123, "title": null });
        assert_eq!(
            settings_from_json(Some(&wrong_type)),
            NotionSettings::default()
        );
    }

    #[test]
    fn 설정_json_왕복이_무손실이다() {
        let s = NotionSettings {
            database_id: Some("25b8f0d4-c2f9-8080-abcd-1234567890ab".to_string()),
            data_source_id: None,
            title: Some("계획표".to_string()),
        };
        assert_eq!(settings_from_json(Some(&settings_to_json(&s))), s);
        // 기본값(전부 None)도 왕복 보존
        let empty = NotionSettings::default();
        assert_eq!(settings_from_json(Some(&settings_to_json(&empty))), empty);
    }

    #[test]
    fn 설정_json에_토큰_관련_필드가_없다() {
        // 토큰은 Keychain에만 — store 직렬화 형태에 token 계열 키가 존재하지 않는다
        let v = settings_to_json(&NotionSettings::default());
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["data_source_id", "database_id", "title"]);
    }

    // ------------------------------------------------------------------
    // U4 — todo 커맨드 순수 로직 (missing 판정, 스냅샷·결과 직렬화 형태)
    // ------------------------------------------------------------------

    #[test]
    fn 미연결_missing_목록이_부족한_항목만_순서대로_담는다() {
        // (토큰, database_id, data_source_id) 존재 여부 조합 4가지
        assert!(todo_missing(true, true, true).is_empty());
        assert_eq!(todo_missing(false, true, true), vec!["token"]);
        assert_eq!(
            todo_missing(true, false, false),
            vec!["database", "data_source"]
        );
        assert_eq!(
            todo_missing(false, false, false),
            vec!["token", "database", "data_source"]
        );
    }

    #[test]
    fn todo_snapshot이_state_태그와_snake_case로_직렬화된다() {
        // not_connected — missing은 snake_case 문자열 배열 (기존 Missing enum 미사용)
        let v = serde_json::to_value(TodoSnapshot::NotConnected {
            missing: vec!["token".to_string(), "data_source".to_string()],
        })
        .unwrap();
        assert_eq!(
            v,
            json!({ "state": "not_connected", "missing": ["token", "data_source"] })
        );

        // no_page — 날짜만 싣는다
        let v = serde_json::to_value(TodoSnapshot::NoPage {
            date: "2026-08-09".to_string(),
        })
        .unwrap();
        assert_eq!(v, json!({ "state": "no_page", "date": "2026-08-09" }));

        // loaded — date·page_id·title·items 전 필드
        let v = serde_json::to_value(TodoSnapshot::Loaded {
            date: "2026-08-09".to_string(),
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "[TODO]".to_string(),
            items: vec![TodoItem {
                id: "block-1".to_string(),
                text: "테스트 항목".to_string(),
                checked: true,
            }],
        })
        .unwrap();
        assert_eq!(
            v,
            json!({
                "state": "loaded",
                "date": "2026-08-09",
                "page_id": "aaaabbbb-cccc-dddd-eeee-ffff00001111",
                "title": "[TODO]",
                "items": [{ "id": "block-1", "text": "테스트 항목", "checked": true }]
            })
        );
    }

    #[test]
    fn todo_outcome의_notice가_없으면_null_있으면_문자열로_직렬화된다() {
        let ok = TodoOutcome {
            snapshot: TodoSnapshot::NoPage {
                date: "2026-08-09".to_string(),
            },
            notice: None,
        };
        let v = serde_json::to_value(&ok).unwrap();
        assert_eq!(v.get("notice"), Some(&Value::Null));
        assert_eq!(v["snapshot"]["state"], json!("no_page"));

        let stale = TodoOutcome {
            snapshot: TodoSnapshot::NoPage {
                date: "2026-08-09".to_string(),
            },
            notice: Some(TODO_STALE_NOTICE.to_string()),
        };
        let v = serde_json::to_value(&stale).unwrap();
        assert_eq!(v["notice"], json!(TODO_STALE_NOTICE));
    }

    #[test]
    fn 블록_소실_안내문구는_db_notfound_메시지와_다르다() {
        // DB용 NotFound 메시지("Integration 연결 확인…")를 블록 소실 안내에 재사용하지 않는다
        assert_ne!(TODO_STALE_NOTICE, ConnectError::NotFound.message());
    }
}
