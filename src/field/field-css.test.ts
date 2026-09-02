import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

/** 판(`field.css`)과 펭귄(`pet.css`)의 방망이가 **같은 그림**인지 대조한다. */
const field = readFileSync(resolve("src/field/field.css"), "utf8");
const pet = readFileSync(resolve("src/pet/css/pinball.css"), "utf8");

/** CSS에서 `url("data:image/svg+xml,...")` 안의 그림들을 뽑는다. */
function bats(css: string): string[] {
  return [...css.matchAll(/url\("(data:image\/svg\+xml,[^"]+)"\)/g)].map((m) => m[1]);
}

describe("핀볼 판", () => {
  it("판과_펭귄의_방망이가_같은_그림이다", () => {
    const f = bats(field);
    const p = bats(pet);
    expect(f.length).toBe(2);
    expect(new Set(f)).toEqual(new Set(p));
  });

  it("휘두르는_프레임이_따로_있다", () => {
    const [a, b] = bats(field);
    expect(a).not.toBe(b);
    expect(field).toMatch(/#field-root:active/);
    expect(pet).toMatch(/\.pg-pinball:active/);
  });

  it("판은_배경을_칠하지_않는다", () => {
    expect(field).toMatch(/background:\s*transparent/);
    expect(field).not.toMatch(/background(-color)?:\s*(#|rgb|hsl)/);
  });
});
