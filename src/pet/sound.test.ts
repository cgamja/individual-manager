import { describe, expect, it } from "vitest";
import type { Behavior, PetSnapshot } from "../lib/pet";
import {
  SOUND_COOLDOWN_MS,
  SoundPlayer,
  passesCooldown,
  soundsFor,
  voiceOffsetFor,
  type SoundName,
} from "./sound";

/** 스냅샷 하나를 빠르게 만든다 — 소리 판정에 안 쓰는 필드는 아무 값이어도 된다. */
const snap = (over: Partial<PetSnapshot> = {}): PetSnapshot => ({
  x: 0,
  y: 0,
  facing: "right",
  vertical: "level",
  air: false,
  speech: null,
  whack_seq: 0,
  pinball: false,
  behavior: { kind: "walk" },
  ...over,
});

describe("soundsFor — 전이 검출", () => {
  it("빠따를_맞으면_퍽이_난다", () => {
    const prev = snap({ whack_seq: 3, behavior: { kind: "swing" } });
    const next = snap({ whack_seq: 4, behavior: { kind: "swing" } });
    expect(soundsFor(prev, next)).toContain("whack");
  });

  it("빠따_횟수가_그대로면_소리가_없다", () => {
    // 말풍선만 바뀐 스냅샷 — 동작도 횟수도 그대로다
    const prev = snap({ whack_seq: 2 });
    const next = snap({ whack_seq: 2, speech: { seq: 1, roll: 7 } });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("빽빽거리는_중에_또_맞아도_퍽이_난다", () => {
    // whack_seq가 늘면 동작이 squawk 그대로여도 퍽이다 (KTD7의 핵심 근거)
    const prev = snap({ whack_seq: 20, behavior: { kind: "squawk" } });
    const next = snap({ whack_seq: 21, behavior: { kind: "squawk" } });
    expect(soundsFor(prev, next)).toContain("whack");
  });

  it("핀볼_타격에는_퍽이_없다", () => {
    // 채로 치면 whack_seq는 그대로고 동작만 thrown이 된다 (AE6)
    const prev = snap({ whack_seq: 5, pinball: true, behavior: { kind: "walk" } });
    const next = snap({ whack_seq: 5, pinball: true, behavior: { kind: "thrown" } });
    expect(soundsFor(prev, next)).toEqual(["whoosh"]);
  });

  it("던지면_휙이_난다", () => {
    const prev = snap({ behavior: { kind: "dragged" } });
    const next = snap({ behavior: { kind: "thrown" }, air: true });
    expect(soundsFor(prev, next)).toContain("whoosh");
  });

  it("던지는_중에는_다시_안_난다", () => {
    const prev = snap({ behavior: { kind: "thrown" }, air: true });
    const next = snap({ behavior: { kind: "thrown" }, air: true, speech: { seq: 1, roll: 0 } });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("빽빽거리기가_새로_시작되면_빽이_난다", () => {
    const prev = snap({ behavior: { kind: "dragged" } });
    const next = snap({ behavior: { kind: "squawk" } });
    expect(soundsFor(prev, next)).toContain("squawk");
  });

  it("발작은_돌진에서만_소리가_난다", () => {
    const walk = snap({ behavior: { kind: "walk" } });
    const dash = snap({ behavior: { kind: "freakout", freakout: "dash" }, air: true });
    const pant = snap({ behavior: { kind: "freakout", freakout: "pant" } });
    expect(soundsFor(walk, dash)).toEqual(["freakout"]);
    // 숨 고르기 진입은 무음 — 광란은 돌진의 것이다
    expect(soundsFor(dash, pant)).toEqual([]);
  });

  it("걷기_헤엄_착지_졸기에는_소리가_없다", () => {
    // R7 회귀 가드 — 저절로 나오는 전 국면이 전부 무음임을 표로 돈다.
    // 나중에 소리를 하나 더 붙일 때 "왜 안 붙였는지"를 코드가 기억하는 자리다
    const silent: Behavior[] = [
      { kind: "walk" },
      { kind: "turn" },
      { kind: "idle", idle: "look_around" },
      { kind: "idle", idle: "stretch" },
      { kind: "idle", idle: "shake" },
      { kind: "idle", idle: "shift_feet" },
      { kind: "swim" },
      { kind: "sleep" },
      { kind: "falling" },
      { kind: "land" },
      { kind: "splat" },
      { kind: "sprawl" },
      { kind: "tumble" },
      { kind: "slide" },
      { kind: "freakout", freakout: "pant" },
      { kind: "ice_fishing", fishing: "dig" },
      { kind: "ice_fishing", fishing: "wait" },
      { kind: "ice_fishing", fishing: "bite" },
      { kind: "ice_fishing", fishing: "catch" },
      { kind: "ice_fishing", fishing: "miss" },
      { kind: "ice_fishing", fishing: "pack" },
    ];
    const prev = snap({ behavior: { kind: "walk" } });
    for (const behavior of silent) {
      expect(soundsFor(prev, snap({ behavior })), JSON.stringify(behavior)).toEqual([]);
    }
  });

  it("첫_스냅샷에는_소리가_없다", () => {
    // 앱을 켜자마자 마침 빽빽거리는 중이었다고 소리를 내면 원인 없는 소리다
    expect(soundsFor(null, snap({ behavior: { kind: "squawk" }, whack_seq: 9 }))).toEqual([]);
  });
});

describe("passesCooldown — 소리별 최소 간격", () => {
  it("쿨다운_안에_다시_요청하면_버린다", () => {
    const names: SoundName[] = ["whack", "whoosh", "squawk", "freakout"];
    for (const name of names) {
      expect(passesCooldown(name, 1000, 1000 + SOUND_COOLDOWN_MS[name] - 1)).toBe(false);
    }
  });

  it("쿨다운이_지나면_다시_난다", () => {
    const names: SoundName[] = ["whack", "whoosh", "squawk", "freakout"];
    for (const name of names) {
      expect(passesCooldown(name, 1000, 1000 + SOUND_COOLDOWN_MS[name])).toBe(true);
    }
  });

  it("처음_내는_소리는_바로_난다", () => {
    expect(passesCooldown("whack", undefined, 0)).toBe(true);
  });
});

/** 소리 그래프의 최소 표면만 흉내 낸 스텁 — 만든 노드의 종류를 기록한다. */
const stubContext = () => {
  const created: string[] = [];
  const param = {
    value: 0,
    setValueAtTime() {},
    linearRampToValueAtTime() {},
    exponentialRampToValueAtTime() {},
    setTargetAtTime() {},
  };
  const node = () => ({
    connect(target: unknown) {
      return target;
    },
    disconnect() {},
    start() {},
    stop() {},
    gain: { ...param },
    frequency: { ...param },
    detune: { ...param },
    Q: { ...param },
    buffer: null as unknown,
    type: "",
  });
  const ctx = {
    created,
    state: "running" as AudioContextState,
    sampleRate: 48000,
    currentTime: 0,
    destination: node(),
    resume: () => Promise.resolve(),
    close: () => Promise.resolve(),
    createGain() {
      created.push("gain");
      return node();
    },
    createOscillator() {
      created.push("oscillator");
      return node();
    },
    createBiquadFilter() {
      created.push("filter");
      return node();
    },
    createBufferSource() {
      created.push("source");
      return node();
    },
    createBuffer(_ch: number, len: number) {
      created.push("buffer");
      return { getChannelData: () => new Float32Array(len) };
    },
  };
  return ctx as typeof ctx & AudioContext;
};

describe("SoundPlayer", () => {
  it("오디오가_없는_환경에서도_터지지_않는다", () => {
    // jsdom에는 AudioContext가 없다 — 소리 없는 무해한 상태로 남는다 (R11)
    const player = new SoundPlayer("pet-1");
    player.setEnabled(true);
    expect(() => player.play("whack", 0)).not.toThrow();
    expect(() => player.nudge()).not.toThrow();
    expect(() => player.close()).not.toThrow();
  });

  it("꺼져_있으면_아무것도_재생하지_않는다", () => {
    const ctx = stubContext();
    const player = new SoundPlayer("pet-1", () => ctx);
    // 기본은 꺼짐이다 (PRD Q6) — setEnabled(true) 없이 재생을 시도한다
    player.play("squawk", 0);
    expect(ctx.created.filter((k) => k !== "gain")).toEqual([]);
  });

  it("켜져_있으면_소리_그래프를_만든다", () => {
    const ctx = stubContext();
    const player = new SoundPlayer("pet-1", () => ctx);
    player.setEnabled(true);
    player.play("squawk", 0);
    expect(ctx.created.filter((k) => k === "oscillator").length).toBeGreaterThan(0);
  });

  it("쿨다운_안의_재생은_버린다", () => {
    const ctx = stubContext();
    const player = new SoundPlayer("pet-1", () => ctx);
    player.setEnabled(true);
    player.play("squawk", 1000);
    const after = ctx.created.length;
    player.play("squawk", 1000 + SOUND_COOLDOWN_MS.squawk - 1);
    expect(ctx.created.length).toBe(after);
  });

  it("컨텍스트가_잠겨_있으면_버리고_깨우기만_시도한다", () => {
    const ctx = stubContext();
    let resumed = 0;
    ctx.state = "suspended";
    ctx.resume = () => {
      resumed += 1;
      return Promise.resolve();
    };
    const player = new SoundPlayer("pet-1", () => ctx);
    player.setEnabled(true);
    player.play("whack", 0);
    // 그 소리는 버려진다 — 3초 뒤의 퍽은 없느니만 못하다 (KTD4)
    expect(ctx.created.filter((k) => k !== "gain")).toEqual([]);
    expect(resumed).toBe(1);
  });
});

describe("voiceOffsetFor — 마리별 목소리", () => {
  it("같은_라벨은_늘_같은_음높이다", () => {
    // PRINCIPLE 3 — 같은 시드에 같은 결과. 껐다 켜도 그 펭귄의 목소리다
    expect(voiceOffsetFor("pet-3")).toBe(voiceOffsetFor("pet-3"));
  });

  it("다른_펭귄은_음높이가_다르다", () => {
    // 상한이 8마리라 라벨 여덟 개가 전부 서로 달라야 한다
    const offsets = new Set(
      Array.from({ length: 8 }, (_, i) => voiceOffsetFor(`pet-${i}`)),
    );
    expect(offsets.size).toBe(8);
  });

  it("이상한_라벨이면_기준음이다", () => {
    expect(voiceOffsetFor("main")).toBe(0);
    expect(voiceOffsetFor("")).toBe(0);
    expect(voiceOffsetFor("pet-")).toBe(0);
  });
});
