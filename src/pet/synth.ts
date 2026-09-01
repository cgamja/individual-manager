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
 * 첨벙 — 물고기를 꺼내는 물소리. 꾸르륵대는 노이즈 한 겹. ≈400ms.
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

  // 물이 갈라지는 "촤아" — 필터를 LFO로 흔들어 꾸르륵대는 물의 질감을
  // 만든다 (고정 필터면 그냥 바람 소리다). 고역 부딪힘("촤")도 앞에 얹어
  // 봤지만 뺐다 — 이 한 겹이 제일 물답게 들렸다 (2026-09-01 사용자 피드백)
  const body = ctx.createBufferSource();
  body.buffer = noiseBuffer(ctx, 0.4);
  const bp = ctx.createBiquadFilter();
  bp.type = "bandpass";
  bp.Q.value = 1.3;
  bp.frequency.setValueAtTime(1600 * r, t0);
  bp.frequency.exponentialRampToValueAtTime(450 * r, t0 + 0.36);
  const wob = ctx.createOscillator();
  wob.type = "sine";
  wob.frequency.value = 13;
  const wobGain = ctx.createGain();
  wobGain.gain.value = 280 * r;
  wob.connect(wobGain);
  wobGain.connect(bp.frequency);
  const bg = ctx.createGain();
  envelope(bg, t0, 1.0, 0.02, 0.34);
  body.connect(bp);
  bp.connect(bg);
  bg.connect(out);
  body.start(t0);
  body.stop(t0 + 0.4);
  wob.start(t0);
  wob.stop(t0 + 0.4);

  // 물방울("뽁뽁뽁")도 얹어 봤지만 뺐다 — 물이 아니라 효과음 장식으로
  // 들렸다 (2026-09-01 사용자 피드백). 첨벙은 이 한 겹이면 된다
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
