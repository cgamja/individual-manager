//! Notion 연결 코어 — Tauri·HTTP 무의존 순수 판정 로직.
//! 입력 문자열 → database ID 정규화, properties JSON → 스키마 판정,
//! Notion 에러 코드 → 한국어 사용자 메시지, 토큰×DB×검증 결과 → 연결 상태.
//! 오류 메시지에는 사용자 입력 원문을 절대 인용하지 않는다
//! (토큰을 DB 필드에 잘못 붙여넣는 실수 대비).

use serde::Serialize;
use serde_json::Value;

/// 연결 과정에서 발생 가능한 오류. HTTP 계층(U2)과 UI(U3)가 공유한다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConnectError {
    /// 입력에서 32자 hex DB ID를 찾지 못함
    InvalidId,
    /// 401 — 토큰이 유효하지 않음
    Unauthorized,
    /// 404 — DB 없음 또는 Integration 미연결
    NotFound,
    /// 필수 속성 누락·타입 불일치 목록
    SchemaMismatch(Vec<String>),
    /// 429 — 재시도 후에도 한도 초과
    RateLimited,
    /// 네트워크 오류 (선택적 설명)
    Network(Option<String>),
    /// 응답 형태가 예상과 다름 (예: data source가 정확히 1개가 아님).
    /// 상세에는 사용자 입력·토큰을 절대 넣지 않는다.
    UnexpectedShape(String),
}

impl ConnectError {
    /// 사용자에게 보여줄 한국어 메시지. 입력 원문은 포함하지 않는다.
    pub fn message(&self) -> String {
        match self {
            ConnectError::InvalidId => "Notion DB 링크나 32자 ID를 붙여넣어 주세요".to_string(),
            ConnectError::Unauthorized => {
                "토큰이 유효하지 않습니다. Notion Integration 토큰을 다시 확인해 주세요".to_string()
            }
            ConnectError::NotFound => "DB를 찾을 수 없습니다. Integration이 해당 DB에 \
                 연결돼 있는지, 붙여넣은 링크가 DB 원본 링크인지 확인해 주세요"
                .to_string(),
            ConnectError::SchemaMismatch(issues) => {
                format!("DB 스키마가 맞지 않습니다: {}", issues.join(", "))
            }
            ConnectError::RateLimited => {
                "Notion 요청 한도를 초과했습니다. 잠시 후 다시 시도해 주세요".to_string()
            }
            ConnectError::Network(detail) => match detail {
                Some(d) => format!("네트워크 오류가 발생했습니다 ({d})"),
                None => "네트워크 오류가 발생했습니다".to_string(),
            },
            ConnectError::UnexpectedShape(detail) => {
                format!("Notion 응답이 예상과 다릅니다 ({detail})")
            }
        }
    }
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

/// 연결 상태 — U3에서 웹뷰로 직렬화된다 (timer의 `Snapshot` 전례).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConnectionState {
    NotConfigured { missing: Missing },
    Connected { title: String },
    Failed { message: String },
}

/// 미설정 상태에서 무엇이 빠졌는지.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Missing {
    Token,
    Database,
    Both,
}

/// URL 또는 맨 ID 문자열에서 32자 hex를 추출해 UUID(8-4-4-4-12)로 정규화한다.
/// URL 경로의 마지막 32-hex 세그먼트를 취하고, 쿼리스트링(`?v=` 뷰 ID)은 무시한다.
pub fn parse_database_id(input: &str) -> Result<String, ConnectError> {
    let without_query = input.trim().split(['?', '#']).next().unwrap_or_default();
    // 경로 세그먼트를 뒤에서부터 훑으며 32자 hex를 찾는다
    for segment in without_query.rsplit('/') {
        if let Some(hex) = trailing_hex32(segment) {
            return Ok(hyphenate(&hex));
        }
    }
    Err(ConnectError::InvalidId)
}

/// 세그먼트의 뒤쪽 하이픈 조각들을 이어붙여 정확히 32자 hex가 되면 소문자로 돌려준다.
/// "제목-<32hex>", 하이픈 있는 UUID, 맨 32hex 모두 이 규칙 하나로 처리된다.
fn trailing_hex32(segment: &str) -> Option<String> {
    let mut joined = String::new();
    for part in segment.rsplit('-') {
        joined.insert_str(0, part);
        if joined.len() == 32 && joined.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(joined.to_ascii_lowercase());
        }
        if joined.len() >= 32 {
            return None;
        }
    }
    None
}

/// 32자 hex를 UUID 형태(8-4-4-4-12)로 하이픈 삽입.
fn hyphenate(hex: &str) -> String {
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// data source의 `properties` 맵에서 `날짜`(date)·`수행도`(select)를 판정한다.
/// 누락·타입 불일치를 모두 열거해 SchemaMismatch로 반환한다.
pub fn validate_schema(properties: &Value) -> Result<(), ConnectError> {
    let mut issues = Vec::new();
    for (name, expected) in [("날짜", "date"), ("수행도", "select")] {
        match properties
            .get(name)
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str())
        {
            None => issues.push(format!("{name}({expected}) 누락")),
            Some(actual) if actual != expected => {
                issues.push(format!("{name}({expected}) 타입 불일치 — 현재 {actual}"))
            }
            Some(_) => {}
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(ConnectError::SchemaMismatch(issues))
    }
}

/// Notion 에러 응답 body의 `code`(안정 enum) 기준 매핑. `message` 문자열 매칭 금지.
pub fn error_from_code(status: u16, code: &str) -> ConnectError {
    match code {
        "unauthorized" => ConnectError::Unauthorized,
        "object_not_found" => ConnectError::NotFound,
        "validation_error" => ConnectError::InvalidId,
        "rate_limited" => ConnectError::RateLimited,
        // 미지의 코드는 status 기준으로 폴백한다
        _ => match status {
            401 => ConnectError::Unauthorized,
            404 => ConnectError::NotFound,
            429 => ConnectError::RateLimited,
            400 => ConnectError::InvalidId,
            _ => ConnectError::Network(Some(format!("HTTP {status}"))),
        },
    }
}

// ---------------------------------------------------------------------------
// HTTP 클라이언트 (U2) — base URL 주입형, database → data source 2단계 조회
// ---------------------------------------------------------------------------

/// 실제 Notion API 기본 주소. 클라이언트는 항상 주입값을 쓰고,
/// 브릿지(U3)가 이 상수를 넘긴다. 테스트는 wiremock 주소를 넘긴다.
pub const NOTION_API_BASE: &str = "https://api.notion.com";

/// Notion API 버전 헤더 값 (data source 2단계 조회를 지원하는 버전).
pub const NOTION_VERSION: &str = "2025-09-03";

/// 429 재시도 총 시도 상한.
const MAX_ATTEMPTS: u32 = 3;

/// 검증 성공 결과 — DB 제목과 이후 쿼리에 쓸 data source ID.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verified {
    pub title: String,
    pub data_source_id: String,
}

/// base URL 주입형 Notion HTTP 클라이언트.
/// 토큰은 요청 헤더로만 쓰고 어떤 로그·에러 메시지에도 남기지 않는다.
pub struct NotionClient {
    base_url: String,
    http: reqwest::Client,
}

impl NotionClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// database → data source 2단계 조회로 연결을 검증한다.
    /// 1) `GET /v1/databases/{id}` — 제목과 data_sources 추출 (정확히 1개여야 함)
    /// 2) `GET /v1/data_sources/{id}` — properties를 `validate_schema`로 판정
    pub async fn verify_connection(
        &self,
        token: &str,
        database_id: &str,
    ) -> Result<Verified, ConnectError> {
        let db = self
            .get_json(&format!("/v1/databases/{database_id}"), token)
            .await?;

        let data_sources = db
            .get("data_sources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if data_sources.len() != 1 {
            return Err(ConnectError::UnexpectedShape(format!(
                "data source가 {}개입니다",
                data_sources.len()
            )));
        }
        let source = &data_sources[0];
        let data_source_id = source
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| ConnectError::UnexpectedShape("data source ID 없음".to_string()))?;

        // DB 제목: title rich text의 plain_text 이어붙임, 비면 data source name으로 폴백
        let title = rich_text_plain(db.get("title"))
            .filter(|t| !t.is_empty())
            .or_else(|| {
                source
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();

        let data_source = self
            .get_json(&format!("/v1/data_sources/{data_source_id}"), token)
            .await?;
        let properties = data_source
            .get("properties")
            .ok_or_else(|| ConnectError::UnexpectedShape("properties 없음".to_string()))?;
        validate_schema(properties)?;

        Ok(Verified {
            title,
            data_source_id,
        })
    }

    /// 공통 헤더로 GET을 보내고 JSON body를 돌려준다. 이후 마일스톤이 재사용한다.
    /// 429는 `Retry-After`(없으면 지수 백오프)만큼 기다려 최대 `MAX_ATTEMPTS`회 재시도.
    /// 오류 경로 어디에도 토큰 값·URL 전문을 넣지 않는다.
    async fn get_json(&self, path: &str, token: &str) -> Result<Value, ConnectError> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0;
        loop {
            let response = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Notion-Version", NOTION_VERSION)
                .send()
                .await
                .map_err(|_| ConnectError::Network(None))?;

            let status = response.status().as_u16();
            if status == 429 {
                attempt += 1;
                if attempt >= MAX_ATTEMPTS {
                    return Err(ConnectError::RateLimited);
                }
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok());
                tokio::time::sleep(retry_delay(attempt - 1, retry_after)).await;
                continue;
            }
            if !(200..300).contains(&status) {
                // body의 `code`(안정 enum)로 매핑, 파싱 실패 시 status 폴백
                let code = response
                    .json::<Value>()
                    .await
                    .ok()
                    .and_then(|body| {
                        body.get("code")
                            .and_then(|c| c.as_str())
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                return Err(error_from_code(status, &code));
            }
            return response
                .json::<Value>()
                .await
                .map_err(|_| ConnectError::Network(None));
        }
    }
}

/// title rich text 배열의 `plain_text`를 이어붙인다. 배열이 아니면 None.
fn rich_text_plain(value: Option<&Value>) -> Option<String> {
    let items = value?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|item| item.get("plain_text").and_then(|t| t.as_str()))
            .collect::<String>(),
    )
}

/// 재시도 대기 시간 계산 — `Retry-After` 헤더(초)가 있으면 그 값,
/// 없으면 지수 백오프(0.5s → 1s). 테스트는 Retry-After: 0으로 대기 없이 돈다.
fn retry_delay(attempt: u32, retry_after_secs: Option<u64>) -> std::time::Duration {
    match retry_after_secs {
        Some(secs) => std::time::Duration::from_secs(secs),
        None => std::time::Duration::from_millis(500 << attempt),
    }
}

/// 토큰 유무 × DB 유무 × 검증 결과(Ok = DB 제목) → 연결 상태.
/// 둘 다 설정됐지만 아직 검증 전(None)이면 재검증을 유도하는 Failed로 둔다.
pub fn determine_connection_state(
    token_saved: bool,
    db_configured: bool,
    verified: Option<Result<String, ConnectError>>,
) -> ConnectionState {
    match (token_saved, db_configured) {
        (false, false) => ConnectionState::NotConfigured {
            missing: Missing::Both,
        },
        (true, false) => ConnectionState::NotConfigured {
            missing: Missing::Database,
        },
        (false, true) => ConnectionState::NotConfigured {
            missing: Missing::Token,
        },
        (true, true) => match verified {
            Some(Ok(title)) => ConnectionState::Connected { title },
            Some(Err(err)) => ConnectionState::Failed {
                message: err.message(),
            },
            None => ConnectionState::Failed {
                message: "연결을 아직 확인하지 못했습니다. 다시 시도해 주세요".to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 픽스처 ID — 전부 가짜 값 (실제 워크스페이스 ID·토큰 아님)
    const 가짜_ID_하이픈없음: &str = "25b8f0d4c2f98080abcd1234567890ab";
    const 가짜_ID_UUID: &str = "25b8f0d4-c2f9-8080-abcd-1234567890ab";
    const 가짜_뷰_ID: &str = "aaaabbbbccccddddeeeeffff00001111";

    fn 정상_스키마() -> Value {
        json!({
            "이름": { "id": "title", "type": "title", "title": {} },
            "날짜": { "id": "a%3Abc", "type": "date", "date": {} },
            "수행도": { "id": "d%3Aef", "type": "select", "select": { "options": [] } }
        })
    }

    #[test]
    fn url에서_하이픈_없는_id를_추출해_정규화한다() {
        // 일반 URL (경로 세그먼트가 "제목-ID" 형태)
        let url = format!("https://www.notion.so/myworkspace/Plan-{가짜_ID_하이픈없음}");
        assert_eq!(parse_database_id(&url).unwrap(), 가짜_ID_UUID);

        // `?v=` 뷰 ID가 붙은 URL — 쿼리는 무시하고 경로의 ID를 취한다
        let url =
            format!("https://www.notion.so/myworkspace/Plan-{가짜_ID_하이픈없음}?v={가짜_뷰_ID}");
        assert_eq!(parse_database_id(&url).unwrap(), 가짜_ID_UUID);

        // 하이픈 있는 UUID 입력
        assert_eq!(parse_database_id(가짜_ID_UUID).unwrap(), 가짜_ID_UUID);

        // 맨 ID 문자열
        assert_eq!(parse_database_id(가짜_ID_하이픈없음).unwrap(), 가짜_ID_UUID);

        // 앞뒤 공백은 무시한다
        let padded = format!("  {가짜_ID_하이픈없음}  ");
        assert_eq!(parse_database_id(&padded).unwrap(), 가짜_ID_UUID);
    }

    #[test]
    fn 뷰_id나_잘못된_길이의_입력은_거부한다() {
        // 경로에 ID가 없고 쿼리에만 뷰 ID가 있는 URL
        let url = format!("https://www.notion.so/myworkspace?v={가짜_뷰_ID}");
        assert_eq!(parse_database_id(&url), Err(ConnectError::InvalidId));

        // 31자 / 33자 hex — 길이가 다르면 거부
        let short = &가짜_ID_하이픈없음[..31];
        assert_eq!(parse_database_id(short), Err(ConnectError::InvalidId));
        let long = format!("{가짜_ID_하이픈없음}f");
        assert_eq!(parse_database_id(&long), Err(ConnectError::InvalidId));

        // hex가 아예 없는 입력
        assert_eq!(parse_database_id("hello"), Err(ConnectError::InvalidId));
        assert_eq!(parse_database_id(""), Err(ConnectError::InvalidId));
    }

    #[test]
    fn 필수_속성이_모두_있으면_스키마_검증을_통과한다() {
        assert_eq!(validate_schema(&정상_스키마()), Ok(()));
    }

    #[test]
    fn 수행도가_없거나_타입이_다르면_누락_속성을_명시해_실패한다() {
        // `날짜` 누락
        let mut props = 정상_스키마();
        props.as_object_mut().unwrap().remove("날짜");
        let err = validate_schema(&props).unwrap_err();
        match &err {
            ConnectError::SchemaMismatch(issues) => {
                assert_eq!(issues.len(), 1);
                assert!(issues[0].contains("날짜(date)"), "issues = {issues:?}");
            }
            other => panic!("SchemaMismatch가 아님: {other:?}"),
        }

        // `수행도`가 select가 아님 (status 타입)
        let mut props = 정상_스키마();
        props["수행도"] = json!({ "type": "status", "status": {} });
        let err = validate_schema(&props).unwrap_err();
        match &err {
            ConnectError::SchemaMismatch(issues) => {
                assert_eq!(issues.len(), 1);
                assert!(issues[0].contains("수행도(select)"), "issues = {issues:?}");
            }
            other => panic!("SchemaMismatch가 아님: {other:?}"),
        }

        // 둘 다 누락 — 전부 열거한다
        let props = json!({ "이름": { "type": "title", "title": {} } });
        let err = validate_schema(&props).unwrap_err();
        match &err {
            ConnectError::SchemaMismatch(issues) => {
                assert_eq!(issues.len(), 2);
                assert!(issues.iter().any(|i| i.contains("날짜(date)")));
                assert!(issues.iter().any(|i| i.contains("수행도(select)")));
            }
            other => panic!("SchemaMismatch가 아님: {other:?}"),
        }
    }

    #[test]
    fn 에러_코드별로_한국어_메시지가_매핑된다() {
        let e = error_from_code(401, "unauthorized");
        assert_eq!(e, ConnectError::Unauthorized);
        assert!(e.message().contains("토큰"), "message = {}", e.message());

        let e = error_from_code(404, "object_not_found");
        assert_eq!(e, ConnectError::NotFound);
        assert!(
            e.message().contains("DB를 찾을 수 없습니다"),
            "message = {}",
            e.message()
        );
        assert!(
            e.message().contains("Integration"),
            "message = {}",
            e.message()
        );

        let e = error_from_code(400, "validation_error");
        assert_eq!(e, ConnectError::InvalidId);

        let e = error_from_code(429, "rate_limited");
        assert_eq!(e, ConnectError::RateLimited);
        assert!(e.message().contains("한도"), "message = {}", e.message());

        // 미지의 코드는 status 기준으로 폴백한다
        assert_eq!(
            error_from_code(401, "unknown_code"),
            ConnectError::Unauthorized
        );
        assert_eq!(error_from_code(404, "unknown_code"), ConnectError::NotFound);
        assert_eq!(
            error_from_code(429, "unknown_code"),
            ConnectError::RateLimited
        );
        assert!(matches!(
            error_from_code(500, "internal_server_error"),
            ConnectError::Network(_)
        ));
    }

    #[test]
    fn 입력_형식_오류는_전용_한국어_메시지로_매핑되고_원문을_포함하지_않는다() {
        // 토큰을 DB 필드에 잘못 붙여넣은 상황 — 가짜 토큰 형태 문자열
        let 잘못된_입력 = "secret_FAKE_NOT_A_REAL_TOKEN_1234";
        let err = parse_database_id(잘못된_입력).unwrap_err();
        assert_eq!(err, ConnectError::InvalidId);

        let msg = err.message();
        assert!(
            !msg.contains(잘못된_입력),
            "메시지에 입력 원문이 포함됨: {msg}"
        );
        assert!(
            !msg.contains("FAKE_NOT_A_REAL_TOKEN"),
            "메시지에 입력 일부가 포함됨: {msg}"
        );
        // 형식 안내만 담는다
        assert!(msg.contains("32자"), "message = {msg}");
        assert!(msg.contains("붙여넣"), "message = {msg}");
    }

    #[test]
    fn 상태_조합_판정이_올바르다() {
        // 둘 다 없음
        assert_eq!(
            determine_connection_state(false, false, None),
            ConnectionState::NotConfigured {
                missing: Missing::Both
            }
        );
        // 토큰만 있음 → DB가 빠짐
        assert_eq!(
            determine_connection_state(true, false, None),
            ConnectionState::NotConfigured {
                missing: Missing::Database
            }
        );
        // DB만 있음 → 토큰이 빠짐
        assert_eq!(
            determine_connection_state(false, true, None),
            ConnectionState::NotConfigured {
                missing: Missing::Token
            }
        );
        // 둘 다 있고 검증 성공 → 연결됨 (DB 제목 포함)
        assert_eq!(
            determine_connection_state(true, true, Some(Ok("계획표".to_string()))),
            ConnectionState::Connected {
                title: "계획표".to_string()
            }
        );
        // 둘 다 있고 검증 실패 → 실패 메시지
        let state = determine_connection_state(true, true, Some(Err(ConnectError::Unauthorized)));
        match &state {
            ConnectionState::Failed { message } => {
                assert!(message.contains("토큰"), "message = {message}");
            }
            other => panic!("Failed가 아님: {other:?}"),
        }
    }

    #[test]
    fn 연결_상태는_snake_case_태그로_직렬화된다() {
        // U3 웹뷰 직렬화 계약 (timer Snapshot 전례)
        let v = serde_json::to_value(ConnectionState::Connected {
            title: "계획표".to_string(),
        })
        .unwrap();
        assert_eq!(v["state"], "connected");
        assert_eq!(v["title"], "계획표");

        let v = serde_json::to_value(ConnectionState::NotConfigured {
            missing: Missing::Both,
        })
        .unwrap();
        assert_eq!(v["state"], "not_configured");
        assert_eq!(v["missing"], "both");
    }
}

#[cfg(test)]
mod http_tests {
    // 한국어 테스트 이름에 포함된 DB·Retry_After 같은 대문자 약어 허용
    #![allow(non_snake_case)]

    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // 픽스처 — 전부 가짜 값 (실제 토큰·워크스페이스 ID 아님)
    const 가짜_토큰: &str = "secret_FAKE_TEST_TOKEN_0000";
    const 가짜_DB_ID: &str = "25b8f0d4-c2f9-8080-abcd-1234567890ab";
    const 가짜_DS_ID: &str = "ffffeeee-dddd-cccc-bbbb-aaaa00001111";
    const 가짜_DS_ID_2: &str = "11110000-aaaa-bbbb-cccc-ddddeeeeffff";

    fn database_응답() -> serde_json::Value {
        json!({
            "object": "database",
            "id": 가짜_DB_ID,
            "title": [
                { "type": "text", "plain_text": "계획", "text": { "content": "계획" } },
                { "type": "text", "plain_text": "표", "text": { "content": "표" } }
            ],
            "data_sources": [ { "id": 가짜_DS_ID, "name": "계획표" } ]
        })
    }

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

    fn 에러_body(status: u16, code: &str) -> serde_json::Value {
        json!({ "object": "error", "status": status, "code": code, "message": "fake" })
    }

    fn db_경로() -> String {
        format!("/v1/databases/{가짜_DB_ID}")
    }

    fn ds_경로() -> String {
        format!("/v1/data_sources/{가짜_DS_ID}")
    }

    async fn mount_정상_database(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(database_응답()))
            .mount(server)
            .await;
    }

    async fn mount_정상_data_source(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path(ds_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn 이단계_조회로_스키마_검증에_성공한다() {
        let server = MockServer::start().await;
        let auth = format!("Bearer {가짜_토큰}");
        // 두 요청 모두 Authorization·Notion-Version 헤더를 요구한다
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .and(header("Authorization", auth.as_str()))
            .and(header("Notion-Version", NOTION_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(database_응답()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(ds_경로()))
            .and(header("Authorization", auth.as_str()))
            .and(header("Notion-Version", NOTION_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(data_source_응답()))
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let verified = client.verify_connection(가짜_토큰, 가짜_DB_ID).await.unwrap();
        assert_eq!(verified.title, "계획표"); // rich text 조각 이어붙임
        assert_eq!(verified.data_source_id, 가짜_DS_ID);
    }

    #[tokio::test]
    async fn 미공유_DB는_404를_연결_힌트가_담긴_실패로_매핑한다() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::NotFound);
        assert!(
            err.message().contains("Integration"),
            "message = {}",
            err.message()
        );
    }

    #[tokio::test]
    async fn 무효_토큰은_401을_토큰_실패로_매핑한다() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(ResponseTemplate::new(401).set_body_json(에러_body(401, "unauthorized")))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Unauthorized);
    }

    #[tokio::test]
    async fn 스키마가_다른_DB는_누락_속성을_담아_실패한다() {
        let server = MockServer::start().await;
        mount_정상_database(&server).await;
        // 수행도가 select가 아니라 status 타입인 data source
        let mut ds = data_source_응답();
        ds["properties"]["수행도"] = json!({ "id": "d%3Aef", "type": "status", "status": {} });
        Mock::given(method("GET"))
            .and(path(ds_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(ds))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        match &err {
            ConnectError::SchemaMismatch(issues) => {
                assert!(
                    issues.iter().any(|i| i.contains("수행도(select)")),
                    "issues = {issues:?}"
                );
            }
            other => panic!("SchemaMismatch가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn data_source가_하나가_아니면_오류로_처리한다() {
        // 0개
        let server = MockServer::start().await;
        let mut db = database_응답();
        db["data_sources"] = json!([]);
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(db))
            .mount(&server)
            .await;
        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        match &err {
            ConnectError::UnexpectedShape(detail) => {
                assert!(detail.contains("0개"), "detail = {detail}")
            }
            other => panic!("UnexpectedShape가 아님: {other:?}"),
        }

        // 2개
        let server = MockServer::start().await;
        let mut db = database_응답();
        db["data_sources"] = json!([
            { "id": 가짜_DS_ID, "name": "계획표" },
            { "id": 가짜_DS_ID_2, "name": "계획표2" }
        ]);
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(db))
            .mount(&server)
            .await;
        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        match &err {
            ConnectError::UnexpectedShape(detail) => {
                assert!(detail.contains("2개"), "detail = {detail}")
            }
            other => panic!("UnexpectedShape가 아님: {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_429는_Retry_After를_기다렸다가_재시도해_성공한다() {
        let server = MockServer::start().await;
        // 첫 요청만 429 (Retry-After: 0 → 테스트가 실제로 기다리지 않는다)
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0")
                    .set_body_json(에러_body(429, "rate_limited")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(database_응답()))
            .expect(1)
            .mount(&server)
            .await;
        mount_정상_data_source(&server).await;

        let client = NotionClient::new(server.uri());
        let verified = client.verify_connection(가짜_토큰, 가짜_DB_ID).await.unwrap();
        assert_eq!(verified.data_source_id, 가짜_DS_ID);
    }

    #[tokio::test]
    async fn 재시도_상한을_넘으면_실패한다() {
        let server = MockServer::start().await;
        // 항상 429 — 상한(3회)만큼 시도한 뒤 RateLimited로 끝나야 한다
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0")
                    .set_body_json(에러_body(429, "rate_limited")),
            )
            .expect(3)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::RateLimited);
    }

    #[tokio::test]
    async fn 네트워크_오류는_네트워크_실패로_매핑된다() {
        // 서버가 없는 포트 — 연결 거부는 상세 없는 Network 오류가 된다
        let client = NotionClient::new("http://127.0.0.1:1");
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Network(None));
    }

    #[tokio::test]
    async fn 페이지_ID를_붙여넣으면_404_실패에_원본_링크_확인_안내가_포함된다() {
        // 페이지 ID로 databases 조회 → Notion은 object_not_found를 돌려준다
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(db_경로()))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .verify_connection(가짜_토큰, 가짜_DB_ID)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::NotFound);
        assert!(
            err.message().contains("원본"),
            "message = {}",
            err.message()
        );
    }
}
