import type { PetSnapshot } from "../lib/pet";
import DONT_ASK_URL from "../assets/sounds/dont-ask.m4a?url";
import {
  playCatch,
  playFreakout,
  playRoll,
  playSquawk,
  playStrike,
  playWhack,
  playWhoosh,
} from "./synth";

/** 낼 수 있는 소리 여덟. 이게 전부다 — 걷기·헤엄·착지·졸기는 무음이다.
 *
 * `strike`와 `roll`은 볼링에서만 난다. 자격 규칙 ①("사용자가 방금 한 짓의
 * 결과")에 그대로 부합한다 — 판을 연 것도 공을 굴린 것도 사용자다.
 * `roll`만 **공 창**이 내고 나머지는 펭귄 창이 낸다.
 *
 * `dont_ask`만 합성이 아니라 음원 파일이다 — 사람 목소리 대사는 합성으로
 * 도달할 수 없다 (PRD §9 Q9의 예외, `MOTIONS.md` 효과음 절). */
export type SoundName =
  | "whack"
  | "whoosh"
  | "squawk"
  | "freakout"
  | "catch"
  | "strike"
  | "roll"
  | "dont_ask";

/** 합성으로 만드는 일곱. `dont_ask`만 음원 파일이라 여기 없다. */
export type SynthName = Exclude<SoundName, "dont_ask">;

/** 직전 스냅샷과 비교해 이번에 낼 소리를 판정한다. 순수 함수 — Web Audio가 */
export const soundsFor = (
  prev: PetSnapshot | null,
  next: PetSnapshot,
): SoundName[] => {
  if (!prev) return [];
  const out: SoundName[] = [];
  if (next.whack_seq > prev.whack_seq) out.push("whack");
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
  if (prev.behavior.kind !== "dont_ask" && next.behavior.kind === "dont_ask") {
    out.push("dont_ask");
  }
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
  // 동작 길이와 같다. 코어가 이미 중복 시작을 거부하지만 소리 쪽에도 벽을 둔다.
  dont_ask: 5_700,
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
  SynthName,
  (ctx: BaseAudioContext, out: AudioNode, semitones: number) => void
> = {
  whack: playWhack,
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
  /** 안물 음원. 첫 재생에서 한 번만 받아 창 수명 동안 들고 있는다 —
   * 미리 받으면 소리를 한 번도 안 쓰는 사용자(기본 꺼짐)도 매 창 154KB를 받는다. */
  private voice: AudioBuffer | null = null;
  private voiceLoading = false;
  private readonly voiceUrl: string;

  constructor(label: string, createContext?: () => AudioContext, voiceUrl = DONT_ASK_URL) {
    this.semitones = voiceOffsetFor(label);
    this.voiceUrl = voiceUrl;
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
    // 켠 직후에 눌러도 첫 판이 들리게 미리 받는다 — 켜기는 설정 창에서
    // 일어나므로 펭귄 창에는 그 뒤로 우클릭이 안 올 수 있다.
    if (on) this.warmVoice();
  }

  /** 음량 단계(0~4). 재생 중인 소리에도 즉시 걸린다 — 마스터가 한 곳인 이유다. */
  setVolume(step: number): void {
    if (!this.out) return;
    try {
      this.out.gain.value = gainForVolume(step);
    } catch {
    }
  }

  /** 사용자 제스처에서 부른다 — suspended 컨텍스트를 깨울 유일한 기회다.
   *
   * 안물 음원도 여기서 미리 받는다. 설정 창을 여는 유일한 길이 **펭귄 우클릭**
   * 이라 버튼보다 이 호출이 반드시 앞선다 — 첫 재생이 디코드를 기다리지 않는다. */
  nudge(): void {
    if (this.ctx && this.ctx.state !== "running") {
      this.ctx.resume().catch(() => {});
    }
    this.warmVoice();
  }

  /** 음원을 한 번만 받아 둔다. 꺼져 있으면 안 받는다 — 소리를 안 쓰는
   * 사용자(기본 꺼짐)가 창마다 154KB를 받을 이유가 없다. */
  private warmVoice(): void {
    const ctx = this.ctx;
    if (!this.enabled || !ctx || this.voice || this.voiceLoading) return;
    this.voiceLoading = true;
    fetch(this.voiceUrl)
      .then((r) => r.arrayBuffer())
      .then((b) => ctx.decodeAudioData(b))
      .then((buf) => {
        this.voice = buf;
      })
      .catch(() => {})
      .finally(() => {
        this.voiceLoading = false;
      });
  }

  /** 게이트(켜짐 → 쿨다운 → 컨텍스트 상태)를 통과하면 재생한다.
   * 게이트는 합성과 음원이 함께 지난다 — 음량·on/off의 원천이 하나여야 한다. */
  play(name: SoundName, now: number): void {
    if (!this.enabled || !this.ctx || !this.out) return;
    if (!passesCooldown(name, this.lastAt[name], now)) return;
    if (this.ctx.state !== "running") {
      this.ctx.resume().catch(() => {});
      if ((this.ctx.state as AudioContextState) !== "running") return;
    }
    if (name === "dont_ask") {
      // 아직 못 받았으면 **쿨다운도 안 태운다** — 태우면 첫 판이 무음인 채로
      // 5.7초 동안 다시 눌러도 조용하다.
      if (!this.startVoice()) return;
      this.lastAt[name] = now;
      return;
    }
    this.lastAt[name] = now;
    try {
      SYNTH[name](this.ctx, this.out, this.semitones);
    } catch {
    }
  }

  /** 음원 한 발. 아직 못 받았으면 받아 두고 `false`를 돌려준다 — 큐에 쌓으면
   * 뒤늦은 목소리가 이미 끝난 춤 위에 흐른다. */
  private startVoice(): boolean {
    const { ctx, out, voice } = this;
    if (!ctx || !out) return false;
    if (!voice) {
      this.warmVoice();
      return false;
    }
    try {
      const src = ctx.createBufferSource();
      src.buffer = voice;
      src.connect(out);
      src.start();
      return true;
    } catch {
      return false;
    }
  }

  /** 언마운트에서 부른다 — 컨텍스트를 OS에 돌려준다. */
  close(): void {
    this.ctx?.close().catch(() => {});
    this.ctx = null;
    this.out = null;
    this.voice = null;
  }
}
