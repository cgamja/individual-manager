import { useCallback, useEffect, useState } from "react";
import { PetCountCard } from "./components/PetCountCard";
import { SettingsCard } from "./components/SettingsCard";
import { TauntCard } from "./components/TauntCard";
import {
  addPet,
  getPetSummary,
  removePet,
  setPetEnabled,
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
 * 여기 있는 것은 마릿수·대사·on/off 셋이 전부다.
 */
function App() {
  const [saveFailed, setSaveFailed] = useState(false);
  const [petEnabled, setPetEnabledState] = useState(DEFAULT_PET_SETTINGS.enabled);
  const [taunts, setTaunts] = useState<readonly string[]>([]);
  /** 마릿수·상한·우클릭 대상. 펭귄은 이 창 밖에서도 늘고 준다. */
  const [petSummary, setPetSummary] = useState<PetSummary>({ count: 1, max: 8, focused: null });

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const savedPet = await loadPetSettings().catch(() => DEFAULT_PET_SETTINGS);
      if (!cancelled) setPetEnabledState(savedPet.enabled);
      const savedTaunts = await loadTaunts().catch(() => []);
      if (!cancelled) setTaunts(savedTaunts);
      const summary = await getPetSummary().catch(() => null);
      if (!cancelled && summary) setPetSummary(summary);
    })();

    // 창이 다시 보일 때 재동기화 (주기 폴링 없음). **우클릭 대상**도 함께 읽는다 —
    // 다른 펭귄을 우클릭해서 열 때마다 삭제 대상이 바뀌므로, 여기서 안 읽으면
    // 엉뚱한 펭귄이 지워진 것처럼 보인다.
    const onVisibility = () => {
      if (!document.hidden) {
        getPetSummary()
          .then(setPetSummary)
          .catch(() => {});
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      cancelled = true;
      document.removeEventListener("visibilitychange", onVisibility);
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
      <TauntCard lines={taunts} onChange={(next) => void handleTauntsChange(next)} />
      <SettingsCard
        petEnabled={petEnabled}
        onPetEnabledChange={(next) => void handlePetEnabledChange(next)}
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
