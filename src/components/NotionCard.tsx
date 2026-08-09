import { useState } from "react";
import type { ConnectionState } from "../lib/notion";

interface NotionCardProps {
  status: ConnectionState;
  /** 저장·검증 커맨드가 진행 중이면 세 트리거를 모두 잠근다. */
  isVerifying: boolean;
  /** resolve된 상태가 failed가 아니면 입력 필드를 비운다 — reject면 유지. */
  onSaveToken: (token: string) => Promise<ConnectionState>;
  onDeleteToken: () => void;
  onSetDatabase: (input: string) => void;
  onTestConnection: () => void;
}

const MISSING_HINT: Record<"token" | "database" | "both", string> = {
  token: "토큰을 입력해 주세요",
  database: "Database를 지정해 주세요",
  both: "토큰과 Database를 입력해 주세요",
};

/** Notion 연결 설정 카드 — 토큰·DB 지정과 연결 상태 표시. */
export function NotionCard({
  status,
  isVerifying,
  onSaveToken,
  onDeleteToken,
  onSetDatabase,
  onTestConnection,
}: NotionCardProps) {
  const [tokenRaw, setTokenRaw] = useState("");
  const [dbRaw, setDbRaw] = useState("");

  // 토큰 저장 여부는 상태에서 파생 — missing이 token/both가 아니면 저장된 것
  const tokenSaved = !(
    status.state === "not_configured" &&
    (status.missing === "token" || status.missing === "both")
  );

  const handleTokenSave = async () => {
    const token = tokenRaw.trim();
    if (token === "") return;
    try {
      const result = await onSaveToken(token);
      // 검증까지 성공(connected 또는 DB 미지정으로 검증 생략)일 때만 비운다
      if (result.state !== "failed") {
        setTokenRaw("");
      }
    } catch {
      // 커맨드 reject — 입력을 유지해 재시도할 수 있게 둔다
    }
  };

  const handleDatabaseSave = () => {
    const input = dbRaw.trim();
    if (input === "") return;
    onSetDatabase(input);
  };

  return (
    <section className="card notion-card" aria-label="Notion 연결 설정">
      <p className="settings-title">Notion 연결</p>

      <div className="notion-row">
        <label htmlFor="notion-token">Integration 토큰</label>
        <div className="notion-input-group">
          <input
            id="notion-token"
            type="password"
            autoComplete="off"
            value={tokenRaw}
            onChange={(e) => setTokenRaw(e.target.value)}
          />
          {/* onBlur 자동 커밋 금지 — 시크릿 필드의 우발적 blur 저장을 막는다 */}
          <button
            type="button"
            aria-label="토큰 저장"
            disabled={isVerifying}
            onClick={handleTokenSave}
          >
            저장
          </button>
        </div>
        {tokenSaved && (
          <div className="notion-token-meta">
            <span className="notion-badge">저장됨</span>
            <button type="button" disabled={isVerifying} onClick={onDeleteToken}>
              삭제
            </button>
          </div>
        )}
      </div>

      <div className="notion-row">
        <label htmlFor="notion-database">Database URL/ID</label>
        <div className="notion-input-group">
          <input
            id="notion-database"
            type="text"
            autoComplete="off"
            value={dbRaw}
            onChange={(e) => setDbRaw(e.target.value)}
          />
          <button
            type="button"
            aria-label="Database 저장"
            disabled={isVerifying}
            onClick={handleDatabaseSave}
          >
            저장
          </button>
        </div>
      </div>

      <div className="notion-actions">
        <button type="button" disabled={isVerifying} onClick={onTestConnection}>
          연결 테스트
        </button>
      </div>

      <p className="notion-status" role="status">
        {isVerifying ? (
          "확인 중..."
        ) : status.state === "connected" ? (
          <>연결됨 · {status.title}</>
        ) : status.state === "failed" ? (
          <span className="notion-error">{status.message}</span>
        ) : (
          MISSING_HINT[status.missing]
        )}
      </p>
    </section>
  );
}
