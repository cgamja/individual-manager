interface SettingsCardProps {
  /** 바탕화면 펭귄 표시 여부 (R8). */
  petEnabled: boolean;
  onPetEnabledChange: (enabled: boolean) => void;
}

/**
 * 설정 카드.
 *
 * v3.0에서 타이머 시간 설정을 걷어내고 **펭귄 on/off만** 남았다. 앱이 소유하는
 * 화면은 펭귄과 이 창뿐이다 (PRD §5.5).
 */
export function SettingsCard({ petEnabled, onPetEnabledChange }: SettingsCardProps) {
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
      <p className="settings-hint">
        끄면 펭귄이 사라져요. 마릿수는 그대로 기억해요.
      </p>
    </section>
  );
}
