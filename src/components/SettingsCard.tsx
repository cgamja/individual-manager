import { SIZE_MAX, SIZE_MIN, SIZE_STEP, type AppTheme } from "../lib/settings";

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
  /** 펭귄 크기 퍼센트 (50~150). 소품과 물리가 함께 따라온다. */
  size: number;
  onSizeChange: (size: number) => void;
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
  yachaRunning: boolean;
  onYacha: () => void;
}

/** 설정 카드. */
export function SettingsCard({
  petEnabled,
  onPetEnabledChange,
  soundEnabled,
  onSoundEnabledChange,
  volume,
  onVolumeChange,
  size,
  onSizeChange,
  theme,
  onThemeChange,
  pinballEnabled,
  onPinballEnabledChange,
  bowlingRunning,
  onBowling,
  volleyballRunning,
  onVolleyball,
  yachaRunning,
  onYacha,
}: SettingsCardProps) {
  // **판 셋은 서로를 배제한다.** 조건을 버튼마다 따로 쓰면 넷째를 더할 때 반드시
  // 하나를 빠뜨린다 — 여기서 한 번 계산해 셋이 같은 값을 본다.
  const 판이_돈다 = bowlingRunning || volleyballRunning || yachaRunning;
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

      {/* 음량과 같은 형태의 슬라이더다. 다만 값이 퍼센트라 옆에 숫자를 보인다 —
          "몇 %인지"가 이 설정의 요점이다. */}
      <div className="settings-row">
        <label htmlFor="pet-size">크기</label>
        <input
          id="pet-size"
          type="range"
          min={SIZE_MIN}
          max={SIZE_MAX}
          step={SIZE_STEP}
          value={size}
          onChange={(e) => onSizeChange(Number(e.target.value))}
        />
        <span className="settings-value">{size}%</span>
      </div>
      <p className="settings-hint">
        화면이 좁으면 줄이세요. 방망이·공·코트 같은 소품과 물리도 같이 줄어서 발이
        바닥에 붙어 있어요.
      </p>

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
          disabled={판이_돈다}
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
          disabled={판이_돈다}
        >
          {volleyballRunning ? "치는 중…" : "비치발리볼 한 판"}
        </button>
      </div>
      <p className="settings-hint">
        펭귄들이 화면 가운데에 모래톱을 깔고 네트를 사이에 두고 서요. 지푸라기 비키니를 입고
        20초쯤 한 판 쳐요. 구경만 하면 돼요 — 공이 모래에 닿으면 끝나고, 이긴 쪽은
        좋아하고 진 쪽은 약 올라요. 두 마리부터 할 수 있고 점수는 안 세요.
      </p>

      {/* **판 셋이 서로를 배제한다.** 조건을 버튼마다 따로 쓰면 넷째를 더할 때
          반드시 하나를 빠뜨린다 — 한 곳에서 계산해 셋이 같은 값을 본다. */}
      <div className="settings-row">
        <span>단체 야차</span>
        <button type="button" onClick={onYacha} disabled={판이_돈다}>
          {yachaRunning ? "치고받는 중…" : "단체 야차 한 판"}
        </button>
      </div>
      <p className="settings-hint">
        펭귄들이 화면 한가운데로 뭉쳐서 복싱 장갑을 끼고 서로 때려요. 많이 맞은
        놈부터 눈이 X자가 되면서 쓰러지고, 마지막 한 마리가 남으면 오른쪽에서
        화장한 미녀 펭귄이 챔피언 벨트를 들고 나와 채워 줘요. 둘 이상이어야
        붙고, 전적은 안 남아요.
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
