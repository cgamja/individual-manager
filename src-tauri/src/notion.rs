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
