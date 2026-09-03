import type { PetSnapshot } from "../lib/pet";
import {
  playCatch,
  playFreakout,
  playRoll,
  playSquawk,
  playStrike,
  playWhack,
  playWhoosh,
} from "./synth";

/** 낼 수 있는 소리 일곱. 이게 전부다 — 걷기·헤엄·착지·졸기는 무음이다.
 *
 * `strike`와 `roll`은 볼링에서만 난다. 자격 규칙 ①("사용자가 방금 한 짓의
 * 결과")에 그대로 부합한다 — 판을 연 것도 공을 굴린 것도 사용자다.
 * `roll`만 **공 창**이 내고 나머지는 펭귄 창이 낸다. */
export type SoundName =
  | "whack"
  | "whoosh"
  | "squawk"
  | "freakout"
  | "catch"
  | "strike"
  | "roll"
  | "punch"
  | "punch-down";

/** 직전 스냅샷과 비교해 이번에 낼 소리를 판정한다. 순수 함수 — Web Audio가 */
export const soundsFor = (
  prev: PetSnapshot | null,
  next: PetSnapshot,
): SoundName[] => {
  if (!prev) return [];
  const out: SoundName[] = [];
  if (next.whack_seq > prev.whack_seq) out.push("whack");
  // **야차의 "퍽".** 라운드마다 **딱 한 마리**의 `punch_seq`만 오른다 — 맞는
  // 마리마다 내면 여덟 마리에서 라운드당 네 발이 겹쳐 기관총이 된다. 대표를
  // 고르는 것은 코어의 판이라, 여기서는 "내 것이 늘었나"만 본다.
  // 쓰러뜨린 한 방은 반음을 낮춰 더 낮고 길게 낸다 — 마흔 발이 다 똑같으면
  // 그게 소음이다.
  if (next.punch_seq > prev.punch_seq) {
    out.push(next.punch_down ? "punch-down" : "punch");
  }
  if (prev.behavior.kind !== "thrown" && next.behavior.kind === "thrown") {
    // 볼링 핀이 맞아 날아가는 것은 **맞은 소리**가 나야 한다. 휙 소리만 나면
    // 스스로 날아간 것처럼 들려 공에 맞았다는 사실이 사라진다.
    out.push(prev.behavior.kind === "bowling" ? "strike" : "whoosh");
  }
  if (prev.behavior.kind !== "squawk" && next.behavior.kind === "squawk") {
    out.push("squawk");
  }
  const wasDash = prev.behavior.kind === "freakout" && prev.behavior.freakout === "dash";
  const isDash = next.behavior.kind === "freakout" && next.behavior.freakout === "dash";
  if (!wasDash && isDash) out.push("freakout");
  const wasCatch =
    prev.behavior.kind === "ice_fishing" && prev.behavior.fishing === "catch";
  const isCatch =
    next.behavior.kind === "ice_fishing" && next.behavior.fishing === "catch";
  if (!wasCatch && isCatch) out.push("catch");
  return out;
};

/** 소리별 최소 간격(ms). 빽빽거리기는 때리는 동안 계속 판을 새로 열어 */
export const SOUND_COOLDOWN_MS: Record<SoundName, number> = {
  whack: 70,
  whoosh: 150,
  squawk: 400,
  freakout: 1000,
  catch: 1500,
  // 연쇄로 여러 마리가 거의 동시에 맞는다. 창이 마리마다 따로라 쿨다운은
  // 한 마리 안에서만 걸리므로 짧게 둔다 — 길면 두 번 맞은 것이 한 번으로 들린다.
  strike: 60,
  roll: 900,
  // 라운드가 340ms마다 도는데 대표는 한 마리뿐이라, 한 마리 안에서 겹칠 일은
  // 거의 없다. 그래도 틱이 밀려 두 라운드가 붙는 경우를 거른다.
  punch: 200,
  // 쓰러뜨린 한 방은 그 라운드의 퍽과 **겹쳐서** 난다 — 서로를 안 거르게
  // 이름을 나눴고, 그래서 쿨다운도 따로다.
  "punch-down": 200,
};

/** 시각을 인자로 받아 시계 없이 테스트한다 (`pet.rs`가 `now_ms`를 받는 이유와 같다). */
export const passesCooldown = (
  name: SoundName,
  lastAt: number | undefined,
  now: number,
): boolean => lastAt === undefined || now - lastAt >= SOUND_COOLDOWN_MS[name];

/** 창 라벨(`pet-<id>`)에서 반음 오프셋을 결정적으로 뽑는다 — 마리마다 목소리가 */
export const voiceOffsetFor = (label: string): number => {
  const m = /^pet-(\d+)$/.exec(label);
  if (!m) return 0;
  return ((Number(m[1]) * 7) % 12) - 5;
};

/** 음량 단계 — 0~4의 다섯 단계, 가운데(2)가 원래의 -18 dBFS다. */
export const DEFAULT_VOLUME_STEP = 2;
export const VOLUME_MAX_STEP = 4;

/** 단계 → 마스터 게인. 이상한 값은 가운데 단계(지금 크기)로 수렴한다. */
export const gainForVolume = (step: number): number => {
  const s =
    Number.isInteger(step) && step >= 0 && step <= VOLUME_MAX_STEP
      ? step
      : DEFAULT_VOLUME_STEP;
  return 0.12 * Math.pow(2, s - DEFAULT_VOLUME_STEP);
};

const SYNTH: Record<
  SoundName,
  (ctx: BaseAudioContext, out: AudioNode, semitones: number) => void
> = {
  whack: playWhack,
  // **새 합성 함수를 안 만든다** — `playWhack`("퍽 — 노이즈 버스트 + 저역 툭")이
  // 이미 정확히 그 소리다. 야차의 퍽은 반음만 다르게 준다.
  punch: (ctx, out, semis) => playWhack(ctx, out, semis + 2),
  "punch-down": (ctx, out, semis) => playWhack(ctx, out, semis - 7),
  whoosh: playWhoosh,
  squawk: playSquawk,
  freakout: playFreakout,
  catch: playCatch,
  strike: playStrike,
  roll: playRoll,
};

/** 한 펭귄 창의 소리 전부 — 컨텍스트 수명, 켜짐/꺼짐, 쿨다운을 소유한다. */
export class SoundPlayer {
  private ctx: AudioContext | null = null;
  private out: GainNode | null = null;
  private enabled = false;
  private lastAt: Partial<Record<SoundName, number>> = {};
  private readonly semitones: number;

  constructor(label: string, createContext?: () => AudioContext) {
    this.semitones = voiceOffsetFor(label);
    try {
      if (createContext) this.ctx = createContext();
      else if (typeof AudioContext !== "undefined") this.ctx = new AudioContext();
      if (this.ctx) {
        this.out = this.ctx.createGain();
        this.out.gain.value = gainForVolume(DEFAULT_VOLUME_STEP);
        this.out.connect(this.ctx.destination);
      }
    } catch {
      this.ctx = null;
      this.out = null;
    }
  }

  /** 효과음 설정. 꺼지면 어떤 상황에서도 소리가 나지 않는다 (R1). */
  setEnabled(on: boolean): void {
    this.enabled = on;
  }

  /** 음량 단계(0~4). 재생 중인 소리에도 즉시 걸린다 — 마스터가 한 곳인 이유다. */
  setVolume(step: number): void {
    if (!this.out) return;
    try {
      this.out.gain.value = gainForVolume(step);
    } catch {
    }
  }

  /** 사용자 제스처에서 부른다 — suspended 컨텍스트를 깨울 유일한 기회다. */
  nudge(): void {
    if (this.ctx && this.ctx.state !== "running") {
      this.ctx.resume().catch(() => {});
    }
  }

  /** 게이트(켜짐 → 쿨다운 → 컨텍스트 상태)를 통과하면 합성해서 재생한다. */
  play(name: SoundName, now: number): void {
    if (!this.enabled || !this.ctx || !this.out) return;
    if (!passesCooldown(name, this.lastAt[name], now)) return;
    if (this.ctx.state !== "running") {
      this.ctx.resume().catch(() => {});
      if ((this.ctx.state as AudioContextState) !== "running") return;
    }
    this.lastAt[name] = now;
    try {
      SYNTH[name](this.ctx, this.out, this.semitones);
    } catch {
    }
  }

  /** 언마운트에서 부른다 — 컨텍스트를 OS에 돌려준다. */
  close(): void {
    this.ctx?.close().catch(() => {});
    this.ctx = null;
    this.out = null;
  }
}
