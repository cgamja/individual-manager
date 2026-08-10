//! Notion 브릿지 — 코어(notion)를 앱에 연결한다: Keychain 토큰 보관,
//! store(`settings.json`) 설정 영속, 연결 커맨드 5종.
//! 토큰 값은 Keychain에만 존재한다 — 응답·로그·store·에러 어디에도 넣지 않는다.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::notion::{
    date_only, determine_connection_state, parse_database_id, ConnectError, ConnectionState,
    DayPage, NotionClient, RowInWindow, TodoItem, NOTION_API_BASE,
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
        /// 이 스냅샷이 "오늘" 것인가 — 프론트가 과거·미래 조회 화면을 구분한다.
        /// "오늘"은 Rust가 소유하므로(KTD3) 판정도 Rust가 실어 보낸다.
        is_today: bool,
    },
    Loaded {
        date: String,
        page_id: String,
        title: String,
        items: Vec<TodoItem>,
        is_today: bool,
        /// 이 행의 `수행도` — 없으면(null) 프론트가 "미지정"으로 그린다 (R1).
        performance: Option<String>,
        /// 이 행의 시작 날짜 — 모르는 경로(생성 직후·확인되지 않은 에코)면 null (R10).
        /// 끝만 보여주면 그 행이 보고 있는 날짜 전부터 시작했는지 알 수 없다.
        range_start: Option<String>,
        /// 행이 날짜 범위를 덮을 때의 끝 날짜 — 하루 행이면 null (R10).
        /// 수행도가 하루가 아니라 이 구간 전체에 적용된다는 사실을 카드가 보여야 한다.
        range_end: Option<String>,
    },
}

/// 스냅샷이 나르는 페이지 메타 (KTD1) — children 재조회는 이 값들을 주지 않으므로
/// 호출자가 아는 값을 그대로 싣는다(`page_title`과 같은 방식). 확인되지 않은 값은
/// 절대 싣지 않는다 (R9).
///
/// 쓰기 커맨드의 인자이기도 하다 — 프론트는 세 값을 낱개로 넘기지 않고 이 객체
/// 하나(`meta`)로 넘긴다. 생략하면 전부 미지정(`default`)으로 본다.
#[derive(Clone, Default, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMeta {
    pub performance: Option<String>,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
}

impl PageMeta {
    fn new(
        performance: Option<String>,
        range_start: Option<String>,
        range_end: Option<String>,
    ) -> Self {
        Self {
            performance,
            range_start,
            range_end,
        }
    }
}

/// 쓰기 커맨드의 반환 — 재조회한 스냅샷(R6)과, 블록 소실·충돌을 오류 대신
/// 안내로 처리할 때(R8)의 문구.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct TodoOutcome {
    /// `None`이면 "재조회 실패 — 프론트는 기존 목록을 유지하고 notice만 표시"라는 뜻이다.
    pub snapshot: Option<TodoSnapshot>,
    pub notice: Option<String>,
}

/// 블록 소실(404)·편집 충돌(409) 시 자동 재조회와 함께 싣는 안내 문구.
/// DB용 NotFound 메시지와 별개다 — 블록 수준 문제를 DB 연결 문제로 오독하게 하지 않는다.
const TODO_STALE_NOTICE: &str =
    "할 일이 원격에서 바뀌어 목록을 새로 불러왔습니다. 다시 시도해 주세요.";

/// 쓰기 커맨드가 미연결 상태에서 호출됐을 때의 오류 메시지 (스냅샷 없는 오류 경로).
const TODO_NOT_CONNECTED_ERROR: &str = "Notion 연결이 필요합니다. 설정을 확인해 주세요";

/// 만들기 클릭 시 오늘 행이 이미 있을 때(stale no_page 화면) 기존 페이지를 불러오며 싣는 안내.
const TODO_PAGE_EXISTS_NOTICE: &str = "오늘 페이지가 이미 있어 불러왔습니다.";

/// 페이지 생성은 성공했지만 직후 목록 조회가 실패했을 때의 안내 —
/// Err로 돌리면 프론트가 page_id를 잃고 만들기 버튼 재클릭이 페이지를 중복 생성한다.
const TODO_CREATED_FETCH_FAILED_NOTICE: &str =
    "페이지는 만들어졌지만 목록 조회에 실패했습니다. 새로고침해 주세요.";

/// 쓰기(추가·토글·편집)는 반영됐지만 직후 재조회가 실패했을 때의 안내 —
/// Err로 돌리면 프론트가 쓰기 실패로 오인하고, 재시도(append 비멱등)가 중복 항목을 만든다.
const TODO_WRITE_REFRESH_FAILED_NOTICE: &str =
    "변경은 반영됐지만 목록 조회에 실패했습니다. 새로고침해 주세요.";

/// 같은 값 재선택이라 아무것도 쓰지 않은 경로(R4)에서 재조회만 실패했을 때의 안내 —
/// 쓰기 경로 문구를 재사용하면 PATCH가 없었는데 "변경은 반영됐다"고 거짓말하게 된다.
const TODO_UNCHANGED_REFRESH_FAILED_NOTICE: &str =
    "이미 같은 값이라 변경하지 않았습니다. 목록 조회에 실패했으니 새로고침해 주세요.";

/// page_id로 연 행이 사라졌을 때(404) 날짜 조회로 폴백하며 싣는 안내.
const TODO_OPEN_FALLBACK_NOTICE: &str = "행을 찾지 못해 그 날짜를 다시 조회했습니다.";

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
/// `today`는 커맨드가 `today_local()`로 주입한다 (KTD3 — 순수 로직은 파라미터로 받는다).
async fn snapshot_by_date(
    base_url: &str,
    access: &TodoAccess,
    date: &str,
    today: &str,
) -> Result<TodoSnapshot, ConnectError> {
    let client = NotionClient::new(base_url);
    match client
        .find_page_by_date(&access.token, &access.data_source_id, date)
        .await?
    {
        None => Ok(TodoSnapshot::NoPage {
            date: date.to_string(),
            is_today: date == today,
        }),
        Some(DayPage {
            page_id,
            title,
            performance,
            start,
            end,
        }) => {
            let items = client.fetch_todos(&access.token, &page_id).await?;
            Ok(TodoSnapshot::Loaded {
                date: date.to_string(),
                page_id,
                title,
                items,
                is_today: date == today,
                // 날짜 쿼리 경로는 이미 받아 온 행에서 값을 뽑는다 — 추가 GET 0 (KTD1)
                performance,
                // 적용 구간은 date-only로 잘라 싣는다 — 원문이 datetime이면 화면에
                // 타임스탬프가 새고 시작·끝 동일 판정도 어긋난다 (KTD3의 date-only 원칙)
                range_start: Some(date_only(&start).to_string()),
                range_end: end.map(|e| date_only(&e).to_string()),
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
    today: &str,
    meta: &PageMeta,
) -> Result<TodoSnapshot, ConnectError> {
    let client = NotionClient::new(base_url);
    match client.fetch_todos(&access.token, page_id).await {
        Ok(items) => Ok(TodoSnapshot::Loaded {
            date: date.to_string(),
            page_id: page_id.to_string(),
            title: page_title.to_string(),
            items,
            is_today: date == today,
            performance: meta.performance.clone(),
            range_start: meta.range_start.clone(),
            range_end: meta.range_end.clone(),
        }),
        // 날짜 폴백은 행을 다시 읽으므로 메타도 원격 값으로 새로 실린다
        Err(ConnectError::NotFound) => snapshot_by_date(base_url, access, date, today).await,
        Err(e) => Err(e),
    }
}

/// 블록 쓰기 결과의 공통 종단 (R6·R8) — 페이지 메타를 바꾸지 않는
/// 쓰기(추가·토글·편집)용. 성공이든 안내(404·409)든 같은 메타를 싣는다.
/// 메타를 바꾸는 쓰기만 `finish_write_split`을 직접 쓴다.
#[allow(clippy::too_many_arguments)]
async fn finish_write(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
    today: &str,
    meta: &PageMeta,
    write: Result<(), ConnectError>,
) -> Result<TodoOutcome, String> {
    finish_write_split(
        base_url, access, page_id, page_title, date, today, meta, meta, write,
    )
    .await
}

/// 쓰기 결과 → 재조회·안내·오류 분기의 본체: 성공 → 재조회 스냅샷,
/// 블록 소실(404)·편집 충돌(409) → Err 대신 재조회 스냅샷 + 안내 문구,
/// 그 외(네트워크·인증·한도·형식) → 스냅샷 없는 오류로 전달.
///
/// 페이지 메타는 두 갈래로 받는다 (R9): `written`은 쓰기가 확인됐을 때 싣는 값,
/// `previous`는 404·409를 안내로 흡수해 저장이 확인되지 않았을 때 싣는 직전 표시값이다.
#[allow(clippy::too_many_arguments)]
async fn finish_write_split(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
    today: &str,
    written: &PageMeta,
    previous: &PageMeta,
    write: Result<(), ConnectError>,
) -> Result<TodoOutcome, String> {
    let (notice, meta) = match write {
        Ok(()) => (None, written),
        Err(ConnectError::NotFound | ConnectError::Conflict) => {
            (Some(TODO_STALE_NOTICE.to_string()), previous)
        }
        Err(e) => return Err(e.message()),
    };
    // 쓰기는 이미 반영됐다 — 재조회 실패를 Err로 돌리면 재클릭이 중복 쓰기를 만든다.
    // snapshot 없이 안내만 싣고, 프론트는 기존 목록을 유지한다.
    match snapshot_after_write(base_url, access, page_id, page_title, date, today, meta).await {
        Ok(snapshot) => Ok(TodoOutcome {
            snapshot: Some(snapshot),
            notice,
        }),
        Err(_) => Ok(TodoOutcome {
            snapshot: None,
            notice: Some(TODO_WRITE_REFRESH_FAILED_NOTICE.to_string()),
        }),
    }
}

/// 오늘 페이지의 to_do 목록 스냅샷을 돌려준다 (R1·R7).
#[tauri::command]
pub async fn notion_todo_list(app: AppHandle) -> Result<TodoSnapshot, String> {
    match todo_access(&app).await? {
        Err(missing) => Ok(TodoSnapshot::NotConnected { missing }),
        Ok(access) => {
            let today = today_local();
            snapshot_by_date(NOTION_API_BASE, &access, &today, &today)
                .await
                .map_err(|e| e.message())
        }
    }
}

/// 페이지 생성의 코어 (`finish_write` 전례 — AppHandle 무의존, 날짜 주입, 테스트 가능).
/// 생성 전에 날짜를 재확인한다 — stale no_page 화면에서의 클릭이 같은 날짜 행을
/// 중복 생성하지 않게, 이미 행이 있으면 생성 없이 그 페이지를 불러온다.
/// 생성 후 목록 조회 실패는 Err 대신 빈 목록 스냅샷 + 안내로 돌려준다 —
/// 골격 페이지에는 to_do 블록이 없어 빈 목록이 정확하고, page_id를 보존해야
/// 만들기 버튼 재클릭이 페이지를 중복 생성하지 않는다.
async fn create_page_outcome(
    base_url: &str,
    access: &TodoAccess,
    date: &str,
    today: &str,
) -> Result<TodoOutcome, String> {
    let client = NotionClient::new(base_url);
    // 사전 확인 실패는 그대로 Err — 아직 아무것도 만들지 않아 재시도가 안전하다
    if let Some(DayPage {
        page_id,
        title,
        performance,
        start,
        end,
    }) = client
        .find_page_by_date(&access.token, &access.data_source_id, date)
        .await
        .map_err(|e| e.message())?
    {
        let items = client
            .fetch_todos(&access.token, &page_id)
            .await
            .map_err(|e| e.message())?;
        return Ok(TodoOutcome {
            snapshot: Some(TodoSnapshot::Loaded {
                date: date.to_string(),
                page_id,
                title,
                items,
                is_today: date == today,
                // 재확인 쿼리가 돌려준 값 — 이미 손에 있으므로 추가 조회가 없다
                performance,
                // 구간은 date-only로 잘라 싣는다 (`snapshot_by_date`와 같은 규칙) —
                // 원문이 datetime이면 화면에 타임스탬프가 새고 시작·끝 동일 판정도 어긋난다
                range_start: Some(date_only(&start).to_string()),
                range_end: end.map(|e| date_only(&e).to_string()),
            }),
            notice: Some(TODO_PAGE_EXISTS_NOTICE.to_string()),
        });
    }
    // 최신 [TODO] 행의 아이콘을 복사한다 — 부가 기능이라 조회 실패·아이콘 없음은
    // None으로 수렴하고(latest_todo_icon 내부 보장) 아이콘 없이 생성한다.
    let icon = client
        .latest_todo_icon(&access.token, &access.data_source_id)
        .await;
    let page_id = client
        .create_day_page(&access.token, &access.data_source_id, date, icon.as_ref())
        .await
        .map_err(|e| e.message())?;
    let (items, notice) = match client.fetch_todos(&access.token, &page_id).await {
        Ok(items) => (items, None),
        Err(_) => (
            Vec::new(),
            Some(TODO_CREATED_FETCH_FAILED_NOTICE.to_string()),
        ),
    };
    Ok(TodoOutcome {
        snapshot: Some(TodoSnapshot::Loaded {
            date: date.to_string(),
            page_id,
            title: "[TODO]".to_string(),
            items,
            is_today: date == today,
            // 갓 만든 하루 행 — 생성 body에 수행도를 넣지 않으므로 미지정이고 범위도 없다
            performance: None,
            range_start: None,
            range_end: None,
        }),
        notice,
    })
}

/// 오늘 행이 없을 때 `[TODO]` 골격 페이지를 만들고 그 페이지의 스냅샷을 돌려준다 (R3).
/// 날짜 재쿼리 없이 POST 응답의 page ID로 곧장 children을 조회한다 (KTD5).
#[tauri::command]
pub async fn notion_todo_create_page(app: AppHandle) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let today = today_local();
    create_page_outcome(NOTION_API_BASE, &access, &today, &today).await
}

// ---------------------------------------------------------------------------
// 행 생성·열기 커맨드 (U4) — 미래 날짜 [TODO] 행 만들기와 행 직접 열기
// ---------------------------------------------------------------------------

/// 행 생성 커맨드의 반환 — 생성 성공(created)과 겹침 차단(exists)을
/// `TodoSnapshot` 전례대로 state 태그로 구분한다 (TS 판별 용이).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CreateRowOutcome {
    Created {
        #[serde(flatten)]
        outcome: TodoOutcome,
    },
    /// 같은 제목 행이 요청 기간과 겹쳐 생성하지 않았다 — 겹친 행과 그 행
    /// 자신의 시작일을 실어 프론트가 그 행을 실제 날짜로 곧장 열 수 있게 한다.
    Exists {
        page_id: String,
        title: String,
        date: String,
        /// 겹친 행의 현재 수행도 — 프론트의 "기존 행 열기"가 그대로 넘긴다.
        performance: Option<String>,
        /// 겹친 행의 적용 구간(date-only) — 열기 경로도 이 값을 되실어야
        /// 여러 날을 덮는 행에서 R10 구간 표시가 사라지지 않는다.
        range_start: Option<String>,
        range_end: Option<String>,
    },
}

/// date-only 구간 겹침 — `[a_start, a_end??a_start] ∩ [b_start, b_end??b_start] ≠ ∅`.
/// 비교는 notion.rs `date_only` 규칙을 그대로 쓴다.
fn ranges_overlap(a_start: &str, a_end: Option<&str>, b_start: &str, b_end: Option<&str>) -> bool {
    date_only(a_start) <= date_only(b_end.unwrap_or(b_start))
        && date_only(b_start) <= date_only(a_end.unwrap_or(a_start))
}

/// 그 날짜와 겹치는 `[TODO]` 행을 찾는다 — 제목 필터 쿼리 한 번
/// (`find_rows_by_title`, 날짜 하한 없음)으로 같은 제목 행 전체를 받아
/// 하루 겹침을 클라이언트에서 판정한다. 날짜 창 조회(31일 하한)는 훨씬 전에
/// 시작한 legacy 범위 행을 조용히 놓쳐 중복 생성을 허용했다.
async fn overlapping_row(
    client: &NotionClient,
    access: &TodoAccess,
    title: &str,
    date: &str,
) -> Result<Option<RowInWindow>, ConnectError> {
    let rows = client
        .find_rows_by_title(&access.token, &access.data_source_id, title)
        .await?;
    Ok(rows
        .into_iter()
        .find(|row| ranges_overlap(&row.start, row.end.as_deref(), date, None)))
}

/// `[TODO]` 행 생성의 코어 (`create_page_outcome` 전례 — AppHandle 무의존, today 주입).
/// 하루 골격(공부/기타 헤딩) + 최신 행 아이콘 복사. 그 날짜를 덮는 `[TODO]` 행이
/// 이미 있으면 생성하지 않고 알린다.
async fn create_row_outcome(
    base_url: &str,
    access: &TodoAccess,
    start: &str,
    today: &str,
) -> Result<CreateRowOutcome, String> {
    let client = NotionClient::new(base_url);
    // 겹침 검사 실패는 그대로 Err — 아직 아무것도 만들지 않아 재시도가 안전하다
    if let Some(row) = overlapping_row(&client, access, "[TODO]", start)
        .await
        .map_err(|e| e.message())?
    {
        // 겹친 행 자신의 시작일 (date-only) — 요청 start를 실으면 프론트가
        // 그 행과 무관한 날짜로 열게 된다
        let start = date_only(&row.start).to_string();
        return Ok(CreateRowOutcome::Exists {
            page_id: row.page_id,
            date: start.clone(),
            title: row.title,
            performance: row.performance,
            // 구간도 date-only로 잘라 싣는다 (`snapshot_by_date`와 같은 규칙)
            range_start: Some(start),
            range_end: row.end.as_deref().map(|e| date_only(e).to_string()),
        });
    }

    // 최신 [TODO] 행 아이콘 복사 — 실패·없음은 None으로 수렴한다(latest_todo_icon
    // 내부 보장). 아이콘 없이도 생성은 진행된다.
    let icon = client
        .latest_todo_icon(&access.token, &access.data_source_id)
        .await;
    let page_id = client
        .create_day_page(&access.token, &access.data_source_id, start, icon.as_ref())
        .await
        .map_err(|e| e.message())?;
    // 생성 직후 children 조회 실패는 Err 대신 빈 목록 + 안내 — page_id를 잃으면
    // 재클릭이 행을 중복 생성한다 (create_page_outcome과 같은 정책). 스냅샷은
    // 날짜 재쿼리 없이 새 page_id로 직접 조회한다.
    let (items, notice) = match client.fetch_todos(&access.token, &page_id).await {
        Ok(items) => (items, None),
        Err(_) => (
            Vec::new(),
            Some(TODO_CREATED_FETCH_FAILED_NOTICE.to_string()),
        ),
    };
    Ok(CreateRowOutcome::Created {
        outcome: TodoOutcome {
            snapshot: Some(TodoSnapshot::Loaded {
                date: start.to_string(),
                page_id,
                title: "[TODO]".to_string(),
                items,
                is_today: start == today,
                // 갓 만든 하루 행 — 수행도 미지정, 범위 없음
                performance: None,
                range_start: None,
                range_end: None,
            }),
            notice,
        },
    })
}

/// 미래 날짜의 `[TODO]` 행을 만든다 — 그 날짜를 덮는 `[TODO]` 행이 이미 있으면
/// 만들지 않고 알린다.
#[tauri::command]
pub async fn notion_todo_create_row(
    start: String,
    app: AppHandle,
) -> Result<CreateRowOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    create_row_outcome(NOTION_API_BASE, &access, &start, &today_local()).await
}

/// 이미 아는 행을 page_id로 직접 여는 코어 — 날짜 조회는 `[TODO]` 우선 규칙
/// 때문에 같은 날짜의 특수 행을 열 수 없어 쓰지 않는다. 행이 사라진 404만
/// 날짜 조회로 폴백하고 안내를 싣는다.
async fn open_page_outcome(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
    today: &str,
    meta: &PageMeta,
) -> Result<TodoOutcome, String> {
    let client = NotionClient::new(base_url);
    match client.fetch_todos(&access.token, page_id).await {
        Ok(items) => Ok(TodoOutcome {
            snapshot: Some(TodoSnapshot::Loaded {
                date: date.to_string(),
                page_id: page_id.to_string(),
                // 제목은 프론트가 목록 스냅샷에서 넘긴다 — children 조회는 제목을 주지 않는다
                title: page_title.to_string(),
                items,
                is_today: date == today,
                // 수행도·적용 구간도 제목과 같은 방식으로 프론트가 넘긴다 (KTD1)
                performance: meta.performance.clone(),
                range_start: meta.range_start.clone(),
                range_end: meta.range_end.clone(),
            }),
            notice: None,
        }),
        Err(ConnectError::NotFound) => {
            let snapshot = snapshot_by_date(base_url, access, date, today)
                .await
                .map_err(|e| e.message())?;
            Ok(TodoOutcome {
                snapshot: Some(snapshot),
                notice: Some(TODO_OPEN_FALLBACK_NOTICE.to_string()),
            })
        }
        Err(e) => Err(e.message()),
    }
}

/// 목록에서 고른 행을 page_id로 연다 — 스냅샷과 (폴백 시) 안내를 돌려준다.
/// `meta`(수행도·적용 구간)는 제목과 같이 프론트가 아는 값을 그대로 되싣는다 —
/// 모르면 생략하고, 그러면 스냅샷도 그만큼만 안다.
#[tauri::command]
pub async fn notion_todo_open_page(
    page_id: String,
    page_title: String,
    date: String,
    meta: Option<PageMeta>,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    open_page_outcome(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &date,
        &today_local(),
        &meta.unwrap_or_default(),
    )
    .await
}

/// 쓰기 커맨드 공통 — 스냅샷 날짜가 없으면 오늘로 본다. `(date, today)`를 돌려준다.
fn resolve_write_date(date: Option<String>) -> (String, String) {
    let today = today_local();
    let date = date.unwrap_or_else(|| today.clone());
    (date, today)
}

/// 추가의 코어 (`finish_write` 전례 — AppHandle 무의존, 날짜 주입, 테스트 가능).
/// `category`가 Some이면 children을 먼저 순회해 그 카테고리 섹션의 마지막 최상위
/// 블록 뒤(after)에 삽입한다 — 섹션이 비었으면 헤딩 블록 바로 뒤. 헤딩이 없거나
/// category가 None이면 끝에 붙는다(기존 동작). 앵커 탐색 실패는 그대로 Err —
/// 아직 아무것도 쓰지 않아 재시도가 안전하다.
#[allow(clippy::too_many_arguments)]
async fn add_todo_outcome(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    text: &str,
    page_title: &str,
    date: &str,
    today: &str,
    category: Option<&str>,
    meta: &PageMeta,
) -> Result<TodoOutcome, String> {
    let client = NotionClient::new(base_url);
    let after = match category {
        None => None,
        Some(category) => client
            .fetch_page_blocks(&access.token, page_id)
            .await
            .map_err(|e| e.message())?
            .anchor_for(category)
            .map(str::to_string),
    };
    let write = client
        .append_todo(&access.token, page_id, text, after.as_deref())
        .await;
    // 할 일 추가는 페이지 메타를 바꾸지 않는다 — 성공·안내 어느 쪽이든 같은 값이다
    finish_write(base_url, access, page_id, page_title, date, today, meta, write).await
}

/// 페이지에 to_do를 추가하고 재조회 스냅샷을 돌려준다 (R4·R6).
/// `category`(공부/기타)가 있으면 그 섹션 아래에, 없으면 본문 끝에 붙는다.
/// `page_title`은 프론트가 현재 스냅샷에서 넘긴다 — children 재조회는 페이지
/// 제목을 주지 않고, 날짜 재쿼리는 KTD5가 금지한다.
/// `date`는 보고 있는 스냅샷의 날짜 — 없으면 오늘(기존 호출 호환).
#[tauri::command]
pub async fn notion_todo_add(
    page_id: String,
    text: String,
    page_title: String,
    date: Option<String>,
    category: Option<String>,
    meta: Option<PageMeta>,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let (date, today) = resolve_write_date(date);
    add_todo_outcome(
        NOTION_API_BASE,
        &access,
        &page_id,
        &text,
        &page_title,
        &date,
        &today,
        category.as_deref(),
        &meta.unwrap_or_default(),
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
    date: Option<String>,
    meta: Option<PageMeta>,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let client = NotionClient::new(NOTION_API_BASE);
    let write = client
        .set_todo_checked(&access.token, &block_id, checked)
        .await;
    let (date, today) = resolve_write_date(date);
    finish_write(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &date,
        &today,
        &meta.unwrap_or_default(),
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
    date: Option<String>,
    meta: Option<PageMeta>,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let client = NotionClient::new(NOTION_API_BASE);
    let write = client.set_todo_text(&access.token, &block_id, &text).await;
    let (date, today) = resolve_write_date(date);
    finish_write(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &date,
        &today,
        &meta.unwrap_or_default(),
        write,
    )
    .await
}

/// 수행도 변경의 코어 (`add_todo_outcome` 전례 — AppHandle 무의존, 날짜 주입).
/// 쓰기가 확인됐을 때만 새 값을 싣고, 404·409를 안내로 흡수한 분기에서는
/// 호출자가 넘긴 직전 값을 그대로 유지한다 (R9).
/// 4개 고정 옵션 가드는 `set_page_performance`가 갖고 있다 — 그 오류를 그대로 전달한다(KTD2).
#[allow(clippy::too_many_arguments)]
async fn set_performance_outcome(
    base_url: &str,
    access: &TodoAccess,
    page_id: &str,
    page_title: &str,
    date: &str,
    today: &str,
    performance: &str,
    previous: &PageMeta,
) -> Result<TodoOutcome, String> {
    // 이미 그 값이면 쓸 것이 없다 (R4) — PATCH 없이 현재 스냅샷만 새로 읽어 돌려준다.
    // 카드도 같은 값 클릭을 막지만(TodoCard), 커맨드는 직접 호출될 수 있다.
    // 쓰기 경로(`finish_write`)에 태우지 않는다 — 재조회가 실패했을 때 그 경로의
    // "변경은 반영됐지만…" 안내는 PATCH가 없었던 여기서는 거짓이다.
    if previous.performance.as_deref() == Some(performance) {
        let refreshed =
            snapshot_after_write(base_url, access, page_id, page_title, date, today, previous)
                .await;
        return Ok(match refreshed {
            Ok(snapshot) => TodoOutcome {
                snapshot: Some(snapshot),
                notice: None,
            },
            Err(_) => TodoOutcome {
                snapshot: None,
                notice: Some(TODO_UNCHANGED_REFRESH_FAILED_NOTICE.to_string()),
            },
        });
    }
    let client = NotionClient::new(base_url);
    let write = client
        .set_page_performance(&access.token, page_id, performance)
        .await;
    // 적용 구간은 이 쓰기로 바뀌지 않는다 — 직전 값을 그대로 이어 나른다
    let written = PageMeta::new(
        Some(performance.to_string()),
        previous.range_start.clone(),
        previous.range_end.clone(),
    );
    finish_write_split(
        base_url, access, page_id, page_title, date, today, &written, previous, write,
    )
    .await
}

/// 보고 있는 날짜의 행 `수행도`를 바꾸고 재조회 스냅샷을 돌려준다 (R3·R7·R9).
/// `meta`는 화면에 지금 보이는 페이지 메타 — 그 `performance`가 직전 표시값이고,
/// 저장이 확인되지 않은 경로에서 시도값(`performance` 인자) 대신 이 값이 되실린다.
#[tauri::command]
pub async fn notion_todo_set_performance(
    page_id: String,
    page_title: String,
    date: Option<String>,
    performance: String,
    meta: Option<PageMeta>,
    app: AppHandle,
) -> Result<TodoOutcome, String> {
    let access = todo_access(&app)
        .await?
        .map_err(|_| TODO_NOT_CONNECTED_ERROR.to_string())?;
    let (date, today) = resolve_write_date(date);
    set_performance_outcome(
        NOTION_API_BASE,
        &access,
        &page_id,
        &page_title,
        &date,
        &today,
        &performance,
        &meta.unwrap_or_default(),
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

        // no_page — 날짜와 is_today를 싣는다
        let v = serde_json::to_value(TodoSnapshot::NoPage {
            date: "2026-08-09".to_string(),
            is_today: true,
        })
        .unwrap();
        assert_eq!(
            v,
            json!({ "state": "no_page", "date": "2026-08-09", "is_today": true })
        );

        // loaded — date·page_id·title·items·is_today 전 필드.
        // 카테고리는 있으면 문자열, 없으면 null로 실린다 (프론트 W2 계약).
        let v = serde_json::to_value(TodoSnapshot::Loaded {
            date: "2026-08-09".to_string(),
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "[TODO]".to_string(),
            items: vec![
                TodoItem {
                    id: "block-1".to_string(),
                    text: "테스트 항목".to_string(),
                    checked: true,
                    category: Some("공부".to_string()),
                },
                TodoItem {
                    id: "block-2".to_string(),
                    text: "헤딩 전 항목".to_string(),
                    checked: false,
                    category: None,
                },
            ],
            is_today: true,
            performance: Some("완료".to_string()),
            range_start: None,
            range_end: None,
        })
        .unwrap();
        assert_eq!(
            v,
            json!({
                "state": "loaded",
                "date": "2026-08-09",
                "page_id": "aaaabbbb-cccc-dddd-eeee-ffff00001111",
                "title": "[TODO]",
                "items": [
                    { "id": "block-1", "text": "테스트 항목", "checked": true,
                      "category": "공부" },
                    { "id": "block-2", "text": "헤딩 전 항목", "checked": false,
                      "category": null }
                ],
                "is_today": true,
                "performance": "완료",
                "range_start": null,
                "range_end": null
            })
        );
    }

    #[test]
    fn 스냅샷_is_today가_날짜에_따라_직렬화된다() {
        // 오늘 날짜 스냅샷 → is_today true, 다른 날짜 → false — 값이 JSON에 그대로 실린다
        let today = TodoSnapshot::NoPage {
            date: "2026-08-09".to_string(),
            is_today: true,
        };
        assert_eq!(serde_json::to_value(&today).unwrap()["is_today"], json!(true));

        let past = TodoSnapshot::Loaded {
            date: "2026-08-01".to_string(),
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "휴일".to_string(),
            items: vec![],
            is_today: false,
            performance: None,
            range_start: None,
            range_end: None,
        };
        assert_eq!(serde_json::to_value(&past).unwrap()["is_today"], json!(false));
    }

    #[test]
    fn create_row_outcome이_state_태그로_직렬화된다() {
        // created — TodoOutcome은 flatten이라 중첩 없이 snapshot·notice가 바로 실린다
        let created = CreateRowOutcome::Created {
            outcome: TodoOutcome {
                snapshot: Some(TodoSnapshot::NoPage {
                    date: "2026-08-09".to_string(),
                    is_today: true,
                }),
                notice: None,
            },
        };
        let v = serde_json::to_value(&created).unwrap();
        assert_eq!(v["state"], json!("created"));
        assert_eq!(v["snapshot"]["state"], json!("no_page"));
        assert_eq!(v.get("notice"), Some(&Value::Null));

        // exists — 겹친 행의 page_id·제목과 판정 기준 날짜, 그리고 적용 구간
        let exists = CreateRowOutcome::Exists {
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "휴일".to_string(),
            date: "2026-08-12".to_string(),
            performance: Some("일부".to_string()),
            range_start: Some("2026-08-12".to_string()),
            range_end: Some("2026-08-14".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&exists).unwrap(),
            json!({
                "state": "exists",
                "page_id": "aaaabbbb-cccc-dddd-eeee-ffff00001111",
                "title": "휴일",
                "date": "2026-08-12",
                "performance": "일부",
                "range_start": "2026-08-12",
                "range_end": "2026-08-14"
            })
        );
    }

    #[test]
    fn todo_outcome의_notice가_없으면_null_있으면_문자열로_직렬화된다() {
        let ok = TodoOutcome {
            snapshot: Some(TodoSnapshot::NoPage {
                date: "2026-08-09".to_string(),
                is_today: true,
            }),
            notice: None,
        };
        let v = serde_json::to_value(&ok).unwrap();
        assert_eq!(v.get("notice"), Some(&Value::Null));
        assert_eq!(v["snapshot"]["state"], json!("no_page"));

        let stale = TodoOutcome {
            snapshot: Some(TodoSnapshot::NoPage {
                date: "2026-08-09".to_string(),
                is_today: true,
            }),
            notice: Some(TODO_STALE_NOTICE.to_string()),
        };
        let v = serde_json::to_value(&stale).unwrap();
        assert_eq!(v["notice"], json!(TODO_STALE_NOTICE));
    }

    #[test]
    fn 스냅샷이_수행도를_snake_case로_직렬화한다() {
        // 값 있음 — 수행도와 적용 구간 시작·끝이 snake_case 키로 실린다 (R1·R10)
        let 범위_행 = TodoSnapshot::Loaded {
            date: "2026-08-13".to_string(),
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "휴가".to_string(),
            items: vec![],
            is_today: false,
            performance: Some("일부".to_string()),
            range_start: Some("2026-08-12".to_string()),
            range_end: Some("2026-08-14".to_string()),
        };
        let v = serde_json::to_value(&범위_행).unwrap();
        assert_eq!(v["performance"], json!("일부"));
        assert_eq!(v["range_start"], json!("2026-08-12"));
        assert_eq!(v["range_end"], json!("2026-08-14"));

        // 값 없음 — 키는 남고 null로 실린다 (프론트가 "미지정"·하루 행으로 읽는다)
        let 하루_행 = TodoSnapshot::Loaded {
            date: "2026-08-09".to_string(),
            page_id: "aaaabbbb-cccc-dddd-eeee-ffff00001111".to_string(),
            title: "[TODO]".to_string(),
            items: vec![],
            is_today: true,
            performance: None,
            range_start: None,
            range_end: None,
        };
        let v = serde_json::to_value(&하루_행).unwrap();
        assert_eq!(v.get("performance"), Some(&Value::Null));
        assert_eq!(v.get("range_start"), Some(&Value::Null));
        assert_eq!(v.get("range_end"), Some(&Value::Null));
    }

    #[test]
    fn 블록_소실_안내문구는_db_notfound_메시지와_다르다() {
        // DB용 NotFound 메시지("Integration 연결 확인…")를 블록 소실 안내에 재사용하지 않는다
        assert_ne!(TODO_STALE_NOTICE, ConnectError::NotFound.message());
    }

    #[test]
    fn 생성_경로_안내문구들도_db_notfound_메시지와_다르다() {
        // 생성 경로의 두 안내(기존 행 불러옴·생성 후 조회 실패)도 오류 메시지와 구분된다
        assert_ne!(TODO_PAGE_EXISTS_NOTICE, ConnectError::NotFound.message());
        assert_ne!(
            TODO_CREATED_FETCH_FAILED_NOTICE,
            ConnectError::NotFound.message()
        );
        assert_ne!(TODO_PAGE_EXISTS_NOTICE, TODO_CREATED_FETCH_FAILED_NOTICE);
    }
}

#[cfg(test)]
mod http_tests {
    // 한국어 테스트 이름에 포함된 404·notice 같은 소문자 아닌 조합 허용 (notion.rs 전례)
    #![allow(non_snake_case)]

    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // 픽스처 — 전부 가짜 값 (실제 토큰·워크스페이스 ID 아님)
    const 가짜_토큰: &str = "secret_FAKE_TEST_TOKEN_0000";
    const 가짜_DS_ID: &str = "ffffeeee-dddd-cccc-bbbb-aaaa00001111";
    const 가짜_페이지_ID: &str = "77770000-1111-2222-3333-444455556666";
    const 가짜_새_페이지_ID: &str = "aaaa0000-1111-2222-3333-444455556666";
    const 가짜_특수_페이지_ID: &str = "bbbb0000-1111-2222-3333-444455556666";
    const 가짜_날짜: &str = "2026-08-09";

    fn 가짜_access() -> TodoAccess {
        TodoAccess {
            token: 가짜_토큰.to_string(),
            data_source_id: 가짜_DS_ID.to_string(),
        }
    }

    fn 쿼리_경로() -> String {
        format!("/v1/data_sources/{가짜_DS_ID}/query")
    }

    fn children_경로(page_id: &str) -> String {
        format!("/v1/blocks/{page_id}/children")
    }

    fn 에러_body(status: u16, code: &str) -> serde_json::Value {
        json!({ "object": "error", "status": status, "code": code, "message": "fake" })
    }

    fn to_do_블록(id: &str, text: &str, checked: bool) -> serde_json::Value {
        json!({
            "object": "block", "id": id, "type": "to_do",
            "has_children": false, "archived": false,
            "to_do": {
                "rich_text": [ { "type": "text", "plain_text": text } ],
                "checked": checked
            }
        })
    }

    fn children_응답(blocks: Vec<serde_json::Value>) -> serde_json::Value {
        json!({ "object": "list", "results": blocks, "has_more": false, "next_cursor": null })
    }

    fn 페이지_행(id: &str, 제목: &str) -> serde_json::Value {
        범위_페이지_행(id, 제목, 가짜_날짜, None)
    }

    fn 범위_페이지_행(id: &str, 제목: &str, start: &str, end: Option<&str>) -> serde_json::Value {
        json!({
            "object": "page",
            "id": id,
            "properties": {
                "날짜": { "id": "a%3Abc", "type": "date",
                          "date": { "start": start, "end": end } },
                "이름": { "id": "title", "type": "title",
                          "title": [ { "type": "text", "plain_text": 제목 } ] }
            }
        })
    }

    fn 쿼리_응답(rows: Vec<serde_json::Value>) -> serde_json::Value {
        json!({ "object": "list", "results": rows, "has_more": false, "next_cursor": null })
    }

    async fn mount_children(server: &MockServer, page_id: &str, blocks: Vec<serde_json::Value>) {
        Mock::given(method("GET"))
            .and(path(children_경로(page_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(blocks)))
            .mount(server)
            .await;
    }

    // ------------------------------------------------------------------
    // finish_write — 쓰기 결과 → 재조회·안내·오류 분기 (R6·R8)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn 쓰기_성공_후_재조회_실패는_스냅샷_없이_안내를_돌려준다() {
        let server = MockServer::start().await;
        // children 조회가 500으로 실패 (404가 아니므로 날짜 폴백 없이 오류 전파)
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&server)
            .await;

        let outcome = finish_write(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            &PageMeta::default(),
            Ok(()),
        )
        .await
        .unwrap();
        // 쓰기는 반영됐으므로 Err가 아니다 — 재시도 유도(중복 추가)를 막는다
        assert_eq!(outcome.snapshot, None);
        assert_eq!(
            outcome.notice,
            Some(TODO_WRITE_REFRESH_FAILED_NOTICE.to_string())
        );
    }

    #[tokio::test]
    async fn 쓰기_성공은_재조회_스냅샷과_notice_없음을_돌려준다() {
        let server = MockServer::start().await;
        mount_children(
            &server,
            가짜_페이지_ID,
            vec![to_do_블록("block-1", "첫째", true)],
        )
        .await;

        let outcome = finish_write(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            &PageMeta::default(),
            Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded {
                date: 가짜_날짜.to_string(),
                page_id: 가짜_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                items: vec![TodoItem {
                    id: "block-1".to_string(),
                    text: "첫째".to_string(),
                    checked: true,
                    category: None,
                }],
                // date == today(가짜_날짜) — 재조회 스냅샷에 오늘 판정이 실린다
                is_today: true,
                performance: None,
                range_start: None,
                range_end: None,
            })
        );
    }

    #[tokio::test]
    async fn 블록_소실과_충돌_쓰기는_오류_대신_재조회_스냅샷과_안내를_돌려준다() {
        let server = MockServer::start().await;
        // 404·409 두 번 모두 재조회가 일어난다
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![])))
            .expect(2)
            .mount(&server)
            .await;

        for write_err in [ConnectError::NotFound, ConnectError::Conflict] {
            let outcome = finish_write(
                &server.uri(),
                &가짜_access(),
                가짜_페이지_ID,
                "[TODO]",
                가짜_날짜,
                가짜_날짜,
                &PageMeta::default(),
                Err(write_err),
            )
            .await
            .unwrap();
            // 안내는 블록 수준 문구 — DB NotFound 오류 메시지와 다르다
            assert_eq!(outcome.notice, Some(TODO_STALE_NOTICE.to_string()));
            assert_ne!(outcome.notice, Some(ConnectError::NotFound.message()));
            assert!(matches!(outcome.snapshot, Some(TodoSnapshot::Loaded { .. })));
        }
    }

    #[tokio::test]
    async fn 일시_오류_쓰기는_재조회_없이_곧장_오류로_끝난다() {
        let server = MockServer::start().await;
        // 재조회 요청이 한 번이라도 오면 실패해야 한다
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![])))
            .expect(0)
            .mount(&server)
            .await;

        for write_err in [ConnectError::RateLimited, ConnectError::Network(None)] {
            let message = write_err.message();
            let err = finish_write(
                &server.uri(),
                &가짜_access(),
                가짜_페이지_ID,
                "[TODO]",
                가짜_날짜,
                가짜_날짜,
                &PageMeta::default(),
                Err(write_err),
            )
            .await
            .unwrap_err();
            assert_eq!(err, message);
        }
    }

    #[tokio::test]
    async fn 쓰기_후_재조회가_404면_날짜_쿼리로_폴백한다() {
        let server = MockServer::start().await;
        // 알던 page_id의 children은 404 — 페이지 자체가 사라진 상황
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 날짜 재쿼리가 새 행을 찾는다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(쿼리_응답(vec![페이지_행(가짜_새_페이지_ID, "[TODO]")])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(
            &server,
            가짜_새_페이지_ID,
            vec![to_do_블록("block-9", "새 항목", false)],
        )
        .await;

        let snapshot = snapshot_after_write(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            &PageMeta::default(),
        )
        .await
        .unwrap();
        match snapshot {
            TodoSnapshot::Loaded { page_id, items, .. } => {
                assert_eq!(page_id, 가짜_새_페이지_ID);
                assert_eq!(items.len(), 1);
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // create_page_outcome — 생성 전 재확인·생성 후 조회 실패 (R3)
    // ------------------------------------------------------------------

    fn data_source_응답() -> serde_json::Value {
        json!({
            "object": "data_source",
            "id": 가짜_DS_ID,
            "properties": {
                "이름": { "id": "title", "type": "title", "title": {} },
                "날짜": { "id": "a%3Abc", "type": "date", "date": {} },
                "수행도": { "id": "d%3Aef", "type": "select", "select": { "options": [] } }
            }
        })
    }

    #[tokio::test]
    async fn 만들기_전_기존_행이_있으면_생성_없이_그_페이지를_불러온다() {
        let server = MockServer::start().await;
        // stale no_page 화면에서 클릭 — 사전 재확인이 기존 행을 찾는다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(쿼리_응답(vec![페이지_행(가짜_페이지_ID, "[TODO]")])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(
            &server,
            가짜_페이지_ID,
            vec![to_do_블록("block-1", "첫째", false)],
        )
        .await;
        // 생성 요청은 한 번도 오면 안 된다 — 중복 행 방지가 이 테스트의 핵심
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(outcome.notice, Some(TODO_PAGE_EXISTS_NOTICE.to_string()));
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded {
                page_id,
                title,
                items,
                ..
            }) => {
                assert_eq!(page_id, 가짜_페이지_ID);
                assert_eq!(title, "[TODO]");
                assert_eq!(items.len(), 1);
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 생성_후_조회_실패는_page_id를_보존한_빈_목록과_안내를_돌려준다() {
        let server = MockServer::start().await;
        // 오늘 행 없음 → 생성 진행 (아이콘 조회와 경로가 같아 날짜 필터 body로만 매치)
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 아이콘 조회 — 이 테스트의 관심사가 아니므로 빈 결과(아이콘 없음)로 응답
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;
        // create_day_page의 스키마 조회 + 페이지 생성
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 생성 직후 children 조회는 실패한다
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_새_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .mount(&server)
            .await;

        // 생성은 성공했으므로 Err가 아니다 — page_id를 잃으면 재클릭이 중복 생성한다
        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            outcome.notice,
            Some(TODO_CREATED_FETCH_FAILED_NOTICE.to_string())
        );
        assert_eq!(
            outcome.snapshot,
            // 골격 페이지에는 to_do가 없으므로 빈 목록이 정확하다
            Some(TodoSnapshot::Loaded {
                date: 가짜_날짜.to_string(),
                page_id: 가짜_새_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                items: vec![],
                is_today: true,
                performance: None,
                range_start: None,
                range_end: None,
            })
        );
    }

    #[tokio::test]
    async fn 사전_확인_조회가_실패하면_페이지를_생성하지_않고_오류를_돌려준다() {
        let server = MockServer::start().await;
        // 재확인 쿼리 자체가 실패 — 아직 아무것도 만들지 않았으므로 Err가 안전하다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let err = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Network(Some("HTTP 500".to_string())).message());
    }

    // ------------------------------------------------------------------
    // U2 — 생성 시 최신 [TODO] 행 아이콘 복사
    // ------------------------------------------------------------------

    /// 사전 재확인(날짜 범위 창)과 아이콘 조회(제목 title 필터)는 같은 쿼리 경로를
    /// 쓴다 — body로만 구분해 매치한다. body 명세는 notion.rs U3 테스트가 소유한다.
    fn 날짜_쿼리_body() -> serde_json::Value {
        날짜_쿼리_body_창("2026-07-09", 가짜_날짜)
    }

    /// 임의 날짜의 창 쿼리 body — 하한은 조회일에서 31일 전 (notion.rs U3 소유).
    fn 날짜_쿼리_body_창(lower: &str, upper: &str) -> serde_json::Value {
        json!({
            "filter": { "and": [
                { "property": "날짜", "date": { "on_or_after": lower } },
                { "property": "날짜", "date": { "on_or_before": upper } }
            ] },
            "sorts": [ { "property": "날짜", "direction": "descending" } ],
            "page_size": 100
        })
    }

    fn 아이콘_쿼리_body() -> serde_json::Value {
        json!({
            "filter": { "property": "title", "title": { "equals": "[TODO]" } },
            "sorts": [ { "property": "날짜", "direction": "descending" } ],
            "page_size": 1
        })
    }

    /// 겹침 검사(제목 필터, 하한 없음) 쿼리 body — 아이콘 조회와 filter 형태는 같지만
    /// page_size(100 vs 1)로 구분된다. body 명세는 notion.rs U4 테스트가 소유한다.
    fn 제목_쿼리_body(제목: &str) -> serde_json::Value {
        json!({
            "filter": { "property": "title", "title": { "equals": 제목 } },
            "sorts": [ { "property": "날짜", "direction": "descending" } ],
            "page_size": 100
        })
    }

    /// data_source_응답()의 title 키("이름") 기준 생성 body — body_json 정확 일치라
    /// icon이 None이면 icon 키 부재까지 검증된다.
    fn 생성_body(icon: Option<serde_json::Value>) -> serde_json::Value {
        let mut body = json!({
            "parent": { "type": "data_source_id", "data_source_id": 가짜_DS_ID },
            "properties": {
                "이름": { "title": [ { "type": "text", "text": { "content": "[TODO]" } } ] },
                "날짜": { "date": { "start": 가짜_날짜 } }
            },
            "children": [
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "공부" } } ] } },
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "기타" } } ] } }
            ]
        });
        if let Some(icon) = icon {
            body["icon"] = icon;
        }
        body
    }

    #[tokio::test]
    async fn 생성_시_최신_TODO_아이콘이_복사된다() {
        let server = MockServer::start().await;
        // 오늘 행 없음 → 생성 진행 (날짜 필터 body로만 매치)
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 최신 [TODO] 행에 emoji 아이콘이 있다 (제목 필터 body로만 매치)
        let mut 아이콘_있는_행 = 페이지_행(가짜_페이지_ID, "[TODO]");
        아이콘_있는_행["icon"] = json!({ "type": "emoji", "emoji": "🌊" });
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![아이콘_있는_행])),
            )
            .expect(1)
            .mount(&server)
            .await;
        // create_day_page의 스키마 조회
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        // 생성 body에 복사된 아이콘이 그대로 실려야 한다
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body(Some(
                json!({ "type": "emoji", "emoji": "🌊" }),
            ))))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_새_페이지_ID, vec![]).await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(outcome.notice, None);
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded {
                date: 가짜_날짜.to_string(),
                page_id: 가짜_새_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                items: vec![],
                is_today: true,
                performance: None,
                range_start: None,
                range_end: None,
            })
        );
    }

    #[tokio::test]
    async fn 아이콘_조회가_실패해도_페이지는_생성된다() {
        // AE5 — 아이콘 조회는 부가 기능: 500이어도 icon 키 없이 생성은 진행된다
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 아이콘 조회(제목 필터)만 500으로 실패
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        // icon 키 없는 정확 일치 body — 아이콘 없이 생성됐음을 증명한다
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body(None)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_새_페이지_ID, vec![]).await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(outcome.notice, None);
        assert!(matches!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded { page_id, .. }) if page_id == 가짜_새_페이지_ID
        ));
    }

    // ------------------------------------------------------------------
    // U4 — 행 생성(create_row_outcome) · 열기(open_page_outcome) · 날짜 재조회
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn 같은_날짜에_TODO_행이_있으면_생성_없이_exists를_돌려준다() {
        let server = MockServer::start().await;
        // 겹침 검사(제목 필터 조회)가 같은 날짜의 [TODO] 행을 찾는다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(쿼리_응답(vec![페이지_행(가짜_페이지_ID, "[TODO]")])),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 생성 요청은 한 번도 오면 안 된다 (AE3)
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CreateRowOutcome::Exists {
                page_id: 가짜_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                date: 가짜_날짜.to_string(),
                performance: None,
                // 하루 행 — 시작일은 실리고 끝은 없다
                range_start: Some(가짜_날짜.to_string()),
                range_end: None,
            }
        );
    }

    #[tokio::test]
    async fn 휴일_행만_있는_날의_TODO_생성은_정상_생성된다() {
        // legacy 휴일 행이 같은 날짜에 남아 있어도 [TODO] 생성을 막지 않는다:
        // 겹침 검사는 제목 필터 조회라 [TODO] 결과가 비어 있으면 생성이 진행된다
        // (같은 날짜의 휴일 행은 서버 필터가 걸러낸다)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 아이콘 조회는 빈 결과 — icon 키 없는 골격 생성 body 정확 일치
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body(None)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_새_페이지_ID, vec![]).await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        match outcome {
            CreateRowOutcome::Created { outcome } => {
                assert_eq!(outcome.notice, None);
                assert_eq!(
                    outcome.snapshot,
                    Some(TodoSnapshot::Loaded {
                        date: 가짜_날짜.to_string(),
                        page_id: 가짜_새_페이지_ID.to_string(),
                        title: "[TODO]".to_string(),
                        items: vec![],
                        is_today: true,
                        performance: None,
                        range_start: None,
                        range_end: None,
                    })
                );
            }
            other => panic!("Created가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 같은_제목_행이_31일_이전에_시작해도_겹침이_검출된다() {
        // legacy [TODO] 범위 행 6/25~9/20 — 요청(9/15)의 31일 창(8/15~) 훨씬 밖에서
        // 시작한 행. 제목 필터 조회는 날짜 하한이 없어 이 행을 받고, 겹침 판정이
        // 생성을 차단한다.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![
                범위_페이지_행(가짜_특수_페이지_ID, "[TODO]", "2026-06-25", Some("2026-09-20")),
            ])))
            .expect(1)
            .mount(&server)
            .await;
        // 생성 요청은 한 번도 오면 안 된다 — 중복 행 방지가 이 테스트의 핵심
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), "2026-09-15", 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CreateRowOutcome::Exists {
                page_id: 가짜_특수_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                // 요청 start(9/15)가 아니라 겹친 행 자신의 시작일 — 프론트가 그 행의
                // 실제 날짜로 곧장 열 수 있어야 한다
                date: "2026-06-25".to_string(),
                performance: None,
                range_start: Some("2026-06-25".to_string()),
                range_end: Some("2026-09-20".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn TODO_생성은_골격과_복사_아이콘을_포함한다() {
        // AE2 백엔드 — create_row 경로의 [TODO] 생성도 골격 body + 최신 아이콘 복사
        let server = MockServer::start().await;
        // 겹침 검사(제목 필터, page_size 100)는 빈 결과 — 생성이 진행된다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        let mut 아이콘_있는_행 = 페이지_행(가짜_페이지_ID, "[TODO]");
        아이콘_있는_행["icon"] = json!({ "type": "emoji", "emoji": "🌊" });
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![아이콘_있는_행])),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        // 골격 children + 복사된 아이콘이 실린 생성 body 정확 일치
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body(Some(
                json!({ "type": "emoji", "emoji": "🌊" }),
            ))))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_새_페이지_ID, vec![]).await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert!(matches!(outcome, CreateRowOutcome::Created { .. }));
    }

    // ------------------------------------------------------------------
    // U4 — 생성·열기 체인의 오류 경로 (계약 고정)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn 겹침_검사_쿼리_실패는_생성_없이_오류를_돌려준다() {
        // 겹침 검사(제목 필터 조회) 자체가 실패 — 아직 아무것도 만들지 않았으므로
        // Err가 안전하다 (재시도해도 중복이 생기지 않는다)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let err = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Network(Some("HTTP 500".to_string())).message());
    }

    #[tokio::test]
    async fn create_row의_생성_POST_실패는_오류로_전파된다() {
        // 겹침 검사는 통과(빈 결과)했지만 생성 POST가 500 — Exists도 Created도 아닌
        // 순수 오류로 전파된다 (프론트가 실패를 알고 재시도할 수 있어야 한다)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 아이콘 조회 — 이 테스트의 관심사가 아니므로 빈 결과(아이콘 없음)로 응답
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;
        // create_day_page의 스키마 조회는 성공한다 — 실패 지점은 생성 POST 하나다
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;

        let err = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Network(Some("HTTP 500".to_string())).message());
    }

    #[tokio::test]
    async fn TODO_행_생성_후_조회_실패는_page_id_보존_안내를_돌려준다() {
        // 생성은 성공했으므로 Err가 아니다 — page_id를 잃으면 재클릭이 행을 중복
        // 생성한다. 골격 페이지에는 to_do가 없으므로 빈 목록이 정확하다.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        // 아이콘 조회 — 이 테스트의 관심사가 아니므로 빈 결과(아이콘 없음)로 응답
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 생성 직후 children 조회는 실패한다
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_새_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .mount(&server)
            .await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        match outcome {
            CreateRowOutcome::Created { outcome } => {
                assert_eq!(
                    outcome.notice,
                    Some(TODO_CREATED_FETCH_FAILED_NOTICE.to_string())
                );
                assert_eq!(
                    outcome.snapshot,
                    Some(TodoSnapshot::Loaded {
                        date: 가짜_날짜.to_string(),
                        page_id: 가짜_새_페이지_ID.to_string(),
                        title: "[TODO]".to_string(),
                        items: vec![],
                        is_today: true,
                        performance: None,
                        range_start: None,
                        range_end: None,
                    })
                );
            }
            other => panic!("Created가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn open_page_폴백_조회_실패는_오류로_전파된다() {
        // 행이 사라진 404 → 날짜 조회 폴백까지 실패 — 스냅샷 없이 오류로 끝난다
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;

        let err = open_page_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "휴일",
            가짜_날짜,
            가짜_날짜,
            &PageMeta::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(err, ConnectError::Network(Some("HTTP 500".to_string())).message());
    }

    #[tokio::test]
    async fn open_page가_그_페이지_스냅샷을_돌려준다() {
        // 같은 날짜에 [TODO]가 있어도 날짜 조회 없이 page_id로 직접 연다 —
        // [TODO] 우선 규칙이 특수 행을 가리는 문제를 피한다
        let server = MockServer::start().await;
        mount_children(
            &server,
            가짜_특수_페이지_ID,
            vec![to_do_블록("block-1", "짐 싸기", false)],
        )
        .await;

        let outcome = open_page_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_특수_페이지_ID,
            "휴일",
            "2026-08-03",
            가짜_날짜,
            &PageMeta::new(
                Some("일부".to_string()),
                Some("2026-08-01".to_string()),
                Some("2026-08-05".to_string()),
            ),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded {
                date: "2026-08-03".to_string(),
                page_id: 가짜_특수_페이지_ID.to_string(),
                // 프론트가 넘긴 제목이 유지된다 — children 조회는 제목을 주지 않는다
                title: "휴일".to_string(),
                items: vec![TodoItem {
                    id: "block-1".to_string(),
                    text: "짐 싸기".to_string(),
                    checked: false,
                    category: None,
                }],
                is_today: false,
                // 수행도·적용 구간도 제목과 같이 프론트가 넘긴 값이 그대로 유지된다
                performance: Some("일부".to_string()),
                range_start: Some("2026-08-01".to_string()),
                range_end: Some("2026-08-05".to_string()),
            })
        );
    }

    #[tokio::test]
    async fn open_page가_사라진_페이지면_날짜로_폴백한다() {
        let server = MockServer::start().await;
        // 열려던 행의 children이 404 — 행이 원격에서 삭제된 상황
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 날짜 조회 폴백이 그 날짜의 다른 행을 찾는다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(쿼리_응답(vec![페이지_행(가짜_새_페이지_ID, "[TODO]")])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(
            &server,
            가짜_새_페이지_ID,
            vec![to_do_블록("block-9", "새 항목", false)],
        )
        .await;

        let outcome = open_page_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "휴일",
            가짜_날짜,
            가짜_날짜,
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, Some(TODO_OPEN_FALLBACK_NOTICE.to_string()));
        assert!(matches!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded { page_id, is_today, .. })
                if page_id == 가짜_새_페이지_ID && is_today
        ));
    }

    #[tokio::test]
    async fn 쓰기_커맨드가_전달된_날짜로_재조회한다() {
        // 쓰기 커맨드는 date 인자를 finish_write로 넘긴다 — 그 날짜가 재조회
        // (404 폴백의 날짜 쿼리) 창을 결정하는지 body 정확 일치로 검증한다.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .expect(1)
            .mount(&server)
            .await;
        // 오늘(가짜_날짜)이 아니라 전달된 2026-08-03의 창으로 조회돼야 한다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body_창("2026-07-03", "2026-08-03")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = finish_write(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "휴일",
            "2026-08-03",
            가짜_날짜,
            &PageMeta::default(),
            Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::NoPage {
                date: "2026-08-03".to_string(),
                is_today: false,
            })
        );
    }

    // ------------------------------------------------------------------
    // 카테고리 삽입 (add_todo_outcome) — 섹션 마지막 블록 뒤 after 삽입
    // ------------------------------------------------------------------

    fn heading_블록(id: &str, text: &str) -> serde_json::Value {
        json!({
            "object": "block", "id": id, "type": "heading_3",
            "has_children": false, "archived": false,
            "heading_3": { "rich_text": [ { "type": "text", "plain_text": text } ] }
        })
    }

    /// 추가 PATCH body — after가 Some이면 최상위 after 키가 실리고, None이면
    /// 키 자체가 없다 (body_json 정확 일치가 부재까지 검증한다).
    fn 추가_body(text: &str, after: Option<&str>) -> serde_json::Value {
        let mut body = json!({
            "children": [ {
                "object": "block",
                "type": "to_do",
                "to_do": {
                    "rich_text": [ { "type": "text", "text": { "content": text } } ],
                    "checked": false
                }
            } ]
        });
        if let Some(after) = after {
            body["after"] = json!(after);
        }
        body
    }

    #[tokio::test]
    async fn 카테고리_삽입은_섹션_마지막_블록_뒤에_after로_붙는다() {
        let server = MockServer::start().await;
        // 공부 섹션의 마지막 최상위 블록은 block-b — 그 뒤(after)에 삽입돼야 한다.
        // children GET은 앵커 탐색 1회 + 쓰기 후 재조회 1회 = 정확히 2회.
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![
                heading_블록("block-h-study", "공부"),
                to_do_블록("block-a", "영어 단어", false),
                to_do_블록("block-b", "알고리즘 1문제", false),
                heading_블록("block-h-etc", "기타"),
                to_do_블록("block-c", "장보기", false),
            ])))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(children_경로(가짜_페이지_ID)))
            .and(body_json(추가_body("한국사 강의", Some("block-b"))))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "list", "results": [] })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let outcome = add_todo_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "한국사 강의",
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            Some("공부"),
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        // 재조회 스냅샷의 카테고리 태깅까지 함께 확인한다
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded { items, .. }) => {
                assert_eq!(
                    items.iter().map(|t| t.category.as_deref()).collect::<Vec<_>>(),
                    vec![Some("공부"), Some("공부"), Some("기타")]
                );
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 빈_섹션은_헤딩_블록_뒤에_삽입한다() {
        let server = MockServer::start().await;
        // 공부 섹션에 블록이 하나도 없다 — 앵커는 헤딩 블록 자신의 id
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![
                heading_블록("block-h-study", "공부"),
                heading_블록("block-h-etc", "기타"),
                to_do_블록("block-c", "장보기", false),
            ])))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(children_경로(가짜_페이지_ID)))
            .and(body_json(추가_body("영어 단어", Some("block-h-study"))))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "list", "results": [] })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let outcome = add_todo_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "영어 단어",
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            Some("공부"),
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        assert!(matches!(outcome.snapshot, Some(TodoSnapshot::Loaded { .. })));
    }

    #[tokio::test]
    async fn 헤딩이_없으면_끝에_append한다() {
        let server = MockServer::start().await;
        // 대상 카테고리 헤딩이 페이지에 없다 — after 없이 끝에 붙는다(기존 동작 폴백).
        // body_json 정확 일치가 after 키 부재까지 검증한다.
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![
                to_do_블록("block-a", "영어 단어", false),
            ])))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(children_경로(가짜_페이지_ID)))
            .and(body_json(추가_body("한국사 강의", None)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "list", "results": [] })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let outcome = add_todo_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "한국사 강의",
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            Some("공부"),
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
    }

    #[tokio::test]
    async fn 카테고리_미지정_추가는_기존_동작을_유지한다() {
        let server = MockServer::start().await;
        // category 없음 — 앵커 탐색 없이 곧장 끝에 붙는다: children GET은
        // 쓰기 후 재조회 1회뿐이어야 한다.
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![
                to_do_블록("block-a", "영어 단어", false),
            ])))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(children_경로(가짜_페이지_ID)))
            .and(body_json(추가_body("장보기", None)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "list", "results": [] })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let outcome = add_todo_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "장보기",
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            None,
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        assert!(matches!(outcome.snapshot, Some(TodoSnapshot::Loaded { .. })));
    }

    // ------------------------------------------------------------------
    // 수행도 (U2) — 스냅샷 적재·변경 커맨드 코어 (R1·R2·R9·R10, KTD1·KTD2)
    // ------------------------------------------------------------------

    /// 수행도 select 값이 채워진 행 픽스처 — 값이 없는 행은 `페이지_행`을 그대로 쓴다.
    fn 수행도_행(id: &str, 제목: &str, 수행도: &str) -> serde_json::Value {
        let mut row = 페이지_행(id, 제목);
        row["properties"]["수행도"] =
            json!({ "id": "d%3Aef", "type": "select", "select": { "name": 수행도 } });
        row
    }

    /// 수행도 PATCH body — 부분 properties 하나뿐 (body 명세는 notion.rs U1이 소유한다).
    fn 수행도_body(값: &str) -> serde_json::Value {
        json!({ "properties": { "수행도": { "select": { "name": 값 } } } })
    }

    #[tokio::test]
    async fn 날짜_조회_스냅샷이_행의_수행도를_싣는다() {
        // 날짜 쿼리 경로는 이미 받아 온 행 JSON에서 값을 뽑는다 — 추가 GET 0 (KTD1)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![수행도_행(
                    가짜_페이지_ID,
                    "[TODO]",
                    "일부",
                )])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_페이지_ID, vec![]).await;

        let snapshot = snapshot_by_date(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            snapshot,
            TodoSnapshot::Loaded {
                date: 가짜_날짜.to_string(),
                page_id: 가짜_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                items: vec![],
                is_today: true,
                performance: Some("일부".to_string()),
                // 하루 행 — 시작일은 실리지만 끝이 없어 카드는 구간을 그리지 않는다
                range_start: Some(가짜_날짜.to_string()),
                range_end: None,
            }
        );
    }

    #[tokio::test]
    async fn 범위_행_스냅샷이_적용_구간_끝을_싣는다() {
        // AE8 — 8/13을 보는데 덮는 행이 8/12~8/14: 사흘 전체가 대상임을 카드가 알아야 한다
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![
                범위_페이지_행(
                    가짜_특수_페이지_ID,
                    "휴가",
                    "2026-08-12",
                    Some("2026-08-14"),
                ),
            ])))
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_특수_페이지_ID, vec![]).await;

        let snapshot = snapshot_by_date(&server.uri(), &가짜_access(), "2026-08-13", 가짜_날짜)
            .await
            .unwrap();
        match snapshot {
            TodoSnapshot::Loaded {
                range_end,
                is_today,
                ..
            } => {
                assert_eq!(range_end, Some("2026-08-14".to_string()));
                assert!(!is_today);
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 범위_행_스냅샷이_적용_구간_시작일도_싣는다() {
        // R10 — 끝만으로는 그 행이 오늘 전에 시작했는지 알 수 없다. 시작일이 없으면
        // 카드는 "8/14까지"만 그려 이미 지난 8/12·8/13까지 바뀐다는 사실을 감춘다.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![
                범위_페이지_행(
                    가짜_특수_페이지_ID,
                    "휴가",
                    "2026-08-12",
                    Some("2026-08-14"),
                ),
            ])))
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_특수_페이지_ID, vec![]).await;

        let snapshot = snapshot_by_date(&server.uri(), &가짜_access(), "2026-08-13", 가짜_날짜)
            .await
            .unwrap();
        let v = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(v["range_start"], json!("2026-08-12"));
        assert_eq!(v["range_end"], json!("2026-08-14"));
    }

    #[tokio::test]
    async fn 수행도_변경_후_스냅샷에_새_값이_실린다() {
        // AE3 — PATCH 1회 + children 재조회. 쓰기가 Ok일 때만 방금 쓴 값을 싣는다.
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .and(body_json(수행도_body("완료")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(1)
            .mount(&server)
            .await;
        mount_children(
            &server,
            가짜_페이지_ID,
            vec![to_do_블록("block-1", "첫째", true)],
        )
        .await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "완료",
            &PageMeta {
                performance: Some("일부".to_string()),
                range_start: None,
                range_end: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded { performance, .. }) => {
                assert_eq!(performance, Some("완료".to_string()));
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 전환된_날짜의_수행도를_그_날짜_행에_쓴다() {
        // AE5 — 쓰기 대상도 재조회 창도 오늘이 아니라 전달된 날짜(8/3) 기준이다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_특수_페이지_ID}")))
            .and(body_json(수행도_body("미완")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(1)
            .mount(&server)
            .await;
        // 재조회 children이 404 → 날짜 폴백이 전달된 날짜의 창으로 조회하는지 본다
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_특수_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body_창("2026-07-03", "2026-08-03")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_특수_페이지_ID,
            "휴일",
            "2026-08-03",
            가짜_날짜,
            "미완",
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::NoPage {
                date: "2026-08-03".to_string(),
                is_today: false,
            })
        );
    }

    #[tokio::test]
    async fn 갓_만든_페이지의_스냅샷은_수행도가_없다() {
        // 생성 body에 수행도를 넣지 않으므로(플랜 004) 새 행은 미지정이다 —
        // 확인되지 않은 값을 에코하지 않는다 (KTD1)
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/data_sources/{가짜_DS_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_새_페이지_ID, vec![]).await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded {
                performance,
                range_end,
                ..
            }) => {
                assert_eq!(performance, None);
                assert_eq!(range_end, None);
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 이미_있는_페이지를_불러오면_그_행의_수행도가_실린다() {
        // 생성 전 재확인이 기존 행을 찾은 조기 반환 분기 — 값은 이미 손에 있다
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![수행도_행(
                    가짜_페이지_ID,
                    "[TODO]",
                    "완료",
                )])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_페이지_ID, vec![]).await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(outcome.notice, Some(TODO_PAGE_EXISTS_NOTICE.to_string()));
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded { performance, .. }) => {
                assert_eq!(performance, Some("완료".to_string()));
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 이미_있는_페이지의_적용_구간도_date_only로_정규화된다() {
        // 생성 전 재확인이 찾은 행의 `날짜`가 datetime이면 그대로 실을 수 없다 —
        // 화면에 타임스탬프가 새고 시작·끝 동일(하루 행) 판정도 어긋난다 (KTD3).
        // 날짜 쿼리 경로(`snapshot_by_date`)와 같은 규칙이어야 한다.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![범위_페이지_행(
                    가짜_페이지_ID,
                    "[TODO]",
                    "2026-08-09T09:00:00.000+09:00",
                    Some("2026-08-09T18:00:00.000+09:00"),
                )])),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_페이지_ID, vec![]).await;

        let outcome = create_page_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded {
                range_start,
                range_end,
                ..
            }) => {
                assert_eq!(range_start, Some(가짜_날짜.to_string()));
                assert_eq!(range_end, Some(가짜_날짜.to_string()));
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 같은_수행도_값은_요청_없이_현재_스냅샷을_돌려준다() {
        // R4 방어 — 카드도 같은 값 클릭을 막지만 커맨드는 직접 호출될 수 있다.
        // PATCH는 한 번도 나가지 않고, 현재 메타 그대로의 스냅샷이 돌아온다.
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;
        mount_children(
            &server,
            가짜_페이지_ID,
            vec![to_do_블록("block-1", "첫째", true)],
        )
        .await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "일부",
            &PageMeta::new(
                Some("일부".to_string()),
                Some("2026-08-12".to_string()),
                Some("2026-08-14".to_string()),
            ),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded {
                performance,
                range_start,
                range_end,
                ..
            }) => {
                assert_eq!(performance, Some("일부".to_string()));
                assert_eq!(range_start, Some("2026-08-12".to_string()));
                assert_eq!(range_end, Some("2026-08-14".to_string()));
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn exists_응답이_그_행의_수행도를_싣는다() {
        // 프론트의 "기존 행 열기"가 넘길 수행도가 여기서 나온다
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![수행도_행(
                    가짜_페이지_ID,
                    "[TODO]",
                    "기타",
                )])),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), 가짜_날짜, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CreateRowOutcome::Exists {
                page_id: 가짜_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                date: 가짜_날짜.to_string(),
                performance: Some("기타".to_string()),
                range_start: Some(가짜_날짜.to_string()),
                range_end: None,
            }
        );
    }

    #[tokio::test]
    async fn exists_응답이_그_행의_적용_구간을_date_only로_싣는다() {
        // 여러 날을 덮는 행을 exists로 열면 R10 구간 표시가 사라지면 안 된다 —
        // 겹친 행의 시작·끝을 date-only로 잘라 실어 프론트가 그대로 되싣게 한다
        // (`snapshot_by_date`와 같은 규칙).
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(제목_쿼리_body("[TODO]")))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![
                범위_페이지_행(
                    가짜_특수_페이지_ID,
                    "[TODO]",
                    "2026-08-12T09:00:00.000+09:00",
                    Some("2026-08-14T18:00:00.000+09:00"),
                ),
            ])))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;

        let outcome = create_row_outcome(&server.uri(), &가짜_access(), "2026-08-13", 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CreateRowOutcome::Exists {
                page_id: 가짜_특수_페이지_ID.to_string(),
                title: "[TODO]".to_string(),
                date: "2026-08-12".to_string(),
                performance: None,
                range_start: Some("2026-08-12".to_string()),
                range_end: Some("2026-08-14".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn 충돌로_안내가_붙은_스냅샷은_직전_수행도를_유지한다() {
        // R9 — 409를 안내로 흡수한 분기는 저장이 확인되지 않았다: 시도값(완료)이 아니라
        // 직전 표시값(일부)이 그대로 남아야 한다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(
                ResponseTemplate::new(409).set_body_json(에러_body(409, "conflict_error")),
            )
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_페이지_ID, vec![]).await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "완료",
            &PageMeta {
                performance: Some("일부".to_string()),
                range_start: None,
                range_end: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, Some(TODO_STALE_NOTICE.to_string()));
        match outcome.snapshot {
            Some(TodoSnapshot::Loaded { performance, .. }) => {
                assert_eq!(performance, Some("일부".to_string()));
            }
            other => panic!("Loaded가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn 허용되지_않은_값은_커맨드에서_오류로_돌아온다() {
        // KTD2 — 가드는 U1의 set_page_performance가 갖고 있다: HTTP 이전에 거부된다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![])))
            .expect(0)
            .mount(&server)
            .await;

        let err = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "아주 잘함",
            &PageMeta::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(err, ConnectError::InvalidPerformance.message());
    }

    #[tokio::test]
    async fn 수행도_쓰기_실패는_오류_메시지로_전달된다() {
        // 일시 오류는 재조회 없이 곧장 오류 — 프론트가 실패를 알고 직전 값을 유지한다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(vec![])))
            .expect(0)
            .mount(&server)
            .await;

        let err = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "완료",
            &PageMeta::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(err, ConnectError::Network(Some("HTTP 500".to_string())).message());
    }

    #[tokio::test]
    async fn 수행도_쓰기_성공_후_재조회_실패는_안내를_돌려준다() {
        // 기존 TODO_WRITE_REFRESH_FAILED_NOTICE 경로가 새 커맨드에서도 그대로 동작한다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .mount(&server)
            .await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "완료",
            &PageMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome.snapshot, None);
        assert_eq!(
            outcome.notice,
            Some(TODO_WRITE_REFRESH_FAILED_NOTICE.to_string())
        );
    }

    #[tokio::test]
    async fn 무변경_경로의_재조회_실패는_변경됐다고_말하지_않는다() {
        // R4 조기 반환은 PATCH를 보내지 않는다 — 그 뒤 재조회가 실패했을 때
        // 쓰기 경로의 "변경은 반영됐지만…" 안내를 쓰면 없던 변경을 있다고 말하게 된다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_페이지_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(children_경로(가짜_페이지_ID)))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .mount(&server)
            .await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_페이지_ID,
            "[TODO]",
            가짜_날짜,
            가짜_날짜,
            "일부",
            &PageMeta::new(Some("일부".to_string()), None, None),
        )
        .await
        .unwrap();
        assert_eq!(outcome.snapshot, None);
        assert_eq!(
            outcome.notice,
            Some(TODO_UNCHANGED_REFRESH_FAILED_NOTICE.to_string())
        );
    }

    #[tokio::test]
    async fn 범위_행의_수행도를_바꿔도_적용_구간이_유지된다() {
        // R9·R10 — 성공 경로의 `written`은 수행도만 갈아 끼우고 구간은 직전 값을
        // 그대로 이어 나른다. 구간이 빠지면 저장 직후 카드에서 "8/12~8/14 적용"이 사라진다
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!("/v1/pages/{가짜_특수_페이지_ID}")))
            .and(body_json(수행도_body("완료")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(1)
            .mount(&server)
            .await;
        mount_children(&server, 가짜_특수_페이지_ID, vec![]).await;

        let outcome = set_performance_outcome(
            &server.uri(),
            &가짜_access(),
            가짜_특수_페이지_ID,
            "휴가",
            "2026-08-13",
            가짜_날짜,
            "완료",
            &PageMeta::new(
                Some("일부".to_string()),
                Some("2026-08-12".to_string()),
                Some("2026-08-14".to_string()),
            ),
        )
        .await
        .unwrap();
        assert_eq!(outcome.notice, None);
        assert_eq!(
            outcome.snapshot,
            Some(TodoSnapshot::Loaded {
                date: "2026-08-13".to_string(),
                page_id: 가짜_특수_페이지_ID.to_string(),
                title: "휴가".to_string(),
                items: vec![],
                is_today: false,
                performance: Some("완료".to_string()),
                range_start: Some("2026-08-12".to_string()),
                range_end: Some("2026-08-14".to_string()),
            })
        );
    }
}
