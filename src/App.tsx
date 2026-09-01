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
  onPetSettings,
  removePet,
  setPetEnabled,
  setPetPinball,
  slidePet,
  squawkPet,
  freakoutPet,
  type PetSummary,
} from "./lib/pet";
import {
  DEFAULT_PET_SETTINGS,
  loadPetSettings,
  loadTaunts,
  savePetSettings,
  saveTaunts,
} from "./lib/settings";
import "./App.css";

/**
 * 설정 창 — 펭귄을 우클릭하면 그 옆에서 열린다 (PRD §5.5).
 *
 * v3.0에서 타이머·알림·런처를 전부 걷어내, 앱이 소유하는 화면은 **펭귄과 이 창**뿐이다.
 * 여기 있는 것은 마릿수·대사·on/off 셋과, 동작을 지금 시켜보는 버튼이다 —
 * 얼음낚시처럼 십 분에 한 번 나오는 동작은 기다려서는 확인할 수 없다.
 */
/** 시켜볼 수 있는 동작들. 설명은 **끝나는 조건**을 적는다 — 모르고 건드리면
 * 버튼이 안 먹은 것처럼 보인다. */
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
  const [petEnabled, setPetEnabledState] = useState(DEFAULT_PET_SETTINGS.enabled);
  const [soundEnabled, setSoundEnabledState] = useState(DEFAULT_PET_SETTINGS.sound);
  const [pinballEnabled, setPinballEnabledState] = useState(DEFAULT_PET_SETTINGS.pinball);
  const [volume, setVolumeState] = useState(DEFAULT_PET_SETTINGS.volume);
  const [taunts, setTaunts] = useState<readonly string[]>([]);
  /** 마릿수·상한·우클릭 대상. 펭귄은 이 창 밖에서도 늘고 준다. */
  const [petSummary, setPetSummary] = useState<PetSummary>({ count: 1, max: 8, focused: null });

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const savedPet = await loadPetSettings().catch(() => DEFAULT_PET_SETTINGS);
      if (!cancelled) {
        setPetEnabledState(savedPet.enabled);
        setSoundEnabledState(savedPet.sound);
        setPinballEnabledState(savedPet.pinball);
        setVolumeState(savedPet.volume);
      }
      const savedTaunts = await loadTaunts().catch(() => []);
      if (!cancelled) setTaunts(savedTaunts);
      const summary = await getPetSummary().catch(() => null);
      if (!cancelled && summary) setPetSummary(summary);
    })();

    // 창이 다시 보일 때 재동기화 (주기 폴링 없음). **우클릭 대상**도 함께 읽는다 —
    // 다른 펭귄을 우클릭해서 열 때마다 삭제 대상이 바뀌므로, 여기서 안 읽으면
    // 엉뚱한 펭귄이 지워진 것처럼 보인다.
    //
    // **펭귄 설정도 다시 읽는다.** 이 창 밖에서 바뀔 수 있다 — 핀볼 판에서 Esc를
    // 누르면 저장소가 바뀌는데, 여기서 안 읽으면 설정 창은 켜진 것으로 보여
    // 체크를 껐다 켜야 실제로 켜지는 꼴이 된다.
    const onVisibility = () => {
      if (!document.hidden) {
        // **맨 위로 되돌린다.** 이 창은 닫을 때 파괴되지 않고 숨겨질 뿐이라
        // 스크롤 위치가 그대로 남는다. 대사를 편집하러 한 번 내려가면 그다음부터
        // 계속 내려간 채로 열려서, 맨 위 카드(펭귄 추가·삭제)가 사라진 것처럼 보인다.
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
          })
          .catch(() => {});
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    // **창이 열려 있는 채로도 바뀔 수 있다.** 핀볼 판에서 Esc를 누르면 저장소가
    // 바뀌는데 그때는 `visibilitychange`가 안 뜬다 — 체크가 켜진 채로 남아
    // 껐다 켜야 실제로 켜지는 꼴이 된다.
    let unlisten: UnlistenFn | undefined;
    void onPetSettings(({ pinball }) => {
      setPinballEnabledState(pinball);
      // 설정이 밖에서 바뀌었으면 마릿수·우클릭 대상도 같이 어긋났을 수 있다
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
    };
  }, []);

  /** 펭귄 on/off — Rust가 창을 만들거나 닫고, 저장은 여기서 한다.
   * 커맨드가 실패하면 화면 표시를 되돌린다 — 켜지지 않았는데 켜진 것처럼 보이지 않게. */
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
      // 창은 바뀌었는데 저장만 실패했다. 표시만 되돌리면 "꺼짐인데 떠 있는"
      // 상태가 되므로 창도 함께 원복해 화면과 실제를 맞춘다
      console.error("펭귄 설정 저장 실패:", err);
      await setPetEnabled(!next).catch(() => {});
      setPetEnabledState(!next);
    }
    await getPetSummary()
      .then(setPetSummary)
      .catch(() => {});
  }, []);

  /** 소리 on/off — 저장하고, 성공하면 떠 있는 펭귄 전부에 방송한다 (R2).
   * 저장에 실패하면 표시를 되돌리고 방송도 안 한다 — "저장은 실패했는데
   * 소리는 켜진" 상태를 만들지 않게. */
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
      // 방송 실패는 표시를 되돌리지 않는다 — 저장은 이미 됐고, 다음 실행이 맞춘다
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

  /** 핀볼 on/off — **거는 것과 저장하는 것 둘 다 한다.** 거는 쪽은 지금 떠 있는
   * 펭귄들에게, 저장은 다음에 태어날 펭귄을 위해. 어느 한쪽이 실패하면 표시를
   * 되돌린다 — 켜졌다고 보이는데 안 튀는 상태를 만들지 않게 (`handlePetEnabledChange`와
   * 같은 규칙이다). */
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

  /** 펭귄 추가·삭제 — 결과를 낙관적으로 그리지 않고 **다시 읽는다.** 상한이나
   * 마지막 한 마리에 걸려 거부될 수 있고, 그때 화면만 늘어나면 거짓말이 된다. */
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
        pinballEnabled={pinballEnabled}
        onPinballEnabledChange={(next) => void handlePinballEnabledChange(next)}
      />
      {saveFailed && (
        <p className="notif-hint" role="status">
          설정 저장에 실패했어요 — 변경은 이번 실행에만 적용돼요
        </p>
      )}
    </main>
  );
}

export default App;
