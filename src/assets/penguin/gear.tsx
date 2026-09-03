/** 펭귄이 드는 것 — 방망이 · 낚싯대 · 낚싯줄 · 찌 · 물고기.
 *
 * **얼음구멍(`pg-hole`)은 여기 없다** — `body.tsx`의 `Ground()`에 있다.
 * `.pg-all` **밖**이어야 착지 포즈가 몸을 누를 때 바닥에 깔린 구멍까지 같이
 * 눌리지 않기 때문이고, 그래서 낚시 그림만 이음매를 가로지른다. 낚시를
 * 손보러 왔다면 두 파일을 다 열어야 한다.
 *
 * **감추는 방식이 둘이다.** 낚시 장비(`pg-rod`·`pg-line`·`pg-float`·`pg-fish`)는
 * `display: none`이고 `pet-css.test.ts`의 `평소 숨기는 도형`이 그걸 지킨다.
 * **방망이(`pg-bat`)는 예외로 `opacity: 0`이다** (`react.css`) — 스윙이
 * 0 → 1로 드러내는 연출이라 `display`로는 안 된다. 그래서 그 검사 목록에도
 * 없다. 일관성을 맞추겠다고 `display: none`으로 바꾸면 스윙이 죽는데
 * 두 러너는 전부 통과한다.
 */

import { BAT_GRIP, BAT_WOOD, FISH, FLOAT, INK } from "../palette";

export function Gear() {
  return (
    <>
      <g className="pg-bat">
        <path
          d="M69.2 85.8 L71.9 85.2 L76.2 122 C76.7 126.6 74.7 129.2 72.6 129.2 C70.5 129.2 68.7 126.6 69.2 122 Z"
          fill={BAT_WOOD}
        />
        <rect x="68.4" y="82.4" width="3.9" height="6.4" rx="1.7" fill={BAT_GRIP} />
      </g>

      <path
        className="pg-rod"
        d="M70 87 L98 95"
        stroke={BAT_WOOD}
        strokeWidth="2.6"
        strokeLinecap="round"
        fill="none"
      />
      <path
        className="pg-line"
        d="M98 95 L90 116"
        stroke={INK}
        strokeWidth="0.9"
        fill="none"
        opacity="0.55"
      />
      <circle className="pg-float" cx="90" cy="117" r="2.8" fill={FLOAT} />
      <g className="pg-fish">
        <ellipse cx="85" cy="107" rx="7" ry="3.6" fill={FISH} />
        <path d="M92 107 L97 103.5 L97 110.5 Z" fill={FISH} />
        <circle cx="81" cy="106" r="0.9" fill={INK} />
      </g>
    </>
  );
}
