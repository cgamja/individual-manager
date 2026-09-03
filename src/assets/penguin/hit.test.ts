import { describe, expect, it } from "vitest";
import {
  PENGUIN_HIT_ARM_RATIO,
  PENGUIN_HIT_HYSTERESIS_RATIO,
  hitBoxInWindow,
  requestBoxInWindow,
  revertBoxInWindow,
  shouldClickThrough,
  type StageMetrics,
} from "./hit";

/** 지금 CSS가 쓰는 값 (`pet/css/base.css`). 여기 손으로 적는 이유는 판정이
 * 이 셋에만 의존한다는 것을 테스트가 드러내야 하기 때문이다 — 값이 바뀌면
 * `pet-css.test.ts`의 "창 여백 상수 동기화"가 따로 잡는다. */
const M: StageMetrics = { size: 140, padX: 52, padTop: 80 };
const WINDOW = { w: M.size + M.padX * 2, h: M.size + M.padTop };

/** 무대 한가운데 — 반드시 펭귄이다. */
const CENTER: [number, number] = [M.padX + M.size / 2, M.padTop + M.size / 2];

describe("히트 상자", () => {
  it("창_안에_들어간다", () => {
    const [l, t, r, b] = hitBoxInWindow(M);
    expect(l).toBeGreaterThanOrEqual(0);
    expect(t).toBeGreaterThanOrEqual(0);
    expect(r).toBeLessThanOrEqual(WINDOW.w);
    expect(b).toBeLessThanOrEqual(WINDOW.h);
  });

  it("창_면적의_4분의_1보다_작다", () => {
    const [l, t, r, b] = hitBoxInWindow(M);
    const ratio = ((r - l) * (b - t)) / (WINDOW.w * WINDOW.h);
    expect(ratio, `창의 ${(ratio * 100).toFixed(1)}%다`).toBeLessThan(0.25);
  });

  it("클릭을_먹는_영역이_창의_절반을_넘지_않는다", () => {
    // 사용자가 실제로 겪는 수는 **요청 상자**다 — 그 밖에서만 클릭이 통과한다.
    // 여유(ARM)와 띠를 늘리면 이 수가 100%로 돌아가고 작업의 목적이 사라진다.
    const [l, t, r, b] = requestBoxInWindow(M);
    const ratio = ((r - l) * (b - t)) / (WINDOW.w * WINDOW.h);
    expect(ratio, `창의 ${(ratio * 100).toFixed(1)}%를 먹는다`).toBeLessThan(0.5);
  });

  it("무대_크기에_비례한다", () => {
    const [l, t, r, b] = hitBoxInWindow({ size: 280, padX: 0, padTop: 0 });
    const [l1, t1, r1, b1] = hitBoxInWindow({ size: 140, padX: 0, padTop: 0 });
    expect([l, t, r, b]).toEqual([l1 * 2, t1 * 2, r1 * 2, b1 * 2]);
  });

  it("세_상자가_안에서_바깥_순서다", () => {
    // 히트 ⊂ 되돌리기 ⊂ 요청. 뒤집히면 경계에서 요청과 되돌리기가 번갈아
    // 일어나거나, 그림에 닿고 나서야 되돌린다.
    const [hl, ht, hr, hb] = hitBoxInWindow(M);
    const [vl, vt, vr, vb] = revertBoxInWindow(M);
    const [ql, qt, qr, qb] = requestBoxInWindow(M);
    expect(ql).toBeLessThan(vl);
    expect(vl).toBeLessThan(hl);
    expect(qt).toBeLessThan(vt);
    expect(vt).toBeLessThan(ht);
    expect(qr).toBeGreaterThan(vr);
    expect(vr).toBeGreaterThan(hr);
    expect(qb).toBeGreaterThan(vb);
    expect(vb).toBeGreaterThan(hb);
  });
});

describe("통과 요청 판정", () => {
  it("펭귄_가운데는_요청하지_않는다", () => {
    expect(shouldClickThrough(CENTER[0], CENTER[1], M)).toBe(false);
  });

  it("방망이_여백은_요청한다", () => {
    expect(shouldClickThrough(10, CENTER[1], M)).toBe(true);
  });

  it("말풍선_자리는_요청한다", () => {
    expect(shouldClickThrough(CENTER[0], 10, M)).toBe(true);
  });

  it("무대_안이어도_레터박스는_요청한다", () => {
    // 무대 왼쪽 끝 5px — 그림이 시작하지도 않은 자리다.
    expect(shouldClickThrough(M.padX + 5, CENTER[1], M)).toBe(true);
  });

  it("히스테리시스_띠_안에서는_요청하지_않는다", () => {
    // 되돌리기 상자와 요청 상자 사이 — Rust는 되돌리지 않고 웹뷰는 요청하지
    // 않는 구간이다. 여기가 비면 경계 위에서 둘이 번갈아 일어난다.
    const [vl] = revertBoxInWindow(M);
    const band = vl - (M.size * PENGUIN_HIT_HYSTERESIS_RATIO) / 2;
    expect(shouldClickThrough(band, CENTER[1], M)).toBe(false);
  });

  it("그림에_닿기_전에_요청을_멈춘다", () => {
    // 요청을 그림 경계까지 끌고 가면, 커서가 올라온 뒤 한 틱 + 세터 한 프레임
    // 동안 클릭이 아래 앱으로 샌다.
    const [hl] = hitBoxInWindow(M);
    const 코앞 = hl - (M.size * PENGUIN_HIT_ARM_RATIO) / 2;
    expect(shouldClickThrough(코앞, CENTER[1], M)).toBe(false);
  });
});
