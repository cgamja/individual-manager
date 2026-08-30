import { useEffect, useState } from "react";
import type { TimerConfig } from "../lib/timer";

interface SettingsCardProps {
  config: TimerConfig;
  /** Idle에서만 편집 가능 — 그 외 상태에서는 비활성화한다. */
  disabled: boolean;
  onChange: (config: TimerConfig) => void;
  /** 바탕화면 펭귄 표시 여부 (R8). 타이머 상태와 무관하게 항상 바꿀 수 있다. */
  petEnabled: boolean;
  onPetEnabledChange: (enabled: boolean) => void;
}

/** 분 단위 입력 검증: 1~999 사이의 정수만 허용한다 (999분 상한 — 개인용 타이머에 충분하다는 가정). */
function parseMinutes(raw: string): number | null {
  if (!/^\d+$/.test(raw.trim())) {
    return null;
  }
  const value = Number(raw.trim());
  return value >= 1 && value <= 999 ? value : null;
}

/** 타이머 시간 설정 카드. */
export function SettingsCard({
  config,
  disabled,
  onChange,
  petEnabled,
  onPetEnabledChange,
}: SettingsCardProps) {
  const [focusRaw, setFocusRaw] = useState(String(config.focus_minutes));
  const [breakRaw, setBreakRaw] = useState(String(config.break_minutes));

  useEffect(() => {
    setFocusRaw(String(config.focus_minutes));
    setBreakRaw(String(config.break_minutes));
  }, [config]);

  const commit = (focusValue: string, breakValue: string) => {
    const focus = parseMinutes(focusValue);
    const brk = parseMinutes(breakValue);
    if (focus === null || brk === null) {
      // 검증 실패 시 입력값을 원래 설정값으로 되돌린다 — 화면과 실제 설정이 어긋나지 않게 한다
      setFocusRaw(String(config.focus_minutes));
      setBreakRaw(String(config.break_minutes));
      return;
    }
    if (focus !== config.focus_minutes || brk !== config.break_minutes) {
      onChange({ focus_minutes: focus, break_minutes: brk });
    }
  };

  return (
    <section className="card settings-card" aria-label="타이머 설정">
      <p className="settings-title">시간 설정</p>
      <div className="settings-row">
        <label htmlFor="focus-minutes">집중(분)</label>
        <input
          id="focus-minutes"
          inputMode="numeric"
          value={focusRaw}
          disabled={disabled}
          onChange={(e) => setFocusRaw(e.target.value)}
          onBlur={() => commit(focusRaw, breakRaw)}
        />
      </div>
      <div className="settings-row">
        <label htmlFor="break-minutes">휴식(분)</label>
        <input
          id="break-minutes"
          inputMode="numeric"
          value={breakRaw}
          disabled={disabled}
          onChange={(e) => setBreakRaw(e.target.value)}
          onBlur={() => commit(focusRaw, breakRaw)}
        />
      </div>
      {disabled && <p className="settings-hint">타이머가 유휴 상태일 때 변경할 수 있어요</p>}
      <div className="settings-row">
        <label htmlFor="pet-enabled">바탕화면 펭귄</label>
        <input
          id="pet-enabled"
          type="checkbox"
          checked={petEnabled}
          onChange={(e) => onPetEnabledChange(e.target.checked)}
        />
      </div>
    </section>
  );
}
