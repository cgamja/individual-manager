interface SettingsCardProps {
  /** 바탕화면 펭귄 표시 여부 (R8). */
  petEnabled: boolean;
  onPetEnabledChange: (enabled: boolean) => void;
  /** 효과음 여부. **기본은 꺼짐** (PRD Q6). */
  soundEnabled: boolean;
  onSoundEnabledChange: (enabled: boolean) => void;
  /** 핀볼 모드 여부. **기본은 꺼짐** — 켜면 착지 4단계가 가려진다. */
  pinballEnabled: boolean;
  onPinballEnabledChange: (enabled: boolean) => void;
}

/**
 * 설정 카드.
 *
 * v3.0에서 타이머 시간 설정을 걷어내고 **펭귄 on/off와 소리 on/off**만 남았다가,
 * 핀볼 모드가 셋째로 붙었다. 앱이 소유하는 화면은 펭귄과 이 창뿐이다 (PRD §5.5).
 */
export function SettingsCard({
  petEnabled,
  onPetEnabledChange,
  soundEnabled,
  onSoundEnabledChange,
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
      {/* 낼 소리가 아직 없다는 것을 말해 준다 — 켰는데 조용하면 고장으로 읽힌다 */}
      <p className="settings-hint">
        아직 낼 소리가 없어요. 빽빽거리기가 들어오면 이 설정을 따릅니다.
      </p>

      <div className="settings-row">
        <label htmlFor="pinball-enabled">핀볼 모드</label>
        <input
          id="pinball-enabled"
          type="checkbox"
          checked={pinballEnabled}
          onChange={(e) => onPinballEnabledChange(e.target.checked)}
        />
      </div>
      {/* **무엇이 달라지는지**를 적는다 — 이름만 보고는 클릭이 바뀌는 걸 모른다.
          "끄면 돌아온다"도 함께 적는다: 착지를 지우는 모드로 읽히면 켜기가 무섭다 */}
      <p className="settings-hint">
        안 널브러지고 계속 튕겨요. 클릭하면 방망이 대신 펭귄이 날아가고, 커서가 채로
        바뀌어요. 끄면 원래대로 돌아와요.
      </p>
    </section>
  );
}
