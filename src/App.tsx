import { useCallback, useEffect, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { MotionCard, type Motion } from "./components/MotionCard";
import { PetCountCard } from "./components/PetCountCard";
import { SettingsCard } from "./components/SettingsCard";
import { TauntCard } from "./components/TauntCard";
import {
  addPet,
  emitPetSound,
  fishPet,
  getPetSummary,
  onBowlingOver,
  onPetSettings,
  removePet,
  setPetEnabled,
  setPetPinball,
  setPetSize,
  setPetTheme,
  slidePet,
  squawkPet,
  startBowling,
  startVolleyball,
  onVolleyOver,
  freakoutPet,
  type PetSummary,
} from "./lib/pet";
import {
  DEFAULT_PET_SETTINGS,
  loadPetSettings,
  loadTaunts,
  savePetSettings,
  saveTaunts,
  type AppTheme,
} from "./lib/settings";
import "./App.css";

/** 설정 창 — 펭귄을 우클릭하면 그 옆에서 열린다 (PRD §5.5). */
/** 시켜볼 수 있는 동작들. 설명은 **끝나는 조건**을 적는다 — 모르고 건드리면 */
const MOTIONS: readonly Motion[] = [
  {
    name: "얼음낚시",
    note: "30~60초 동안 앉아 있어요. 던지거나 때리면 그 자리에서 그만둬요. 헤엄치는 중에 시키면 그 높이에서 허공에 드리워요.",
    run: fishPet,
  },
  {
    name: "슬라이딩",
    note: "2.4초 동안 미끄러지고 일어서요. 바닥에 있을 때만 돼요 — 공중에서 배를 깔면 그냥 헤엄이에요.",
    run: slidePet,
  },
  {
    name: "빽빽거리기",
    note: "1.4초 동안 부풀리고 퍼덕이며 화내요. 더 때려도 안 끊겨요. 소리는 안 나요.",
    run: squawkPet,
  },
  {
    name: "발작",
    note: "2~4초 동안 사방으로 튀고, 바닥까지 내려와 0.7초 숨을 고르고 끝나요 — 높이 떠 있었으면 전체가 7초쯤 갈 수 있어요. 저절로는 며칠에 한 번 나와요.",
    run: freakoutPet,
  },
];

function App() {
  const [saveFailed, setSaveFailed] = useState(false);
  /** 판을 못 연 이유. **버튼이 눌렸는데 아무 일도 안 일어나면 고장으로 읽히므로**
   * 코어가 만든 문구를 그대로 띄운다 (PRD §5.10). 성공하면 지운다. */
  const [boardNotice, setBoardNotice] = useState<string | null>(null);
  const [petEnabled, setPetEnabledState] = useState(DEFAULT_PET_SETTINGS.enabled);
  const [soundEnabled, setSoundEnabledState] = useState(DEFAULT_PET_SETTINGS.sound);
  const [pinballEnabled, setPinballEnabledState] = useState(DEFAULT_PET_SETTINGS.pinball);
  const [volume, setVolumeState] = useState(DEFAULT_PET_SETTINGS.volume);
  const [size, setSizeState] = useState(DEFAULT_PET_SETTINGS.size);
  const [theme, setThemeState] = useState(DEFAULT_PET_SETTINGS.theme);
  const [taunts, setTaunts] = useState<readonly string[]>([]);
  /** 마릿수·상한·우클릭 대상. 펭귄은 이 창 밖에서도 늘고 준다. */
  const [petSummary, setPetSummary] = useState<PetSummary>({
    count: 1,
    max: 8,
    focused: null,
    bowling: false,
    volleyball: false,
  });

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const savedPet = await loadPetSettings().catch(() => DEFAULT_PET_SETTINGS);
      if (!cancelled) {
        setPetEnabledState(savedPet.enabled);
        setSoundEnabledState(savedPet.sound);
        setPinballEnabledState(savedPet.pinball);
        setVolumeState(savedPet.volume);
        setSizeState(savedPet.size);
        setThemeState(savedPet.theme);
      }
      const savedTaunts = await loadTaunts().catch(() => []);
      if (!cancelled) setTaunts(savedTaunts);
      const summary = await getPetSummary().catch(() => null);
      if (!cancelled && summary) setPetSummary(summary);
    })();

    const onVisibility = () => {
      if (!document.hidden) {
        window.scrollTo(0, 0);
        getPetSummary()
          .then(setPetSummary)
          .catch(() => {});
        loadPetSettings()
          .then((saved) => {
            setPetEnabledState(saved.enabled);
            setSoundEnabledState(saved.sound);
            setPinballEnabledState(saved.pinball);
            setVolumeState(saved.volume);
            setSizeState(saved.size);
            setThemeState(saved.theme);
          })
          .catch(() => {});
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    // 판이 끝나는 것은 공이 정한다 — 사용자가 아니라서 여기서 다시 읽어야
    // "볼링 한 판" 버튼이 되살아난다.
    let unlistenBowling: UnlistenFn | undefined;
    void onBowlingOver(() => {
      getPetSummary()
        .then(setPetSummary)
        .catch(() => {});
    })
      .then((off) => {
        if (cancelled) off();
        else unlistenBowling = off;
      })
      .catch(() => {});

    // 비치발리볼도 같다 — 판을 끝내는 것은 예산이지 사용자가 아니다.
    let unlistenVolley: UnlistenFn | undefined;
    void onVolleyOver(() => {
      getPetSummary()
        .then(setPetSummary)
        .catch(() => {});
    })
      .then((off) => {
        if (cancelled) off();
        else unlistenVolley = off;
      })
      .catch(() => {});

    let unlisten: UnlistenFn | undefined;
    void onPetSettings(({ pinball }) => {
      setPinballEnabledState(pinball);
      getPetSummary()
        .then(setPetSummary)
        .catch(() => {});
    })
      .then((off) => {
        if (cancelled) off();
        else unlisten = off;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      document.removeEventListener("visibilitychange", onVisibility);
      unlisten?.();
      unlistenBowling?.();
      unlistenVolley?.();
    };
  }, []);

  /** 펭귄 on/off — Rust가 창을 만들거나 닫고, 저장은 여기서 한다. */
  const handlePetEnabledChange = useCallback(async (next: boolean) => {
    setPetEnabledState(next);
    try {
      await setPetEnabled(next);
    } catch (err) {
      console.error("펭귄 표시 변경 실패:", err);
      setPetEnabledState(!next);
      return;
    }
    try {
      await savePetSettings({ enabled: next });
    } catch (err) {
      console.error("펭귄 설정 저장 실패:", err);
      await setPetEnabled(!next).catch(() => {});
      setPetEnabledState(!next);
    }
    await getPetSummary()
      .then(setPetSummary)
      .catch(() => {});
  }, []);

  /** 소리 on/off — 저장하고, 성공하면 떠 있는 펭귄 전부에 방송한다 (R2). */
  const handleSoundEnabledChange = useCallback(
    async (next: boolean) => {
      setSoundEnabledState(next);
      try {
        await savePetSettings({ sound: next });
      } catch (err) {
        console.error("소리 설정 저장 실패:", err);
        setSoundEnabledState(!next);
        return;
      }
      await emitPetSound(next, volume).catch((err) =>
        console.error("소리 설정 방송 실패:", err),
      );
    },
    [volume],
  );

  /** 음량 단계 — 토글과 같은 규칙: 저장 성공 뒤에만 방송, 실패하면 되돌린다. */
  const handleVolumeChange = useCallback(
    async (next: number) => {
      const prev = volume;
      setVolumeState(next);
      try {
        await savePetSettings({ volume: next });
      } catch (err) {
        console.error("음량 저장 실패:", err);
        setVolumeState(prev);
        return;
      }
      await emitPetSound(soundEnabled, next).catch((err) =>
        console.error("음량 방송 실패:", err),
      );
    },
    [soundEnabled, volume],
  );

  /** 크기 — 음량과 같은 순서다: 저장이 성공한 뒤에만 창에 건다. Rust의 틱과
   * 새 창은 저장된 값을 원천으로 읽으므로 순서가 뒤집히면 한 틱이 옛 배율이다. */
  const handleSizeChange = useCallback(
    async (next: number) => {
      const prev = size;
      setSizeState(next);
      try {
        await savePetSettings({ size: next });
      } catch (err) {
        console.error("크기 저장 실패:", err);
        setSizeState(prev);
        return;
      }
      await setPetSize(next).catch((err) => console.error("크기 적용 실패:", err));
    },
    [size],
  );

  /** 핀볼 on/off — **거는 것과 저장하는 것 둘 다 한다.** 거는 쪽은 지금 떠 있는 */
  const handlePinballEnabledChange = useCallback(async (next: boolean) => {
    setPinballEnabledState(next);
    try {
      await setPetPinball(next);
    } catch (err) {
      console.error("핀볼 모드 변경 실패:", err);
      setPinballEnabledState(!next);
      return;
    }
    try {
      await savePetSettings({ pinball: next });
    } catch (err) {
      console.error("핀볼 설정 저장 실패:", err);
      await setPetPinball(!next).catch(() => {});
      setPinballEnabledState(!next);
    }
  }, []);

  /** 테마 — 거는 것(지금 떠 있는 창·트레이)과 저장(다음 실행) 둘 다. */
  const handleThemeChange = useCallback(
    async (next: AppTheme) => {
      const prev = theme;
      setThemeState(next);
      try {
        await setPetTheme(next);
      } catch (err) {
        console.error("테마 변경 실패:", err);
        setThemeState(prev);
        return;
      }
      try {
        await savePetSettings({ theme: next });
      } catch (err) {
        console.error("테마 저장 실패:", err);
        await setPetTheme(prev).catch(() => {});
        setThemeState(prev);
      }
    },
    [theme],
  );

  /** 펭귄 추가·삭제 — 결과를 낙관적으로 그리지 않고 **다시 읽는다.** 상한이나 */
  const refreshPets = useCallback(async () => {
    const summary = await getPetSummary().catch(() => null);
    if (summary) setPetSummary(summary);
  }, []);

  const handlePetAdd = useCallback(async () => {
    await addPet().catch((err) => console.error("펭귄 추가 실패:", err));
    await refreshPets();
  }, [refreshPets]);

  const handlePetRemove = useCallback(async () => {
    await removePet().catch((err) => console.error("펭귄 삭제 실패:", err));
    await refreshPets();
  }, [refreshPets]);

  /** 볼링 한 판 — **저장하지 않는다.** 켜 두는 모드가 아니라 몇 초짜리
   * 한 판이라, 앱을 껐다 켜면 판은 그냥 없다 (KTD11). */
  const handleBowling = useCallback(async () => {
    try {
      await startBowling();
      setBoardNotice(null);
    } catch (err) {
      setBoardNotice(typeof err === "string" ? err : "볼링을 못 열었어요");
    }
    await refreshPets();
  }, [refreshPets]);

  /** 비치발리볼 한 판 — 볼링과 같은 규칙으로 **저장하지 않는다.** 20초짜리
   * 한 판이라 앱을 껐다 켜면 판은 그냥 없다. */
  const handleVolleyball = useCallback(async () => {
    try {
      await startVolleyball();
      setBoardNotice(null);
    } catch (err) {
      // 코어가 이유를 넷으로 갈라 준다 (두 마리부터 / 짝수만 / 이미 판이 돈다 /
      // 코트를 깔 자리가 없다). 콘솔에만 찍으면 사용자에게는 "안 눌린다"로 보인다.
      setBoardNotice(typeof err === "string" ? err : "비치발리볼을 못 열었어요");
    }
    await refreshPets();
  }, [refreshPets]);

  /** 대사 편집 — 화면을 먼저 바꾸고 저장한다. 실패하면 되돌린다. */
  const handleTauntsChange = useCallback(
    async (next: string[]) => {
      const before = taunts;
      setTaunts(next);
      try {
        await saveTaunts(next);
        setSaveFailed(false);
      } catch (err) {
        console.error("대사 저장 실패:", err);
        setTaunts(before);
        setSaveFailed(true);
      }
    },
    [taunts],
  );

  return (
    <main className="popover">
      <PetCountCard
        count={petSummary.count}
        max={petSummary.max}
        focused={petSummary.focused}
        onAdd={() => void handlePetAdd()}
        onRemove={() => void handlePetRemove()}
      />
      <MotionCard focused={petSummary.focused} motions={MOTIONS} />
      <TauntCard lines={taunts} onChange={(next) => void handleTauntsChange(next)} />
      <SettingsCard
        petEnabled={petEnabled}
        onPetEnabledChange={(next) => void handlePetEnabledChange(next)}
        soundEnabled={soundEnabled}
        onSoundEnabledChange={(next) => void handleSoundEnabledChange(next)}
        volume={volume}
        onVolumeChange={(next) => void handleVolumeChange(next)}
        size={size}
        onSizeChange={(next) => void handleSizeChange(next)}
        theme={theme}
        onThemeChange={(next) => void handleThemeChange(next)}
        pinballEnabled={pinballEnabled}
        onPinballEnabledChange={(next) => void handlePinballEnabledChange(next)}
        bowlingRunning={petSummary.bowling}
        onBowling={() => void handleBowling()}
        volleyballRunning={petSummary.volleyball}
        onVolleyball={() => void handleVolleyball()}
      />
      {boardNotice && (
        <p className="notif-hint" role="status">
          {boardNotice}
        </p>
      )}
      {saveFailed && (
        <p className="notif-hint" role="status">
          설정 저장에 실패했어요 — 변경은 이번 실행에만 적용돼요
        </p>
      )}
    </main>
  );
}

export default App;
