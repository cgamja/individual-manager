interface SettingsCardProps {
  /** 바탕화면 펭귄 표시 여부 (R8). */
  petEnabled: boolean;
  onPetEnabledChange: (enabled: boolean) => void;
  /** 효과음 여부. **기본은 꺼짐** (PRD Q6). */
  soundEnabled: boolean;
  onSoundEnabledChange: (enabled: boolean) => void;
}

/**
 * 설정 카드.
 *
 * v3.0에서 타이머 시간 설정을 걷어내고 **펭귄 on/off와 소리 on/off**만 남았다.
 * 앱이 소유하는 화면은 펭귄과 이 창뿐이다 (PRD §5.5).
 */
export function SettingsCard({
  petEnabled,
  onPetEnabledChange,
  soundEnabled,
  onSoundEnabledChange,
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
      {/* 낼 소리가 아직 없다는 것을 말해 준다 — 켰는데 조용하면 고장으로 읽힌다 */}
      <p className="settings-hint">
        아직 낼 소리가 없어요. 빽빽거리기가 들어오면 이 설정을 따릅니다.
      </p>
    </section>
  );
}
