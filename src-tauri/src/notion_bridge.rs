//! Notion 브릿지 — 코어(notion)를 앱에 연결한다: Keychain 토큰 보관,
//! store(`settings.json`) 설정 영속, 연결 커맨드 5종.
//! 토큰 값은 Keychain에만 존재한다 — 응답·로그·store·에러 어디에도 넣지 않는다.

use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::notion::{
    determine_connection_state, parse_database_id, ConnectError, ConnectionState, NotionClient,
    NOTION_API_BASE,
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
                write_settings(
                    app,
                    &NotionSettings {
                        database_id: Some(database_id.to_string()),
                        data_source_id: None,
                        title: None,
                    },
                )?;
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
#[tauri::command]
pub async fn notion_delete_token(app: AppHandle) -> Result<ConnectionState, String> {
    on_keychain(delete_token_blocking).await?;
    let settings = read_settings(&app)?;
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
}
