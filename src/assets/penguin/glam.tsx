/** 화장 — 속눈썹 · 볼터치 · 입술.
 *
 * **미녀 펭귄(챔피언에게 벨트를 채워 주는 배우)만 쓴다.** 훌라 차림(`hula.tsx`)과는
 * 별개다: 저쪽은 비치발리볼 판의 **옷**이고 이쪽은 **얼굴**이라, 야차에 훌라 치마가
 * 나오면 두 판이 섞인다. 몸은 `<Penguin female />`을 그대로 쓴다.
 *
 * 패턴은 `hula.tsx` 그대로 — 레이어 하나, 기본 `display: none`, 켜는 것은 CSS다.
 */

import { BLUSH, INK, LIP } from "../palette";

export function Glam() {
  return (
    <g className="pg-glam">
      {/* 위로 말린 속눈썹 셋. 눈 바깥쪽에서 뻗는다. */}
      <path
        className="pg-lash"
        d="M60.5 24.4 C62.6 22.8 63.6 22.4 64.6 22.6
           M61.4 26.4 C63.8 25.6 64.9 25.6 65.8 26.0
           M61.2 28.6 C63.4 28.6 64.4 28.9 65.2 29.5"
        stroke={INK}
        strokeWidth="1.3"
        strokeLinecap="round"
        fill="none"
      />
      {/* 볼터치. */}
      <ellipse
        className="pg-blush"
        cx="53"
        cy="34"
        rx="4.2"
        ry="2.6"
        fill={BLUSH}
        opacity="0.75"
      />
      {/* 부리 아래쪽에 얹는 립스틱. */}
      <path
        className="pg-lip"
        d="M64.6 35.4 C68.6 35.0 72.2 34.6 75.4 33.8"
        stroke={LIP}
        strokeWidth="2.4"
        strokeLinecap="round"
        fill="none"
      />
    </g>
  );
}
