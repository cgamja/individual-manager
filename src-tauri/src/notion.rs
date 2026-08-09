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
    /// 409 — 다른 곳에서 같은 항목을 먼저 수정함 (쓰기 충돌)
    Conflict,
    /// rich_text 한 조각의 2000자 상한 초과 — HTTP 호출 전에 거부한다
    TooLong,
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
            ConnectError::Conflict => {
                "다른 곳에서 같은 항목이 수정됐습니다. 새로고침 후 다시 시도해 주세요".to_string()
            }
            ConnectError::TooLong => {
                format!("할 일 텍스트가 너무 깁니다 ({MAX_RICH_TEXT_CHARS}자 제한)")
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
        "conflict_error" => ConnectError::Conflict,
        // 미지의 코드는 status 기준으로 폴백한다
        _ => match status {
            401 => ConnectError::Unauthorized,
            404 => ConnectError::NotFound,
            409 => ConnectError::Conflict,
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

/// 요청당 타임아웃(초) — 응답이 멈춰도 커맨드·UI 잠금이 이 상한을 넘지 않는다.
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// `Retry-After` 헤더 값의 상한(초) — 서버 값 하나로 장시간 잠기지 않게 클램프한다.
const MAX_RETRY_AFTER_SECS: u64 = 30;

/// rich_text `text.content` 한 조각의 문자 수 상한 (Notion API 요청 제한).
const MAX_RICH_TEXT_CHARS: usize = 2000;

/// 검증 성공 결과 — DB 제목과 이후 쿼리에 쓸 data source ID.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verified {
    pub title: String,
    pub data_source_id: String,
}

/// 페이지 본문의 to_do 블록 하나 — U4에서 웹뷰로 직렬화된다 (`ConnectionState` 전례).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub checked: bool,
}

/// 새 페이지 생성에 실을 페이지 아이콘 — 최신 `[TODO]` 행에서 복사한다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PageIcon {
    /// 유니코드 이모지 아이콘 — 그대로 복사 가능
    Emoji(String),
    /// 외부 URL 아이콘 — URL로 복사 가능
    External(String),
}

/// 복사할 수 없는 아이콘(file·custom_emoji)의 폴백 이모지.
pub const DEFAULT_PAGE_ICON: &str = "📝";

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
            // 타임아웃 없는 기본 클라이언트는 응답이 멈추면 커맨드가 영원히 pending되고
            // 프론트 isVerifying 잠금이 풀리지 않는다 — 요청당 상한을 강제한다.
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .build()
                .expect("reqwest 클라이언트 생성 실패"),
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

    /// 주어진 날짜(로컬 `YYYY-MM-DD`, 브릿지가 주입)의 행을 data source에서 찾는다.
    /// date-only `equals` 필터만 쓴다 — datetime을 섞으면 타임존 드리프트가 생긴다(KTD3).
    /// 반환: `Some((page_id, 제목))` 또는 행 없음 `None`. 복수 행이면 `[TODO]` 제목 우선.
    pub async fn find_page_by_date(
        &self,
        token: &str,
        data_source_id: &str,
        date: &str,
    ) -> Result<Option<(String, String)>, ConnectError> {
        let body = serde_json::json!({
            "filter": { "property": "날짜", "date": { "equals": date } },
            "page_size": 5
        });
        let response = self
            .request_json(
                reqwest::Method::POST,
                &format!("/v1/data_sources/{data_source_id}/query"),
                token,
                Some(&body),
            )
            .await?;
        let results = response
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ConnectError::UnexpectedShape("results가 배열이 아님".to_string()))?;
        Ok(pick_day_page(results))
    }

    /// 가장 최근 `[TODO]` 행의 페이지 아이콘을 읽는다 — 새 페이지 생성 시 복사용.
    /// 부가 기능이므로 HTTP·파싱 오류는 전파하지 않고 None으로 수렴한다.
    pub async fn latest_todo_icon(&self, token: &str, data_source_id: &str) -> Option<PageIcon> {
        // 제목 필터의 property는 표시 이름이 아니라 title 속성의 고정 id "title"을 쓴다
        // (title 키를 하드코딩하지 않는다는 모듈 규칙과 같은 이유).
        let body = serde_json::json!({
            "filter": { "property": "title", "title": { "equals": "[TODO]" } },
            "sorts": [ { "property": "날짜", "direction": "descending" } ],
            "page_size": 1
        });
        let response = self
            .request_json(
                reqwest::Method::POST,
                &format!("/v1/data_sources/{data_source_id}/query"),
                token,
                Some(&body),
            )
            .await
            .ok()?;
        let row = response.get("results")?.as_array()?.first()?;
        page_icon_from_json(row.get("icon"))
    }

    /// 페이지 본문의 최상위 to_do 블록을 페이지 순서대로 모두 수집한다(KTD6).
    /// `has_more`/`next_cursor` 페이지네이션 루프(100개/페이지)로 끝까지 돈다.
    pub async fn fetch_todos(
        &self,
        token: &str,
        page_id: &str,
    ) -> Result<Vec<TodoItem>, ConnectError> {
        let mut items = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let path = match &cursor {
                Some(c) => format!("/v1/blocks/{page_id}/children?page_size=100&start_cursor={c}"),
                None => format!("/v1/blocks/{page_id}/children?page_size=100"),
            };
            let response = self.get_json(&path, token).await?;
            let results = response
                .get("results")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    ConnectError::UnexpectedShape("results가 배열이 아님".to_string())
                })?;
            items.extend(results.iter().filter_map(todo_from_block));
            if !response
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return Ok(items);
            }
            cursor = response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if cursor.is_none() {
                return Err(ConnectError::UnexpectedShape(
                    "has_more인데 next_cursor 없음".to_string(),
                ));
            }
        }
    }

    /// 페이지 본문 끝에 미체크 to_do 블록 하나를 추가한다(KTD6 — 위치 지정 없음).
    pub async fn append_todo(
        &self,
        token: &str,
        page_id: &str,
        text: &str,
    ) -> Result<(), ConnectError> {
        let body = serde_json::json!({
            "children": [ {
                "object": "block",
                "type": "to_do",
                "to_do": { "rich_text": plain_rich_text(text)?, "checked": false }
            } ]
        });
        self.request_json(
            reqwest::Method::PATCH,
            &format!("/v1/blocks/{page_id}/children"),
            token,
            Some(&body),
        )
        .await?;
        Ok(())
    }

    /// to_do 블록의 체크 상태만 바꾼다 — `checked`만 보내면 rich_text는 유지된다.
    pub async fn set_todo_checked(
        &self,
        token: &str,
        block_id: &str,
        checked: bool,
    ) -> Result<(), ConnectError> {
        let body = serde_json::json!({ "to_do": { "checked": checked } });
        self.request_json(
            reqwest::Method::PATCH,
            &format!("/v1/blocks/{block_id}"),
            token,
            Some(&body),
        )
        .await?;
        Ok(())
    }

    /// to_do 블록의 텍스트를 전체 교체한다 — `rich_text`만 보내면 checked는 유지된다.
    /// 서식 있는 기존 rich_text는 plain text 1조각으로 대체된다(단순 편집 정책).
    pub async fn set_todo_text(
        &self,
        token: &str,
        block_id: &str,
        text: &str,
    ) -> Result<(), ConnectError> {
        let body = serde_json::json!({ "to_do": { "rich_text": plain_rich_text(text)? } });
        self.request_json(
            reqwest::Method::PATCH,
            &format!("/v1/blocks/{block_id}"),
            token,
            Some(&body),
        )
        .await?;
        Ok(())
    }

    /// 오늘 행이 없을 때 하루 골격 페이지를 만든다 — 제목 `[TODO]`,
    /// `날짜` date-only, 본문에 heading_3 `공부`/`기타` 두 섹션.
    /// title 속성 키는 DB마다 다르므로 스키마를 먼저 조회해 알아낸다.
    /// `icon`이 Some이면 페이지 아이콘으로 싣고, None이면 icon 키를 생략한다.
    /// 반환: 생성된 페이지 ID.
    pub async fn create_day_page(
        &self,
        token: &str,
        data_source_id: &str,
        date: &str,
        icon: Option<&PageIcon>,
    ) -> Result<String, ConnectError> {

        let data_source = self
            .get_json(&format!("/v1/data_sources/{data_source_id}"), token)
            .await?;
        let properties = data_source
            .get("properties")
            .ok_or_else(|| ConnectError::UnexpectedShape("properties 없음".to_string()))?;
        let title_key = title_property_key(properties)
            .ok_or_else(|| ConnectError::UnexpectedShape("title 속성 없음".to_string()))?;

        let mut body = serde_json::json!({
            "parent": { "type": "data_source_id", "data_source_id": data_source_id },
            "properties": {
                title_key: { "title": plain_rich_text("[TODO]")? },
                "날짜": { "date": { "start": date } }
            },
            "children": [ heading_3_block("공부"), heading_3_block("기타") ]
        });
        // 아이콘은 선택 — None이면 icon 키 자체를 넣지 않는다
        if let Some(icon) = icon {
            body["icon"] = page_icon_json(icon);
        }
        let page = self
            .request_json(reqwest::Method::POST, "/v1/pages", token, Some(&body))
            .await?;
        page.get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| ConnectError::UnexpectedShape("생성된 페이지 ID 없음".to_string()))
    }

    /// 공통 헤더로 GET을 보내고 JSON body를 돌려준다.
    async fn get_json(&self, path: &str, token: &str) -> Result<Value, ConnectError> {
        self.request_json(reqwest::Method::GET, path, token, None)
            .await
    }

    /// 공통 요청 경로 — GET/POST/PATCH를 하나의 재시도·오류 매핑으로 처리한다.
    /// 429는 `Retry-After`(없으면 지수 백오프)만큼 기다려 최대 `MAX_ATTEMPTS`회 재시도.
    /// 오류 경로 어디에도 토큰 값·URL 전문을 넣지 않는다.
    async fn request_json(
        &self,
        method: reqwest::Method,
        path: &str,
        token: &str,
        body: Option<&Value>,
    ) -> Result<Value, ConnectError> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0;
        loop {
            let mut request = self
                .http
                .request(method.clone(), &url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Notion-Version", NOTION_VERSION);
            if let Some(json) = body {
                request = request.json(json);
            }
            let response = request
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        ConnectError::Network(Some("응답 시간 초과".to_string()))
                    } else {
                        ConnectError::Network(None)
                    }
                })?;

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

/// plain text 1조각짜리 rich_text 배열을 만든다 (쓰기 요청 공용).
/// Notion의 조각당 2000자 상한을 넘는 입력은 HTTP 호출 전에 거부한다.
fn plain_rich_text(text: &str) -> Result<Value, ConnectError> {
    if text.chars().count() > MAX_RICH_TEXT_CHARS {
        return Err(ConnectError::TooLong);
    }
    Ok(serde_json::json!([
        { "type": "text", "text": { "content": text } }
    ]))
}

/// data source `properties` 맵에서 `type == "title"`인 속성의 키 이름을 찾는다.
/// 키 이름은 DB마다 다르므로 하드코딩하지 않는다 (`page_title`과 같은 규칙).
fn title_property_key(properties: &Value) -> Option<String> {
    properties.as_object()?.iter().find_map(|(key, prop)| {
        (prop.get("type").and_then(|t| t.as_str()) == Some("title")).then(|| key.clone())
    })
}

/// heading_3 블록 하나 — 하루 골격 페이지의 섹션 헤딩용.
fn heading_3_block(text: &str) -> Value {
    serde_json::json!({
        "object": "block",
        "type": "heading_3",
        "heading_3": { "rich_text": [ { "type": "text", "text": { "content": text } } ] }
    })
}

/// query 결과에서 하루의 행 하나를 고른다 — 같은 날짜에 휴일·MT 행이 섞일 수 있으므로
/// 제목이 정확히 `[TODO]`인 행을 우선하고, 없으면 첫 행을 쓴다. 반환: (page_id, 제목).
fn pick_day_page(results: &[Value]) -> Option<(String, String)> {
    let candidates: Vec<(String, String)> = results
        .iter()
        .filter_map(|page| {
            let id = page.get("id")?.as_str()?.to_string();
            Some((id, page_title(page)))
        })
        .collect();
    candidates
        .iter()
        .find(|(_, title)| title == "[TODO]")
        .or_else(|| candidates.first())
        .cloned()
}

/// 페이지 `properties`에서 title 타입 속성을 찾아 plain_text를 이어붙인다.
/// 키 이름은 하드코딩하지 않는다 — DB마다 다를 수 있다(사용자 DB는 `이름`).
fn page_title(page: &Value) -> String {
    page.get("properties")
        .and_then(|props| props.as_object())
        .and_then(|props| {
            props
                .values()
                .find(|p| p.get("type").and_then(|t| t.as_str()) == Some("title"))
        })
        .and_then(|p| rich_text_plain(p.get("title")))
        .unwrap_or_default()
}

/// 페이지 JSON의 `icon` 값 → PageIcon 매핑 (순수 함수).
/// emoji는 그대로, external은 URL로 복사한다. file은 URL이 1시간 만료라 복사 불가,
/// custom_emoji는 복사 범위 밖이므로 둘 다 기본 이모지로 폴백한다.
/// icon이 null·없음이거나 형태를 모르면 None.
fn page_icon_from_json(icon: Option<&Value>) -> Option<PageIcon> {
    let icon = icon?;
    match icon.get("type").and_then(|t| t.as_str())? {
        "emoji" => icon
            .get("emoji")
            .and_then(|e| e.as_str())
            .map(|e| PageIcon::Emoji(e.to_string())),
        "external" => icon
            .get("external")
            .and_then(|x| x.get("url"))
            .and_then(|u| u.as_str())
            .map(|u| PageIcon::External(u.to_string())),
        "file" | "custom_emoji" => Some(PageIcon::Emoji(DEFAULT_PAGE_ICON.to_string())),
        // 미지의 아이콘 타입은 복사하지 않는다 (부가 기능 — 조용히 생략)
        _ => None,
    }
}

/// PageIcon → 페이지 생성 body의 최상위 `icon` JSON.
fn page_icon_json(icon: &PageIcon) -> Value {
    match icon {
        PageIcon::Emoji(emoji) => serde_json::json!({ "type": "emoji", "emoji": emoji }),
        PageIcon::External(url) => {
            serde_json::json!({ "type": "external", "external": { "url": url } })
        }
    }
}

/// 블록 JSON → TodoItem 변환. `type == "to_do"`이고 archived가 아닌 블록만 변환한다.
/// 오류 대신 None — 형태가 어긋난 블록은 목록에서 조용히 제외된다.
fn todo_from_block(block: &Value) -> Option<TodoItem> {
    if block.get("type").and_then(|t| t.as_str()) != Some("to_do") {
        return None;
    }
    if block.get("archived").and_then(|a| a.as_bool()) == Some(true) {
        return None;
    }
    let id = block.get("id")?.as_str()?.to_string();
    let to_do = block.get("to_do")?;
    let text = rich_text_plain(to_do.get("rich_text")).unwrap_or_default();
    let checked = to_do
        .get("checked")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);
    Some(TodoItem { id, text, checked })
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

/// 재시도 대기 시간 계산 — `Retry-After` 헤더(초)가 있으면 그 값(상한 30초 클램프 —
/// 서버 값 하나로 커맨드·UI 잠금이 장시간 붙잡히지 않게), 없으면 지수 백오프(0.5s → 1s).
/// 테스트는 Retry-After: 0으로 대기 없이 돈다.
fn retry_delay(attempt: u32, retry_after_secs: Option<u64>) -> std::time::Duration {
    match retry_after_secs {
        Some(secs) => std::time::Duration::from_secs(secs.min(MAX_RETRY_AFTER_SECS)),
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
    fn 재시도_대기는_retry_after_상한을_클램프한다() {
        // 서버가 보낸 큰 값은 상한(30초)으로 클램프, 작은 값은 그대로
        assert_eq!(
            retry_delay(0, Some(3600)),
            std::time::Duration::from_secs(30)
        );
        assert_eq!(retry_delay(0, Some(5)), std::time::Duration::from_secs(5));
        // 헤더 없으면 지수 백오프
        assert_eq!(
            retry_delay(1, None),
            std::time::Duration::from_millis(1000)
        );
    }

    fn 가짜_페이지_행(id: &str, 제목: &str) -> Value {
        json!({
            "object": "page",
            "id": id,
            "properties": {
                "날짜": { "id": "a%3Abc", "type": "date",
                          "date": { "start": "2026-08-09", "end": null } },
                "이름": { "id": "title", "type": "title",
                          "title": [ { "type": "text", "plain_text": 제목 } ] }
            }
        })
    }

    #[test]
    fn rich_text_조각들이_plain_text로_연결된다() {
        let block = json!({
            "object": "block",
            "id": "block-1",
            "type": "to_do",
            "has_children": false,
            "archived": false,
            "to_do": {
                "rich_text": [
                    { "type": "text", "plain_text": "10:00 알고리즘 " },
                    { "type": "text", "plain_text": "1문제" }
                ],
                "checked": true
            }
        });
        assert_eq!(
            todo_from_block(&block),
            Some(TodoItem {
                id: "block-1".to_string(),
                text: "10:00 알고리즘 1문제".to_string(),
                checked: true,
            })
        );
    }

    #[test]
    fn to_do가_아닌_블록과_archived_블록은_변환되지_않는다() {
        let heading = json!({
            "object": "block", "id": "block-h", "type": "heading_3",
            "has_children": false, "archived": false,
            "heading_3": { "rich_text": [ { "type": "text", "plain_text": "공부" } ] }
        });
        assert_eq!(todo_from_block(&heading), None);

        let archived = json!({
            "object": "block", "id": "block-a", "type": "to_do",
            "has_children": false, "archived": true,
            "to_do": { "rich_text": [ { "type": "text", "plain_text": "지운 항목" } ],
                       "checked": false }
        });
        assert_eq!(todo_from_block(&archived), None);
    }

    #[test]
    #[allow(non_snake_case)]
    fn TODO_제목_행이_없으면_첫_행을_선택한다() {
        // [TODO] 행이 없는 날 (휴일만 있는 날) — 첫 행 폴백
        let rows = vec![
            가짜_페이지_행("page-holiday", "휴일"),
            가짜_페이지_행("page-mt", "MT"),
        ];
        assert_eq!(
            pick_day_page(&rows),
            Some(("page-holiday".to_string(), "휴일".to_string()))
        );

        // 빈 결과 → None
        assert_eq!(pick_day_page(&[]), None);
    }

    #[test]
    fn 긴_텍스트는_2000자_제한으로_거부된다() {
        // 2000자는 통과, 2001자는 거부 (바이트가 아니라 문자 수 기준)
        let 경계_텍스트 = "가".repeat(2000);
        assert!(plain_rich_text(&경계_텍스트).is_ok());

        let 긴_텍스트 = "가".repeat(2001);
        let err = plain_rich_text(&긴_텍스트).unwrap_err();
        assert_eq!(err, ConnectError::TooLong);

        // 메시지는 제한 안내만 담고 사용자 입력 원문을 포함하지 않는다 (모듈 규칙)
        let msg = err.message();
        assert!(msg.contains("2000자"), "message = {msg}");
        assert!(!msg.contains("가가가"), "메시지에 입력 원문이 포함됨: {msg}");
    }

    #[test]
    fn plain_rich_text는_text_조각_하나를_만든다() {
        let value = plain_rich_text("10:00 알고리즘 1문제").unwrap();
        assert_eq!(
            value,
            json!([ { "type": "text", "text": { "content": "10:00 알고리즘 1문제" } } ])
        );
    }

    #[test]
    fn properties에서_title_타입_속성_키를_찾는다() {
        // 키 이름이 DB마다 다르므로 하드코딩하지 않는다 — type == "title"인 키를 찾는다
        let props = json!({
            "날짜": { "id": "a%3Abc", "type": "date", "date": {} },
            "할일제목": { "id": "title", "type": "title", "title": {} }
        });
        assert_eq!(
            title_property_key(&props),
            Some("할일제목".to_string())
        );

        // title 타입이 없으면 None
        let props = json!({ "날짜": { "type": "date", "date": {} } });
        assert_eq!(title_property_key(&props), None);
    }

    #[test]
    fn 아이콘_JSON_매핑은_emoji와_external만_복사하고_나머지는_폴백한다() {
        // emoji → 그대로 복사
        let icon = json!({ "type": "emoji", "emoji": "🌊" });
        assert_eq!(
            page_icon_from_json(Some(&icon)),
            Some(PageIcon::Emoji("🌊".to_string()))
        );

        // external → URL로 복사
        let icon = json!({ "type": "external",
                           "external": { "url": "https://example.com/icon.png" } });
        assert_eq!(
            page_icon_from_json(Some(&icon)),
            Some(PageIcon::External("https://example.com/icon.png".to_string()))
        );

        // file → 기본 이모지 폴백 (URL 1시간 만료라 복사 불가)
        let icon = json!({ "type": "file",
                           "file": { "url": "https://s3.example.com/x.png",
                                     "expiry_time": "2026-08-10T00:00:00.000Z" } });
        assert_eq!(
            page_icon_from_json(Some(&icon)),
            Some(PageIcon::Emoji(DEFAULT_PAGE_ICON.to_string()))
        );

        // custom_emoji → 기본 이모지 폴백 (복사 범위 밖)
        let icon = json!({ "type": "custom_emoji",
                           "custom_emoji": { "id": "ce-1", "name": "펭귄" } });
        assert_eq!(
            page_icon_from_json(Some(&icon)),
            Some(PageIcon::Emoji(DEFAULT_PAGE_ICON.to_string()))
        );
    }

    #[test]
    fn 아이콘이_null이거나_없으면_None이다() {
        assert_eq!(page_icon_from_json(Some(&Value::Null)), None);
        assert_eq!(page_icon_from_json(None), None);
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
    use wiremock::matchers::{body_json, header, method, path, query_param};
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
    async fn POST_요청도_429면_Retry_After를_기다렸다가_재시도한다() {
        let server = MockServer::start().await;
        let 요청_body = json!({ "parent": { "data_source_id": 가짜_DS_ID } });
        // 첫 요청만 429 (Retry-After: 0 → 실제 대기 없음) — 재시도에도 같은 body가 와야 한다
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(요청_body.clone()))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0")
                    .set_body_json(에러_body(429, "rate_limited")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(요청_body.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "page" })))
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let result = client
            .request_json(reqwest::Method::POST, "/v1/pages", 가짜_토큰, Some(&요청_body))
            .await
            .unwrap();
        assert_eq!(result["object"], "page");
    }

    #[tokio::test]
    async fn PATCH_요청의_에러_code가_ConnectError로_매핑된다() {
        // conflict_error → Conflict (새 변형)
        let server = MockServer::start().await;
        let 페이지_경로 = format!("/v1/pages/{가짜_DS_ID_2}");
        Mock::given(method("PATCH"))
            .and(path(페이지_경로.as_str()))
            .respond_with(ResponseTemplate::new(409).set_body_json(에러_body(409, "conflict_error")))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .request_json(
                reqwest::Method::PATCH,
                &페이지_경로,
                가짜_토큰,
                Some(&json!({ "properties": {} })),
            )
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Conflict);
        assert!(
            err.message().contains("새로고침"),
            "message = {}",
            err.message()
        );

        // 기존 코드 매핑도 PATCH 경로에서 동일하게 동작한다 (unauthorized → Unauthorized)
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(페이지_경로.as_str()))
            .respond_with(ResponseTemplate::new(401).set_body_json(에러_body(401, "unauthorized")))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .request_json(
                reqwest::Method::PATCH,
                &페이지_경로,
                가짜_토큰,
                Some(&json!({ "properties": {} })),
            )
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Unauthorized);
    }

    #[tokio::test]
    async fn 상태_409는_body_code가_없어도_Conflict로_매핑된다() {
        // body에 code 필드가 없음 → status 기준 폴백 (401/404/429 폴백과 동일한 규칙)
        let server = MockServer::start().await;
        let 페이지_경로 = format!("/v1/pages/{가짜_DS_ID_2}");
        Mock::given(method("PATCH"))
            .and(path(페이지_경로.as_str()))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .request_json(
                reqwest::Method::PATCH,
                &페이지_경로,
                가짜_토큰,
                Some(&json!({ "properties": {} })),
            )
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::Conflict);
    }

    // --- U2. 조회 경로 — 오늘 행 쿼리 + to_do 블록 수집 픽스처 ---

    const 가짜_페이지_ID: &str = "77770000-1111-2222-3333-444455556666";
    const 가짜_휴일_페이지_ID: &str = "88880000-1111-2222-3333-444455556666";
    const 가짜_날짜: &str = "2026-08-09";

    fn 쿼리_경로() -> String {
        format!("/v1/data_sources/{가짜_DS_ID}/query")
    }

    fn children_경로() -> String {
        format!("/v1/blocks/{가짜_페이지_ID}/children")
    }

    fn 날짜_쿼리_body() -> serde_json::Value {
        json!({
            "filter": { "property": "날짜", "date": { "equals": 가짜_날짜 } },
            "page_size": 5
        })
    }

    fn 페이지_행(id: &str, 제목: &str) -> serde_json::Value {
        json!({
            "object": "page",
            "id": id,
            "properties": {
                "날짜": { "id": "a%3Abc", "type": "date",
                          "date": { "start": 가짜_날짜, "end": null } },
                "이름": { "id": "title", "type": "title",
                          "title": [ { "type": "text", "plain_text": 제목 } ] }
            }
        })
    }

    fn 쿼리_응답(rows: Vec<serde_json::Value>) -> serde_json::Value {
        json!({ "object": "list", "results": rows, "has_more": false, "next_cursor": null })
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

    fn children_응답(
        blocks: Vec<serde_json::Value>,
        next_cursor: Option<&str>,
    ) -> serde_json::Value {
        json!({
            "object": "list",
            "results": blocks,
            "has_more": next_cursor.is_some(),
            "next_cursor": next_cursor
        })
    }

    #[tokio::test]
    async fn 날짜_필터_쿼리_body가_date_only_equals로_전송된다() {
        let server = MockServer::start().await;
        // 필터는 date-only equals 하나 + page_size — 그 외 필드가 붙으면 매치 실패한다
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(header("Notion-Version", NOTION_VERSION))
            .and(body_json(날짜_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(쿼리_응답(vec![페이지_행(가짜_페이지_ID, "[TODO]")])),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let found = client
            .find_page_by_date(가짜_토큰, 가짜_DS_ID, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            found,
            Some((가짜_페이지_ID.to_string(), "[TODO]".to_string()))
        );
    }

    #[tokio::test]
    async fn 결과가_없으면_None을_돌려준다() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![])))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let found = client
            .find_page_by_date(가짜_토큰, 가짜_DS_ID, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn 결과가_여러_행이면_TODO_제목_행을_우선_선택한다() {
        // 같은 날짜에 휴일 행이 먼저 오는 혼재 픽스처 — [TODO] 행을 골라야 한다
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![
                페이지_행(가짜_휴일_페이지_ID, "휴일"),
                페이지_행(가짜_페이지_ID, "[TODO]"),
            ])))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let found = client
            .find_page_by_date(가짜_토큰, 가짜_DS_ID, 가짜_날짜)
            .await
            .unwrap();
        assert_eq!(
            found,
            Some((가짜_페이지_ID.to_string(), "[TODO]".to_string()))
        );
    }

    #[tokio::test]
    async fn 두_페이지로_나뉜_children을_병합해_순서를_유지한다() {
        let server = MockServer::start().await;
        // 커서 있는 요청 매처를 먼저 등록한다 — 뒤의 일반 매처가 가로채지 않게
        Mock::given(method("GET"))
            .and(path(children_경로()))
            .and(query_param("page_size", "100"))
            .and(query_param("start_cursor", "cursor-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(
                vec![
                    to_do_블록("block-3", "셋째", false),
                    to_do_블록("block-4", "넷째", true),
                ],
                None,
            )))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(children_경로()))
            .and(query_param("page_size", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(
                vec![
                    to_do_블록("block-1", "첫째", true),
                    to_do_블록("block-2", "둘째", false),
                ],
                Some("cursor-1"),
            )))
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let todos = client.fetch_todos(가짜_토큰, 가짜_페이지_ID).await.unwrap();
        assert_eq!(
            todos.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["block-1", "block-2", "block-3", "block-4"]
        );
        assert_eq!(
            todos.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
            vec!["첫째", "둘째", "셋째", "넷째"]
        );
        assert_eq!(
            todos.iter().map(|t| t.checked).collect::<Vec<_>>(),
            vec![true, false, false, true]
        );
    }

    #[tokio::test]
    async fn to_do가_아닌_블록과_archived_블록은_목록에서_제외된다() {
        let server = MockServer::start().await;
        // heading·paragraph·archived to_do 혼재 — to_do 2개만 남아야 한다
        let mut archived = to_do_블록("block-arch", "지운 항목", false);
        archived["archived"] = json!(true);
        Mock::given(method("GET"))
            .and(path(children_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(children_응답(
                vec![
                    json!({
                        "object": "block", "id": "block-h", "type": "heading_3",
                        "has_children": false, "archived": false,
                        "heading_3": { "rich_text": [ { "type": "text", "plain_text": "공부" } ] }
                    }),
                    to_do_블록("block-1", "첫째", false),
                    json!({
                        "object": "block", "id": "block-p", "type": "paragraph",
                        "has_children": false, "archived": false,
                        "paragraph": { "rich_text": [ { "type": "text", "plain_text": "메모" } ] }
                    }),
                    archived,
                    to_do_블록("block-2", "둘째", true),
                ],
                None,
            )))
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let todos = client.fetch_todos(가짜_토큰, 가짜_페이지_ID).await.unwrap();
        assert_eq!(
            todos.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["block-1", "block-2"]
        );
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

    // --- U3. 쓰기 경로 — 추가·토글·편집·페이지 생성 픽스처 ---

    const 가짜_블록_ID: &str = "99990000-1111-2222-3333-444455556666";
    const 가짜_새_페이지_ID: &str = "aaaa0000-1111-2222-3333-444455556666";

    fn 블록_경로() -> String {
        format!("/v1/blocks/{가짜_블록_ID}")
    }

    #[tokio::test]
    async fn 추가_요청_body가_to_do_블록_한_개를_담는다() {
        let server = MockServer::start().await;
        // children 1개, after 없음(끝 추가), plain text 1조각, checked: false — 정확히 이 body만 매치
        let 추가_body = json!({
            "children": [ {
                "object": "block",
                "type": "to_do",
                "to_do": {
                    "rich_text": [ { "type": "text", "text": { "content": "10:00 알고리즘 1문제" } } ],
                    "checked": false
                }
            } ]
        });
        Mock::given(method("PATCH"))
            .and(path(children_경로()))
            .and(header("Notion-Version", NOTION_VERSION))
            .and(body_json(추가_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "list", "results": [] })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        client
            .append_todo(가짜_토큰, 가짜_페이지_ID, "10:00 알고리즘 1문제")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn 토글은_checked만_보내고_rich_text를_보내지_않는다() {
        let server = MockServer::start().await;
        // body_json은 정확 일치 — rich_text 등 다른 필드가 섞이면 매치가 실패해 404가 난다
        Mock::given(method("PATCH"))
            .and(path(블록_경로()))
            .and(body_json(json!({ "to_do": { "checked": true } })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "object": "block" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        client
            .set_todo_checked(가짜_토큰, 가짜_블록_ID, true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn 텍스트_편집은_rich_text만_보내고_checked를_보내지_않는다() {
        let server = MockServer::start().await;
        // rich_text 전체 교체만 전송 — checked가 섞이면 정확 일치 매치가 실패한다
        let 편집_body = json!({
            "to_do": {
                "rich_text": [ { "type": "text", "text": { "content": "11:00 알고리즘 2문제" } } ]
            }
        });
        Mock::given(method("PATCH"))
            .and(path(블록_경로()))
            .and(body_json(편집_body))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "object": "block" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        client
            .set_todo_text(가짜_토큰, 가짜_블록_ID, "11:00 알고리즘 2문제")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn 페이지_생성_body에_data_source_parent와_날짜와_골격_children이_담긴다() {
        let server = MockServer::start().await;
        // 스키마 조회 — title 키가 "이름"이 아닌 DB도 있으므로 하드코딩 검증용 가짜 키를 쓴다
        let mut ds = data_source_응답();
        let props = ds["properties"].as_object_mut().unwrap();
        props.remove("이름");
        props.insert(
            "할일제목".to_string(),
            json!({ "id": "title", "type": "title", "title": {} }),
        );
        Mock::given(method("GET"))
            .and(path(ds_경로()))
            .respond_with(ResponseTemplate::new(200).set_body_json(ds))
            .expect(1)
            .mount(&server)
            .await;

        // 생성 body — 조회로 얻은 title 키 + 날짜(date-only) + heading_3 골격 2개
        let 생성_body = json!({
            "parent": { "type": "data_source_id", "data_source_id": 가짜_DS_ID },
            "properties": {
                "할일제목": {
                    "title": [ { "type": "text", "text": { "content": "[TODO]" } } ]
                },
                "날짜": { "date": { "start": 가짜_날짜 } }
            },
            "children": [
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "공부" } } ] } },
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "기타" } } ] } }
            ]
        });
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let page_id = client
            .create_day_page(가짜_토큰, 가짜_DS_ID, 가짜_날짜, None)
            .await
            .unwrap();
        assert_eq!(page_id, 가짜_새_페이지_ID);
    }

    // --- U1(M2 확장). 아이콘 복사 — 최신 [TODO] 행 아이콘 조회 + 생성 body icon ---

    fn 아이콘_쿼리_body() -> serde_json::Value {
        json!({
            "filter": { "property": "title", "title": { "equals": "[TODO]" } },
            "sorts": [ { "property": "날짜", "direction": "descending" } ],
            "page_size": 1
        })
    }

    fn 아이콘_행(icon: serde_json::Value) -> serde_json::Value {
        let mut row = 페이지_행(가짜_페이지_ID, "[TODO]");
        row["icon"] = icon;
        row
    }

    async fn mount_아이콘_쿼리(server: &MockServer, rows: Vec<serde_json::Value>) {
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(ResponseTemplate::new(200).set_body_json(쿼리_응답(rows)))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn 최신_TODO_행_쿼리_body가_제목_필터와_날짜_내림차순으로_전송된다() {
        let server = MockServer::start().await;
        // 제목 필터는 표시 이름("이름")이 아니라 고정 id "title"을 써야 한다.
        // body_json은 정확 일치 — 필터·정렬·page_size가 다르면 매치가 실패한다.
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .and(header("Notion-Version", NOTION_VERSION))
            .and(body_json(아이콘_쿼리_body()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(쿼리_응답(vec![아이콘_행(
                    json!({ "type": "emoji", "emoji": "🌊" }),
                )])),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let icon = client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await;
        assert_eq!(icon, Some(PageIcon::Emoji("🌊".to_string())));
    }

    #[tokio::test]
    async fn emoji_아이콘은_그대로_복사된다() {
        let server = MockServer::start().await;
        mount_아이콘_쿼리(
            &server,
            vec![아이콘_행(json!({ "type": "emoji", "emoji": "✅" }))],
        )
        .await;

        let client = NotionClient::new(server.uri());
        let icon = client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await;
        assert_eq!(icon, Some(PageIcon::Emoji("✅".to_string())));
    }

    #[tokio::test]
    async fn external_아이콘은_url로_복사된다() {
        let server = MockServer::start().await;
        mount_아이콘_쿼리(
            &server,
            vec![아이콘_행(json!({
                "type": "external",
                "external": { "url": "https://example.com/icon.png" }
            }))],
        )
        .await;

        let client = NotionClient::new(server.uri());
        let icon = client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await;
        assert_eq!(
            icon,
            Some(PageIcon::External("https://example.com/icon.png".to_string()))
        );
    }

    #[tokio::test]
    async fn file_아이콘은_기본_이모지로_폴백한다() {
        // file 아이콘의 URL은 1시간 만료라 복사할 수 없다 — 기본 이모지로 폴백
        let server = MockServer::start().await;
        mount_아이콘_쿼리(
            &server,
            vec![아이콘_행(json!({
                "type": "file",
                "file": { "url": "https://s3.example.com/x.png",
                          "expiry_time": "2026-08-10T00:00:00.000Z" }
            }))],
        )
        .await;

        let client = NotionClient::new(server.uri());
        let icon = client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await;
        assert_eq!(icon, Some(PageIcon::Emoji(DEFAULT_PAGE_ICON.to_string())));
    }

    #[tokio::test]
    async fn custom_emoji_아이콘도_기본_이모지로_폴백한다() {
        // custom_emoji는 워크스페이스 종속이라 복사 범위 밖 — 기본 이모지로 폴백
        let server = MockServer::start().await;
        mount_아이콘_쿼리(
            &server,
            vec![아이콘_행(json!({
                "type": "custom_emoji",
                "custom_emoji": { "id": "ce-1", "name": "펭귄" }
            }))],
        )
        .await;

        let client = NotionClient::new(server.uri());
        let icon = client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await;
        assert_eq!(icon, Some(PageIcon::Emoji(DEFAULT_PAGE_ICON.to_string())));
    }

    #[tokio::test]
    async fn 아이콘_없는_행과_빈_결과는_None을_돌려준다() {
        // 행은 있지만 icon이 null
        let server = MockServer::start().await;
        mount_아이콘_쿼리(&server, vec![아이콘_행(json!(null))]).await;
        let client = NotionClient::new(server.uri());
        assert_eq!(client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await, None);

        // 결과가 아예 없음
        let server = MockServer::start().await;
        mount_아이콘_쿼리(&server, vec![]).await;
        let client = NotionClient::new(server.uri());
        assert_eq!(client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await, None);
    }

    #[tokio::test]
    async fn 조회_실패는_오류_대신_None으로_수렴한다() {
        // 아이콘 복사는 부가 기능 — 500이어도 페이지 생성을 막지 않게 None으로 수렴
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(쿼리_경로()))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(에러_body(500, "internal_server_error")),
            )
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        assert_eq!(client.latest_todo_icon(가짜_토큰, 가짜_DS_ID).await, None);
    }

    #[tokio::test]
    async fn 아이콘이_있으면_생성_body에_icon이_포함되고_없으면_생략된다() {
        let 생성_body_공통 = json!({
            "parent": { "type": "data_source_id", "data_source_id": 가짜_DS_ID },
            "properties": {
                "이름": {
                    "title": [ { "type": "text", "text": { "content": "[TODO]" } } ]
                },
                "날짜": { "date": { "start": 가짜_날짜 } }
            },
            "children": [
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "공부" } } ] } },
                { "object": "block", "type": "heading_3",
                  "heading_3": { "rich_text": [ { "type": "text", "text": { "content": "기타" } } ] } }
            ]
        });

        // Some(emoji) → 최상위 icon 포함 (body_json 정확 일치)
        let server = MockServer::start().await;
        mount_정상_data_source(&server).await;
        let mut icon_포함_body = 생성_body_공통.clone();
        icon_포함_body["icon"] = json!({ "type": "emoji", "emoji": "🌊" });
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(icon_포함_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let icon = PageIcon::Emoji("🌊".to_string());
        let page_id = client
            .create_day_page(가짜_토큰, 가짜_DS_ID, 가짜_날짜, Some(&icon))
            .await
            .unwrap();
        assert_eq!(page_id, 가짜_새_페이지_ID);

        // Some(external) → external icon 포함
        let server = MockServer::start().await;
        mount_정상_data_source(&server).await;
        let mut external_포함_body = 생성_body_공통.clone();
        external_포함_body["icon"] =
            json!({ "type": "external", "external": { "url": "https://example.com/i.png" } });
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(external_포함_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let icon = PageIcon::External("https://example.com/i.png".to_string());
        let page_id = client
            .create_day_page(가짜_토큰, 가짜_DS_ID, 가짜_날짜, Some(&icon))
            .await
            .unwrap();
        assert_eq!(page_id, 가짜_새_페이지_ID);

        // None → icon 키 자체가 없어야 한다 (body_json 정확 일치가 이를 보장)
        let server = MockServer::start().await;
        mount_정상_data_source(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(생성_body_공통))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "object": "page", "id": 가짜_새_페이지_ID })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let page_id = client
            .create_day_page(가짜_토큰, 가짜_DS_ID, 가짜_날짜, None)
            .await
            .unwrap();
        assert_eq!(page_id, 가짜_새_페이지_ID);
    }

    #[tokio::test]
    async fn 소실된_블록_업데이트는_404를_NotFound로_매핑한다() {
        // 다른 곳에서 블록이 삭제된 뒤 토글 시도 — object_not_found → NotFound
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(블록_경로()))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(에러_body(404, "object_not_found")),
            )
            .mount(&server)
            .await;

        let client = NotionClient::new(server.uri());
        let err = client
            .set_todo_checked(가짜_토큰, 가짜_블록_ID, false)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::NotFound);
    }

    #[tokio::test]
    async fn 긴_텍스트는_HTTP_호출_전에_거부된다() {
        // 서버가 없는 주소 — 요청이 나갔다면 Network 오류가 됐을 것이다.
        // TooLong이 나오면 가드가 HTTP보다 먼저 동작한 것이다.
        let client = NotionClient::new("http://127.0.0.1:1");
        let 긴_텍스트 = "가".repeat(2001);

        let err = client
            .append_todo(가짜_토큰, 가짜_페이지_ID, &긴_텍스트)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::TooLong);

        let err = client
            .set_todo_text(가짜_토큰, 가짜_블록_ID, &긴_텍스트)
            .await
            .unwrap_err();
        assert_eq!(err, ConnectError::TooLong);
    }
}
