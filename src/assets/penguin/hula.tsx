/** 펭귄의 훌라 차림 — 치마(암수 공통) · 상의(암컷만) · 레이(암수 공통).
 *
 * 상의는 CSS가 `.pg-female`에서만 켠다. 설계 이력은 `MOTIONS.md`에 있고,
 * 결론은 `pet-css.test.ts`의 `비치발리볼` 블록이 검사로 못 박아 뒀다.
 */

import { LEI, LEI_ALT, STRAW, STRAW_DARK } from "../palette";

export function Hula() {
  return (
    <>
      {/* 훌라 차림 — 수컷은 치마+레이, 암컷은 상의까지. `female`은 창 라벨에서
          결정적으로 파생한다(`isFemalePet`) — 저장할 것이 없다.

          `.pg-all` 안이고 몸통 위, 날개 아래다. */}
      <g className="pg-luau">
        {/* ── 치마 — 암수 공통. 허리띠 + 톱니진 라피아 단 ── */}
        <g className="pg-luau-skirt">
          {/* 얇은 허리끈. 가닥이 여기 매달린다. */}
          <path
            d="M31 87 C38 82.5 56 82.5 63 87"
            stroke={STRAW}
            strokeWidth="5"
            strokeLinecap="round"
            fill="none"
          />
          {/* 가닥 치마. 한 덩어리 실루엣에 아랫단만 톱니진다 — 창이 140px이라
              가닥을 낱낱이 그리면 움직일 때 뭉갠다. */}
          <path
            d="M32 88 C39 84.5 55 84.5 62 88
               L60.5 98 L58.5 105 L57 97 L54 110 L52 98 L50 111.5
               L48 99 L46 111 L44 99 L41.5 108 L39.5 98 L37 104 L35.5 96 L33.5 97 Z"
            fill={STRAW}
          />
          {/* 가닥은 몇 개만 암시한다. */}
          <path
            d="M41 89 L40 104 M47 89.5 L47 107 M54 89 L54.5 106"
            stroke={STRAW_DARK}
            strokeWidth="1.3"
            strokeLinecap="round"
            fill="none"
            opacity="0.7"
          />
          {/* 허리선 — 경계가 보여야 옷으로 읽힌다. */}
          <path
            d="M31.5 88 C38.5 84 55.5 84 62.5 88"
            stroke={STRAW_DARK}
            strokeWidth="1.8"
            strokeLinecap="round"
            fill="none"
          />
        </g>

        {/* ── 상의 — 암컷만. 지푸라기 삼각형 둘 + 끈 ──

            **얇게 그리되 옷임을 분명히** 한다. 셋이 함께 있어야 한다:
            흰 배와 대비되는 색, 보이는 끈, 도형마다 테두리.
            **그리는 순서가 겹치는 순서다** — 등뒤 끈이 컵 위로 나와야 보인다. */}
        <g className="pg-luau-top">
          {/* 등뒤로 도는 끈. 컵 위로 나온다 — 아래면 삼각형에 완전히 덮인다. */}
          <path
            d="M32.5 58.5 C40 62.5 55 62.5 62.5 58.5"
            stroke={STRAW_DARK}
            strokeWidth="2"
            strokeLinecap="round"
            fill="none"
          />
          {/* 목뒤로 올라가는 V자 끈 */}
          <path
            d="M40 60 L47 49.5 L55 60"
            stroke={STRAW_DARK}
            strokeWidth="1.9"
            strokeLinecap="round"
            strokeLinejoin="round"
            fill="none"
          />
          {/* 삼각형 둘. 얕게 잡는다 — 깊이 16 미만을 검사가 지킨다. */}
          <path
            d="M34.5 60.5 L47 60.5 L41 72.5 Z"
            fill={STRAW}
            stroke={STRAW_DARK}
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
          <path
            d="M47 60.5 L59.5 60.5 L53.5 72.5 Z"
            fill={STRAW}
            stroke={STRAW_DARK}
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
        </g>

        {/* ── 레이 — 암수 공통. 상의보다 뒤에 그린다(목걸이는 끈 위에 걸린다) ── */}
        <g className="pg-lei">
          <path
            d="M34 52 C40 62 55 62 62 52"
            stroke={LEI}
            strokeWidth="5.5"
            strokeLinecap="round"
            fill="none"
          />
          <circle cx="39" cy="57" r="2.6" fill={LEI_ALT} />
          <circle cx="48" cy="60" r="2.8" fill={LEI_ALT} />
          <circle cx="57" cy="57" r="2.6" fill={LEI_ALT} />
        </g>

      </g>
    </>
  );
}
