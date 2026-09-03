/** 펭귄이 드는 것 — 방망이 · 낚싯대 · 낚싯줄 · 찌 · 물고기.
 *
 * 평소에는 CSS가 전부 `display: none`으로 감춘다. 동작이 켤 때만 보인다
 * (`pet-css.test.ts`의 `평소 숨기는 도형`이 그걸 지킨다).
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
