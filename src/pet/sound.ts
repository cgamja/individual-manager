import type { PetSnapshot } from "../lib/pet";

/**
 * 펭귄이 낼 수 있는 소리 넷. 이게 전부다 — 걷기·헤엄·착지·졸기는 무음이다.
 *
 * 기준은 "사용자가 방금 한 짓의 결과이거나, 시간당 한 번보다 드물거나"다.
 * 저절로 나면서 자주 나는 소리(착지 3종은 시간당 150회가 넘는다)는 상주 앱을
 * 고문으로 만든다 — 눈은 감을 수 있지만 귀는 못 감는다.
 */
export type SoundName = "whack" | "whoosh" | "squawk" | "freakout";

/**
 * 직전 스냅샷과 비교해 이번에 낼 소리를 판정한다. 순수 함수 — Web Audio가
 * 한 줄도 안 들어가므로 jsdom에서 전부 테스트된다.
 *
 * 퍽은 동작이 아니라 `whack_seq`(맞은 횟수)로 잡는다. 빽빽거리는 중에 또
 * 맞으면 스윙 없이 판만 새로 열리는데(`pet.rs`의 whack), 그때도 횟수는
 * 늘어난다 — 소리는 방망이가 아니라 충격의 것이다. 핀볼 타격은 횟수가 안
 * 늘어 퍽이 저절로 빠지고 휙만 남는다.
 *
 * 나머지 셋은 동작의 edge다: 같은 동작이 이어지는 동안(말풍선만 바뀌어도
 * 스냅샷은 다시 온다) 소리를 또 내면 안 되므로 "아니었다가 됐다"만 본다.
 */
export const soundsFor = (
  prev: PetSnapshot | null,
  next: PetSnapshot,
): SoundName[] => {
  // 첫 스냅샷은 무음 — 켜자마자 마침 빽빽거리는 중이었다고 소리를 내면
  // 원인 없는 소리가 된다
  if (!prev) return [];
  const out: SoundName[] = [];
  if (next.whack_seq > prev.whack_seq) out.push("whack");
  if (prev.behavior.kind !== "thrown" && next.behavior.kind === "thrown") {
    out.push("whoosh");
  }
  if (prev.behavior.kind !== "squawk" && next.behavior.kind === "squawk") {
    out.push("squawk");
  }
  // 광란은 돌진 국면의 것이다 — 숨 고르기(pant) 진입은 무음
  const wasDash = prev.behavior.kind === "freakout" && prev.behavior.freakout === "dash";
  const isDash = next.behavior.kind === "freakout" && next.behavior.freakout === "dash";
  if (!wasDash && isDash) out.push("freakout");
  return out;
};

/**
 * 소리별 최소 간격(ms). 빽빽거리기는 때리는 동안 계속 판을 새로 열어
 * 클릭마다 전이가 생기므로, 이게 없으면 초당 5~10발의 빽 소리가 겹쳐
 * 화난 펭귄이 아니라 고장난 스피커가 된다 (KTD8).
 */
export const SOUND_COOLDOWN_MS: Record<SoundName, number> = {
  // 사람이 낼 수 있는 최고 연타 속도보다 짧다 — 정상 연타는 하나도 안 잘린다
  whack: 70,
  // 소리 길이(≈180ms)의 대부분. 핀볼 랠리의 정상 간격보다는 짧다
  whoosh: 150,
  // 소리 길이(≈350ms)보다 길다 — 연타 중에도 이어지는 하나의 화로 들린다
  squawk: 400,
  // 한 판에 한 번이면 충분하다
  freakout: 1000,
};

/** 시각을 인자로 받아 시계 없이 테스트한다 (`pet.rs`가 `now_ms`를 받는 이유와 같다). */
export const passesCooldown = (
  name: SoundName,
  lastAt: number | undefined,
  now: number,
): boolean => lastAt === undefined || now - lastAt >= SOUND_COOLDOWN_MS[name];

/**
 * 창 라벨(`pet-<id>`)에서 반음 오프셋을 결정적으로 뽑는다 — 마리마다 목소리가
 * 다르되, 같은 펭귄은 껐다 켜도 같은 목소리여야 "그 펭귄"으로 읽힌다
 * (PRINCIPLE 3). `Math.random()`을 쓰지 않는 이유가 이것이다.
 *
 * 7은 12와 서로소라 연속한 id 열두 개가 전부 다른 음을 받는다(상한은 8마리다).
 * 라벨이 예상 밖이면 기준음(0)이다.
 */
export const voiceOffsetFor = (label: string): number => {
  const m = /^pet-(\d+)$/.exec(label);
  if (!m) return 0;
  return ((Number(m[1]) * 7) % 12) - 5;
};
