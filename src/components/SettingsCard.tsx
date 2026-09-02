import type { AppTheme } from "../lib/settings";

interface SettingsCardProps {
  /** 바탕화면 펭귄 표시 여부 (R8). */
  petEnabled: boolean;
  onPetEnabledChange: (enabled: boolean) => void;
  /** 효과음 여부. **기본은 꺼짐** (PRD Q6). */
  soundEnabled: boolean;
  onSoundEnabledChange: (enabled: boolean) => void;
  /** 음량 단계 0~4. 가운데(2)가 기본 크기, 단계마다 두 배(6dB)씩. */
  volume: number;
  onVolumeChange: (volume: number) => void;
  /** 겉모습 테마 — 이 창의 겉모습을 정한다. 트레이는 항상 자동(템플릿)이다. */
  theme: AppTheme;
  onThemeChange: (theme: AppTheme) => void;
  /** 핀볼 모드 여부. **기본은 꺼짐** — 켜면 착지 4단계가 가려진다. */
  pinballEnabled: boolean;
  onPinballEnabledChange: (enabled: boolean) => void;
}

/** 설정 카드. */
export function SettingsCard({
  petEnabled,
  onPetEnabledChange,
  soundEnabled,
  onSoundEnabledChange,
  volume,
  onVolumeChange,
  theme,
  onThemeChange,
  pinballEnabled,
  onPinballEnabledChange,
}: SettingsCardProps) {
  return (
    <section className="card settings-card">
      <div className="settings-row">
        <label htmlFor="pet-enabled">바탕화면 펭귄</label>
        <input
          id="pet-enabled"
          type="checkbox"
          checked={petEnabled}
          onChange={(e) => onPetEnabledChange(e.target.checked)}
        />
      </div>
      <p className="settings-hint">끄면 펭귄이 사라져요. 마릿수는 그대로 기억해요.</p>

      <div className="settings-row">
        <label htmlFor="sound-enabled">효과음</label>
        <input
          id="sound-enabled"
          type="checkbox"
          checked={soundEnabled}
          onChange={(e) => onSoundEnabledChange(e.target.checked)}
        />
      </div>
      <p className="settings-hint">
        때리거나 던지면 소리가 나요. 빽빽거리기·발작·물고기 잡는 순간에도요.
        그 밖에는 조용해요.
      </p>

      <div className="settings-row">
        <label htmlFor="sound-volume">음량</label>
        <input
          id="sound-volume"
          type="range"
          min={0}
          max={4}
          step={1}
          value={volume}
          onChange={(e) => onVolumeChange(Number(e.target.value))}
        />
      </div>

      <div className="settings-row">
        <label htmlFor="app-theme">테마</label>
        <select
          id="app-theme"
          value={theme}
          onChange={(e) => onThemeChange(e.target.value as AppTheme)}
        >
          <option value="system">시스템 설정</option>
          <option value="light">라이트</option>
          <option value="dark">다크</option>
        </select>
      </div>

      <div className="settings-row">
        <label htmlFor="pinball-enabled">핀볼 모드</label>
        <input
          id="pinball-enabled"
          type="checkbox"
          checked={pinballEnabled}
          onChange={(e) => onPinballEnabledChange(e.target.checked)}
        />
      </div>
      <p className="settings-hint">
        안 널브러지고 계속 튕겨요. 클릭하면 방망이 대신 펭귄이 날아가고, 커서가 채로
        바뀌어요. 끄면 원래대로 돌아와요.
      </p>
    </section>
  );
}
