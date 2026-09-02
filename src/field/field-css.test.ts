import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * 판(`field.css`)과 펭귄(`pet.css`)의 방망이가 **같은 그림**인지 대조한다.
 *
 * 판은 펭귄 창보다 아래에 깔리므로 커서는 두 창을 오간다 — 그림이 갈라지면
 * 펭귄에 커서를 올릴 때마다 방망이가 바뀌어 창 경계가 눈에 보인다. 상수를
 * 공유하려면 CSS를 JS에서 주입해야 하는데, 그건 "커서 한 줄"인 이 페이지를
 * 복잡하게 만든다. 그래서 중복을 두고 **대조로 막는다.**
 */
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
    // 누르면 위로, 떼면 아래로. 두 프레임이 같으면 휘두르는 게 안 보인다
    const [a, b] = bats(field);
    expect(a).not.toBe(b);
    expect(field).toMatch(/#field-root:active/);
    expect(pet).toMatch(/\.pg-pinball:active/);
  });

  it("판은_배경을_칠하지_않는다", () => {
    // 조금이라도 칠하면 화면 전체에 막이 씌워진 것처럼 보인다
    expect(field).toMatch(/background:\s*transparent/);
    expect(field).not.toMatch(/background(-color)?:\s*(#|rgb|hsl)/);
  });
});
