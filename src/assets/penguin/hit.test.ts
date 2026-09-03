import { describe, expect, it } from "vitest";
import {
  PENGUIN_HIT_HYSTERESIS_RATIO,
  hitBoxInWindow,
  requestBoxInWindow,
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

  it("무대_크기에_비례한다", () => {
    const [l, t, r, b] = hitBoxInWindow({ size: 280, padX: 0, padTop: 0 });
    const [l1, t1, r1, b1] = hitBoxInWindow({ size: 140, padX: 0, padTop: 0 });
    expect([l, t, r, b]).toEqual([l1 * 2, t1 * 2, r1 * 2, b1 * 2]);
  });

  it("요청_상자가_히트_상자를_포함한다", () => {
    // 방향이 뒤집히면 그 사이 띠에서 요청과 되돌리기가 매 틱 번갈아 일어난다.
    const [hl, ht, hr, hb] = hitBoxInWindow(M);
    const [rl, rt, rr, rb] = requestBoxInWindow(M);
    expect(rl).toBeLessThan(hl);
    expect(rt).toBeLessThan(ht);
    expect(rr).toBeGreaterThan(hr);
    expect(rb).toBeGreaterThan(hb);
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
    const [hl] = hitBoxInWindow(M);
    const band = hl - (M.size * PENGUIN_HIT_HYSTERESIS_RATIO) / 2;
    expect(shouldClickThrough(band, CENTER[1], M)).toBe(false);
  });
});
