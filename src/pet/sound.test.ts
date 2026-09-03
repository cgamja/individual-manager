import { describe, expect, it } from "vitest";
import type { Behavior, PetSnapshot } from "../lib/pet";
import {
  DEFAULT_VOLUME_STEP,
  SOUND_COOLDOWN_MS,
  SoundPlayer,
  gainForVolume,
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
  punch_seq: 0,
  punch_down: false,
  punch_blocked: false,
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
    const prev = snap({ whack_seq: 2 });
    const next = snap({ whack_seq: 2, speech: { seq: 1, roll: 7 } });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("빽빽거리는_중에_또_맞아도_퍽이_난다", () => {
    const prev = snap({ whack_seq: 20, behavior: { kind: "squawk" } });
    const next = snap({ whack_seq: 21, behavior: { kind: "squawk" } });
    expect(soundsFor(prev, next)).toContain("whack");
  });

  it("핀볼_타격에는_퍽이_없다", () => {
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
    expect(soundsFor(dash, pant)).toEqual([]);
  });

  it("걷기_헤엄_착지_졸기에는_소리가_없다", () => {
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
      { kind: "sassy", sassy: "turn_away" },
      { kind: "sassy", sassy: "head_flick" },
      { kind: "sassy", sassy: "wing_flick" },
      { kind: "sassy", sassy: "eye_roll" },
      { kind: "sassy", sassy: "butt_wiggle" },
      { kind: "ice_fishing", fishing: "dig" },
      { kind: "ice_fishing", fishing: "wait" },
      { kind: "ice_fishing", fishing: "bite" },
      { kind: "ice_fishing", fishing: "miss" },
      { kind: "ice_fishing", fishing: "pack" },
    ];
    const prev = snap({ behavior: { kind: "walk" } });
    for (const behavior of silent) {
      expect(soundsFor(prev, snap({ behavior })), JSON.stringify(behavior)).toEqual([]);
    }
  });

  it("잡으면_퐁이_난다", () => {
    const prev = snap({ behavior: { kind: "ice_fishing", fishing: "bite" } });
    const next = snap({ behavior: { kind: "ice_fishing", fishing: "catch" } });
    expect(soundsFor(prev, next)).toEqual(["catch"]);
  });

  it("다시_드리워_또_잡으면_또_난다", () => {
    const phases = ["catch", "wait", "bite", "catch"] as const;
    const sounds = phases.slice(1).map((fishing, i) =>
      soundsFor(
        snap({ behavior: { kind: "ice_fishing", fishing: phases[i] } }),
        snap({ behavior: { kind: "ice_fishing", fishing } }),
      ),
    );
    expect(sounds).toEqual([[], [], ["catch"]]);
  });

  it("잡는_국면이_이어지는_동안은_다시_안_난다", () => {
    const prev = snap({ behavior: { kind: "ice_fishing", fishing: "catch" } });
    const next = snap({
      behavior: { kind: "ice_fishing", fishing: "catch" },
      speech: { seq: 1, roll: 0 },
    });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("잡는_국면이_끝나면_조용하다", () => {
    const caught = snap({ behavior: { kind: "ice_fishing", fishing: "catch" } });
    const wait = snap({ behavior: { kind: "ice_fishing", fishing: "wait" } });
    const pack = snap({ behavior: { kind: "ice_fishing", fishing: "pack" } });
    expect(soundsFor(caught, wait)).toEqual([]);
    expect(soundsFor(caught, pack)).toEqual([]);
  });

  it("꽝은_조용하다", () => {
    const prev = snap({ behavior: { kind: "ice_fishing", fishing: "wait" } });
    const next = snap({ behavior: { kind: "ice_fishing", fishing: "miss" } });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("드리우기와_입질은_조용하다", () => {
    const dig = snap({ behavior: { kind: "ice_fishing", fishing: "dig" } });
    const wait = snap({ behavior: { kind: "ice_fishing", fishing: "wait" } });
    const bite = snap({ behavior: { kind: "ice_fishing", fishing: "bite" } });
    expect(soundsFor(dig, wait)).toEqual([]);
    expect(soundsFor(wait, bite)).toEqual([]);
  });

  it("첫_스냅샷에는_소리가_없다", () => {
    expect(soundsFor(null, snap({ behavior: { kind: "squawk" }, whack_seq: 9 }))).toEqual([]);
  });
});

describe("passesCooldown — 소리별 최소 간격", () => {
  it("쿨다운_안에_다시_요청하면_버린다", () => {
    const names: SoundName[] = ["whack", "whoosh", "squawk", "freakout", "catch"];
    for (const name of names) {
      expect(passesCooldown(name, 1000, 1000 + SOUND_COOLDOWN_MS[name] - 1)).toBe(false);
    }
  });

  it("쿨다운이_지나면_다시_난다", () => {
    const names: SoundName[] = ["whack", "whoosh", "squawk", "freakout", "catch"];
    for (const name of names) {
      expect(passesCooldown(name, 1000, 1000 + SOUND_COOLDOWN_MS[name])).toBe(true);
    }
  });

  it("처음_내는_소리는_바로_난다", () => {
    expect(passesCooldown("whack", undefined, 0)).toBe(true);
  });
});

describe("gainForVolume — 음량 단계", () => {
  it("가운데_단계가_지금_크기다", () => {
    expect(gainForVolume(DEFAULT_VOLUME_STEP)).toBeCloseTo(0.12);
  });

  it("단계마다_두_배씩_커진다", () => {
    for (let s = 0; s < 4; s++) {
      expect(gainForVolume(s + 1)).toBeCloseTo(gainForVolume(s) * 2);
    }
  });

  it("이상한_단계는_지금_크기다", () => {
    for (const bad of [-1, 5, 1.5, NaN]) {
      expect(gainForVolume(bad), String(bad)).toBeCloseTo(0.12);
    }
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
  const gains: Array<{ gain: { value: number } }> = [];
  const ctx = {
    created,
    gains,
    state: "running" as AudioContextState,
    sampleRate: 48000,
    currentTime: 0,
    destination: node(),
    resume: () => Promise.resolve(),
    close: () => Promise.resolve(),
    createGain() {
      created.push("gain");
      const n = node();
      gains.push(n);
      return n;
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
    const player = new SoundPlayer("pet-1");
    player.setEnabled(true);
    expect(() => player.play("whack", 0)).not.toThrow();
    expect(() => player.nudge()).not.toThrow();
    expect(() => player.close()).not.toThrow();
  });

  it("꺼져_있으면_아무것도_재생하지_않는다", () => {
    const ctx = stubContext();
    const player = new SoundPlayer("pet-1", () => ctx);
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

  it("모든_소리가_스텁에서_그래프를_만든다", () => {
    const names: SoundName[] = ["whack", "whoosh", "squawk", "freakout", "catch"];
    for (const name of names) {
      const ctx = stubContext();
      const player = new SoundPlayer("pet-1", () => ctx);
      player.setEnabled(true);
      player.play(name, 0);
      expect(
        ctx.created.filter((k) => k === "oscillator" || k === "source").length,
        name,
      ).toBeGreaterThan(0);
    }
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

  it("음량을_바꾸면_마스터_게인이_바뀐다", () => {
    const ctx = stubContext();
    const player = new SoundPlayer("pet-1", () => ctx);
    expect(ctx.gains[0].gain.value).toBeCloseTo(gainForVolume(DEFAULT_VOLUME_STEP));
    player.setVolume(4);
    expect(ctx.gains[0].gain.value).toBeCloseTo(gainForVolume(4));
    player.setVolume(0);
    expect(ctx.gains[0].gain.value).toBeCloseTo(gainForVolume(0));
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
    expect(ctx.created.filter((k) => k !== "gain")).toEqual([]);
    expect(resumed).toBe(1);
  });
});

describe("voiceOffsetFor — 마리별 목소리", () => {
  it("같은_라벨은_늘_같은_음높이다", () => {
    expect(voiceOffsetFor("pet-3")).toBe(voiceOffsetFor("pet-3"));
  });

  it("다른_펭귄은_음높이가_다르다", () => {
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

describe("야차의 퍽", () => {
  it("punch_seq가_늘면_퍽이_난다", () => {
    const prev = snap({ punch_seq: 3 });
    const next = snap({ punch_seq: 4 });
    expect(soundsFor(prev, next)).toContain("punch");
  });

  it("대표가_아니면_소리를_안_낸다", () => {
    // 맞기만 한 마리는 `punch_seq`가 안 오른다 — 라운드마다 딱 한 마리다.
    const prev = snap({ punch_seq: 3, behavior: { kind: "yacha", yacha: "guard" } });
    const next = snap({ punch_seq: 3, behavior: { kind: "yacha", yacha: "hurt" } });
    expect(soundsFor(prev, next)).toEqual([]);
  });

  it("같은_seq가_두_번_오면_한_번만_난다", () => {
    const a = snap({ punch_seq: 5 });
    expect(soundsFor(a, snap({ punch_seq: 5 }))).toEqual([]);
  });

  it("쓰러뜨린_한_방은_다른_소리다", () => {
    const prev = snap({ punch_seq: 1 });
    const next = snap({ punch_seq: 2, punch_down: true });
    expect(soundsFor(prev, next)).toContain("punch-down");
    expect(soundsFor(prev, next)).not.toContain("punch");
  });

  it("퍽과_쓰러뜨린_한_방은_서로를_안_거른다", () => {
    // 쿨다운은 **이름별**로 걸린다. 둘이 다른 이름이라 쓰러뜨린 라운드에
    // 평소 퍽이 방금 났어도 마무리 한 방이 안 걸러진다.
    expect(passesCooldown("punch", 0, 10)).toBe(false);
    expect(passesCooldown("punch-down", undefined, 10)).toBe(true);
  });

  it("퍽에도_쿨다운이_있다", () => {
    expect(SOUND_COOLDOWN_MS.punch).toBeGreaterThan(0);
    expect(SOUND_COOLDOWN_MS["punch-down"]).toBeGreaterThan(0);
  });
});
