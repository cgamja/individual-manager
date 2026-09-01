/**
 * 소리 다섯을 그 자리에서 합성한다 (KTD1 — Q9 확정).
 *
 * 음원 파일을 쓰지 않는 이유: 번들이 0바이트 늘고, 라이선스·출처 표기가
 * 없고, 마리마다 목소리를 값으로 흔들 수 있다. 소리 하나 고치는 데 오디오
 * 편집기가 아니라 아래 상수 한 줄이면 된다.
 *
 * **여기의 상수는 전부 귀로 맞춘 취향값이다.** 테스트로 못 박지 않는다 —
 * 값을 고정하면 소리를 다듬을 때마다 테스트가 깨진다 (`PINBALL_DAMPING`과
 * 같은 부류). `Math.random()`은 쓰지 않는다 (PRINCIPLE 3).
 */

/** 반음 오프셋 → 주파수 배율. 마리마다 목소리를 다르게 만드는 손잡이다. */
const ratio = (semitones: number): number => Math.pow(2, semitones / 12);

/**
 * 결정적 화이트노이즈 버퍼 (xorshift32, 고정 시드). `Math.random()`으로
 * 채우면 같은 펭귄의 퍽이 매번 미묘하게 달라진다 — 들리는 차이는 아니지만
 * "같은 시드에 같은 결과"를 소리에서만 어길 이유가 없다.
 */
const noiseBuffer = (ctx: BaseAudioContext, seconds: number): AudioBuffer => {
  const len = Math.max(1, Math.floor(ctx.sampleRate * seconds));
  const buf = ctx.createBuffer(1, len, ctx.sampleRate);
  const data = buf.getChannelData(0);
  let s = 0x9e3779b9;
  for (let i = 0; i < len; i++) {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    data[i] = ((s >>> 0) / 0xffffffff) * 2 - 1;
  }
  return buf;
};

/**
 * 게인 엔벨로프. **0에서 시작해 어택으로 올린다** — 0이 아닌 값에서 바로
 * 시작하면 클릭 노이즈("틱")가 나고, 그게 실제로는 가장 거슬리는 소리다 (KTD9).
 */
const envelope = (
  gain: GainNode,
  t0: number,
  peak: number,
  attack: number,
  decay: number,
): void => {
  gain.gain.setValueAtTime(0, t0);
  gain.gain.linearRampToValueAtTime(peak, t0 + attack);
  gain.gain.exponentialRampToValueAtTime(0.001, t0 + attack + decay);
};

/** 퍽 — 노이즈 버스트(로우패스) + 저역 "툭". ≈70ms. */
export const playWhack = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  const noise = ctx.createBufferSource();
  noise.buffer = noiseBuffer(ctx, 0.08);
  const lp = ctx.createBiquadFilter();
  lp.type = "lowpass";
  lp.frequency.value = 1200 * r;
  const ng = ctx.createGain();
  envelope(ng, t0, 0.9, 0.005, 0.055);
  noise.connect(lp);
  lp.connect(ng);
  ng.connect(out);
  noise.start(t0);
  noise.stop(t0 + 0.08);

  const thump = ctx.createOscillator();
  thump.type = "sine";
  thump.frequency.setValueAtTime(90 * r, t0);
  thump.frequency.exponentialRampToValueAtTime(55 * r, t0 + 0.07);
  const tg = ctx.createGain();
  envelope(tg, t0, 0.8, 0.005, 0.06);
  thump.connect(tg);
  tg.connect(out);
  thump.start(t0);
  thump.stop(t0 + 0.08);
};

/** 휙 — 노이즈를 밴드패스에 넣고 중심주파수를 위로 훑는다. ≈180ms. */
export const playWhoosh = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  const noise = ctx.createBufferSource();
  noise.buffer = noiseBuffer(ctx, 0.2);
  const bp = ctx.createBiquadFilter();
  bp.type = "bandpass";
  bp.Q.value = 1.2;
  bp.frequency.setValueAtTime(400 * r, t0);
  bp.frequency.exponentialRampToValueAtTime(2200 * r, t0 + 0.18);
  const g = ctx.createGain();
  envelope(g, t0, 0.55, 0.02, 0.16);
  noise.connect(bp);
  bp.connect(g);
  g.connect(out);
  noise.start(t0);
  noise.stop(t0 + 0.2);
};

/**
 * 빽 한 발의 공통 골격 — 톱니파에 포먼트 필터 둘을 병렬로 물리고 빠른
 * 비브라토를 건다. 광란이 이걸 짧게 줄여 연달아 쓴다.
 */
const squawkBurst = (
  ctx: BaseAudioContext,
  out: AudioNode,
  t0: number,
  baseHz: number,
  dur: number,
  peak: number,
): void => {
  const saw = ctx.createOscillator();
  saw.type = "sawtooth";
  saw.frequency.setValueAtTime(baseHz, t0);
  // 끝에서 살짝 내려앉는다 — 새 울음의 "꺾임"이 여기서 나온다
  saw.frequency.setTargetAtTime(baseHz * 0.8, t0 + dur * 0.6, dur * 0.25);

  // 빠른 비브라토 — 진폭이 아니라 음높이를 흔들어야 "생물"로 들린다
  const vib = ctx.createOscillator();
  vib.type = "sine";
  vib.frequency.value = 28;
  const vibGain = ctx.createGain();
  vibGain.gain.value = baseHz * 0.06;
  vib.connect(vibGain);
  vibGain.connect(saw.frequency);

  const g = ctx.createGain();
  envelope(g, t0, peak, 0.008, dur - 0.008);

  // 포먼트 둘 — 이게 없으면 그냥 버저다
  for (const [freq, q] of [
    [1500, 5],
    [2900, 7],
  ]) {
    const f = ctx.createBiquadFilter();
    f.type = "bandpass";
    f.frequency.value = freq;
    f.Q.value = q;
    saw.connect(f);
    f.connect(g);
  }
  g.connect(out);
  saw.start(t0);
  saw.stop(t0 + dur);
  vib.start(t0);
  vib.stop(t0 + dur);
};

/** 빽! — 화난 한 발. ≈350ms. */
export const playSquawk = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  squawkBurst(ctx, out, ctx.currentTime, 750 * ratio(semitones), 0.35, 1.6);
};

/**
 * 첨벙 — 물고기를 꺼내는 물소리. 노이즈 스플래시 + 물방울 하나. ≈350ms.
 *
 * 처음엔 사인 "퐁"이었는데 물이 아니라 알림음으로 들렸다(사용자 피드백,
 * 2026-09-01) — 물소리의 정체는 음이 아니라 **노이즈**다. 퍽과 같은 노이즈
 * 버퍼를 밴드패스로 물길처럼 쓸어내린다.
 */
export const playCatch = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  // 첨벙 — 노이즈를 밴드패스에 넣고 중심주파수를 아래로 쓸어내린다
  // (수면을 뚫고 나오는 순간 → 물이 가라앉는 순간)
  const splash = ctx.createBufferSource();
  splash.buffer = noiseBuffer(ctx, 0.3);
  const bp = ctx.createBiquadFilter();
  bp.type = "bandpass";
  bp.Q.value = 0.9;
  bp.frequency.setValueAtTime(2600 * r, t0);
  bp.frequency.exponentialRampToValueAtTime(700 * r, t0 + 0.26);
  const sg = ctx.createGain();
  envelope(sg, t0, 1.1, 0.01, 0.26);
  splash.connect(bp);
  bp.connect(sg);
  sg.connect(out);
  splash.start(t0);
  splash.stop(t0 + 0.3);

  // 물방울 "뽁" — 떨어진 물이 튀어 오른다. 물방울은 내려가는 음이 아니라
  // **올라가는 음**이다 (기포가 좁아지며 음이 솟는다)
  const drip = ctx.createOscillator();
  drip.type = "sine";
  drip.frequency.setValueAtTime(420 * r, t0 + 0.22);
  drip.frequency.exponentialRampToValueAtTime(1150 * r, t0 + 0.3);
  const dg = ctx.createGain();
  envelope(dg, t0 + 0.22, 0.9, 0.008, 0.11);
  drip.connect(dg);
  dg.connect(out);
  drip.start(t0 + 0.22);
  drip.stop(t0 + 0.36);
};

/** 광란 — 빽을 짧게 줄여 여섯 발, 음높이를 계단식으로 올리며. ≈700ms. */
export const playFreakout = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;
  for (let i = 0; i < 6; i++) {
    squawkBurst(ctx, out, t0 + i * 0.115, 750 * r * ratio(i), 0.09, 1.3);
  }
};
