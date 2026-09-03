/** 화장 — 볼터치 · 속눈썹 · 립스틱. **세 가지뿐이다.**
 *
 * **미녀 펭귄(챔피언에게 벨트를 채워 주는 배우)만 쓴다.** 훌라 차림(`hula.tsx`)과는
 * 별개다: 저쪽은 비치발리볼 판의 **옷**이고 이쪽은 **얼굴**이라, 야차에 훌라 치마가
 * 나오면 두 판이 섞인다. 몸은 `<Penguin female />`을 그대로 쓴다.
 *
 * 좌표는 아티팩트 v7에서 그대로 옮겼다 — 눈·부리 자리에 맞춰 잡은 값이라
 * 눈대중으로 바꾸면 얼굴이 어긋난다.
 */

import { BLUSH, INK, LIP } from "../palette";

export function Glam() {
  return (
    <g className="pg-glam">
      {/* 볼터치 */}
      <ellipse cx="43.5" cy="34.5" rx="4.6" ry="3" fill={BLUSH} opacity="0.42" />
      {/* 속눈썹 셋 — 눈 바깥쪽에서 위로 뻗는다 */}
      <path
        className="pg-lash"
        d="M53.4 22.6 L51.4 20.4 M56.4 21.2 L55.6 18.6 M59.4 21.6 L59.6 18.9"
        stroke={INK}
        strokeWidth="1.3"
        strokeLinecap="round"
        fill="none"
      />
      {/* 립스틱 — 아랫부리를 덮는다 */}
      <path className="pg-lip" d="M64 32.4 L74.4 33 L64 35.4 Z" fill={LIP} />
    </g>
  );
}
