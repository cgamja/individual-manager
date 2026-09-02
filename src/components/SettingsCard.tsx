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
  /** 볼링 판이 도는 중인가. 도는 중에 또 누르면 무시되므로 버튼을 끈다 (A3). */
  bowlingRunning: boolean;
  onBowling: () => void;
  /** 비치발리볼 판이 도는 중인가. **두 판은 서로를 배제하므로** 어느 쪽이든
   * 도는 동안 버튼 둘이 함께 비활성된다. */
  volleyballRunning: boolean;
  onVolleyball: () => void;
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
  bowlingRunning,
  onBowling,
  volleyballRunning,
  onVolleyball,
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

      {/* **토글이 아니라 버튼이다.** 볼링은 켜 두는 모드가 아니라 몇 초짜리
          한 판이고, 그래서 저장하지도 않는다. `MotionCard`가 아니라 여기 있는
          이유는 그쪽 규칙("우클릭한 펭귄이 없으면 비활성")을 볼링이 안 따르기
          때문이다 — 판에는 화면의 펭귄 전부가 참여한다 (R1). */}
      <div className="settings-row">
        <span>볼링</span>
        <button
          type="button"
          onClick={onBowling}
          disabled={bowlingRunning || volleyballRunning}
        >
          {bowlingRunning ? "굴리는 중…" : "볼링 한 판"}
        </button>
      </div>
      <p className="settings-hint">
        펭귄들이 오른쪽 바닥에 한 줄로 서요. 다 서면 왼쪽에 공이 놓이고, 마우스로
        집어 뿌리면 굴러가요. 공이 멎으면 끝나요 — 점수는 안 세요.
      </p>

      {/* **사용자가 아무것도 안 하는 유일한 판이다.** 볼링은 공을 굴려야
          진행되지만 이건 누르고 나면 20초 동안 할 일이 없다 — 그래서 안내
          문구가 "무엇을 하세요"가 아니라 "무엇이 보입니다"다. */}
      <div className="settings-row">
        <span>비치발리볼</span>
        <button
          type="button"
          onClick={onVolleyball}
          disabled={bowlingRunning || volleyballRunning}
        >
          {volleyballRunning ? "치는 중…" : "비치발리볼 한 판"}
        </button>
      </div>
      <p className="settings-hint">
        펭귄들이 화면 가운데에 떠서 마주 서고 바닥에 모래사장이 깔려요. 훌라 차림으로
        20초쯤 한 판 쳐요. 구경만 하면 돼요 — 공이 모래에 닿으면 끝나고, 이긴 쪽은
        좋아하고 진 쪽은 약 올라요. 두 마리부터 할 수 있고 점수는 안 세요.
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
      <p className="settings-hint">
        안 널브러지고 계속 튕겨요. 클릭하면 방망이 대신 펭귄이 날아가고, 커서가 채로
        바뀌어요. 끄면 원래대로 돌아와요.
      </p>
    </section>
  );
}
