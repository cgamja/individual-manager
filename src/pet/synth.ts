/** 소리 일곱을 그 자리에서 합성한다 (KTD1 — Q9 확정). */

/** 반음 오프셋 → 주파수 배율. 마리마다 목소리를 다르게 만드는 손잡이다. */
const ratio = (semitones: number): number => Math.pow(2, semitones / 12);

/** 결정적 화이트노이즈 버퍼 (xorshift32, 고정 시드). `Math.random()`으로 */
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

/** 게인 엔벨로프. **0에서 시작해 어택으로 올린다** — 0이 아닌 값에서 바로 */
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

/** 빽 한 발의 공통 골격 — 톱니파에 포먼트 필터 둘을 병렬로 물리고 빠른 */
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
  saw.frequency.setTargetAtTime(baseHz * 0.8, t0 + dur * 0.6, dur * 0.25);

  const vib = ctx.createOscillator();
  vib.type = "sine";
  vib.frequency.value = 28;
  const vibGain = ctx.createGain();
  vibGain.gain.value = baseHz * 0.06;
  vib.connect(vibGain);
  vibGain.connect(saw.frequency);

  const g = ctx.createGain();
  envelope(g, t0, peak, 0.008, dur - 0.008);

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

/** 첨벙 — 물고기를 꺼내는 물소리. 꾸르륵대는 노이즈 한 겹. ≈480ms. */
export const playCatch = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  const body = ctx.createBufferSource();
  body.buffer = noiseBuffer(ctx, 0.5);
  const bp = ctx.createBiquadFilter();
  bp.type = "bandpass";
  bp.Q.value = 0.9;
  bp.frequency.setValueAtTime(1200 * r, t0);
  bp.frequency.exponentialRampToValueAtTime(400 * r, t0 + 0.44);
  const wob = ctx.createOscillator();
  wob.type = "sine";
  wob.frequency.value = 10;
  const wobGain = ctx.createGain();
  wobGain.gain.value = 170 * r;
  wob.connect(wobGain);
  wobGain.connect(bp.frequency);
  const bg = ctx.createGain();
  envelope(bg, t0, 0.85, 0.07, 0.4);
  body.connect(bp);
  bp.connect(bg);
  bg.connect(out);
  body.start(t0);
  body.stop(t0 + 0.5);
  wob.start(t0);
  wob.stop(t0 + 0.5);

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

/** 딱 — 볼링공에 핀이 맞는 소리. 나무 두 개가 부딪히는 짧고 단단한 어택.
 * 퍽(`playWhack`)보다 **높고 짧다** — 살점이 아니라 나무여야 한다. ≈90ms. */
export const playStrike = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  // 딱 하는 어택 — 짧은 노이즈를 하이패스로 올려 나무 결을 만든다.
  const click = ctx.createBufferSource();
  click.buffer = noiseBuffer(ctx, 0.05);
  const hp = ctx.createBiquadFilter();
  hp.type = "highpass";
  hp.frequency.value = 1600 * r;
  const cg = ctx.createGain();
  envelope(cg, t0, 0.9, 0.002, 0.035);
  click.connect(hp);
  hp.connect(cg);
  cg.connect(out);
  click.start(t0);
  click.stop(t0 + 0.05);

  // 통 — 핀 몸통이 울리는 음정. 두 배음을 살짝 어긋나게 겹쳐 나무처럼 만든다.
  for (const [hz, peak] of [
    [420, 0.7],
    [631, 0.35],
  ]) {
    const body = ctx.createOscillator();
    body.type = "triangle";
    body.frequency.setValueAtTime(hz * r, t0);
    body.frequency.exponentialRampToValueAtTime(hz * r * 0.86, t0 + 0.085);
    const g = ctx.createGain();
    envelope(g, t0, peak, 0.003, 0.08);
    body.connect(g);
    g.connect(out);
    body.start(t0);
    body.stop(t0 + 0.09);
  }
};

/** 드르륵 — 공이 굴러가기 시작하는 소리. 낮은 노이즈를 로우패스에 넣고
 * 천천히 죽인다. **한 발로 끝낸다** — 굴러가는 내내 이어지는 소리는 상주 앱이
 * 낼 만한 것이 아니고, 이 앱의 소리 장치는 전부 한 발짜리다. ≈700ms. */
export const playRoll = (
  ctx: BaseAudioContext,
  out: AudioNode,
  semitones: number,
): void => {
  const r = ratio(semitones);
  const t0 = ctx.currentTime;

  const rumble = ctx.createBufferSource();
  rumble.buffer = noiseBuffer(ctx, 0.75);
  const lp = ctx.createBiquadFilter();
  lp.type = "lowpass";
  lp.frequency.setValueAtTime(320 * r, t0);
  lp.frequency.exponentialRampToValueAtTime(140 * r, t0 + 0.7);
  lp.Q.value = 3;
  // 나무 바닥의 결 — 느린 흔들림을 얹어야 "구른다"로 읽힌다.
  const wob = ctx.createOscillator();
  wob.type = "sine";
  wob.frequency.value = 17;
  const wobGain = ctx.createGain();
  wobGain.gain.value = 60 * r;
  wob.connect(wobGain);
  wobGain.connect(lp.frequency);
  const g = ctx.createGain();
  envelope(g, t0, 0.8, 0.03, 0.66);
  rumble.connect(lp);
  lp.connect(g);
  g.connect(out);
  rumble.start(t0);
  rumble.stop(t0 + 0.75);
  wob.start(t0);
  wob.stop(t0 + 0.75);
};
