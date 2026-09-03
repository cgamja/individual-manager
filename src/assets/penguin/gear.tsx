/** 펭귄이 드는 것 — 방망이 · 낚싯대 · 낚싯줄 · 찌 · 물고기 · 야차 차림.
 *
 * **얼음구멍(`pg-hole`)은 `body.tsx`의 `Ground()`에 있다** — `.pg-all` 밖이라야
 * 착지 포즈에 안 눌린다. 낚시를 손보려면 두 파일을 다 연다.
 *
 * 낚시 장비는 `display: none`, **방망이만 `opacity: 0`**으로 감춘다 —
 * 스윙이 0 → 1로 드러내는 연출이라 `display`로는 안 된다.
 */

import {
  ANGER,
  BAT_EDGE,
  BAT_GRIP,
  BAT_WOOD,
  BELT_GOLD,
  BELT_GOLD_DARK,
  FISH,
  FLOAT,
  GLOVE,
  GLOVE_DARK,
  GLOVE_LACE,
  INK,
} from "../palette";

export function Gear() {
  return (
    <>
      <g className="pg-bat">
        <path
          d="M69.2 85.8 L71.9 85.2 L76.2 122 C76.7 126.6 74.7 129.2 72.6 129.2 C70.5 129.2 68.7 126.6 69.2 122 Z"
          fill={BAT_WOOD}
          stroke={BAT_EDGE}
          strokeWidth="0.9"
          strokeLinejoin="round"
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

      <BoxingGloves />
      <ChampionBelt />
      <AngryMark />
    </>
  );
}

/** 복싱 장갑 — 날개 끝 둘에 끼운다. 야차가 무엇인지 말하는 **단 하나의 신호**라
 * 크고 둥글게 그린다. */
function BoxingGloves() {
  return (
    <g className="pg-gloves">
      {/* 먼쪽 날개 끝 */}
      <g className="pg-glove--far">
        <ellipse cx="30" cy="88" rx="10" ry="9" fill={GLOVE_DARK} />
        <path d="M22 84 C24 79 30 78 33 81" stroke={GLOVE} strokeWidth="4" strokeLinecap="round" fill="none" />
        <rect x="26" y="94" width="9" height="3.4" rx="1.7" fill={GLOVE_LACE} opacity="0.75" />
      </g>
      {/* 가까운쪽 날개 끝 */}
      <g className="pg-glove--near">
        <ellipse cx="73" cy="89" rx="11.5" ry="10.5" fill={GLOVE} />
        <path d="M82 84 C80 78 73 77 70 80" stroke={GLOVE_DARK} strokeWidth="4.5" strokeLinecap="round" fill="none" />
        <ellipse cx="76" cy="85" rx="4" ry="3" fill={GLOVE_LACE} opacity="0.35" />
        <rect x="67" y="96" width="11" height="4" rx="2" fill={GLOVE_LACE} />
      </g>
    </g>
  );
}

/** 챔피언 벨트 — 띠 + 판 + 가운데 장식 **세 겹**.
 *
 * **몸통 가운데에 그린다.** 몸통 외곽이 local x 29~71이라 띠가 그 폭을 다 덮고
 * 판이 정중앙(50)에 온다. 흰 배(`cx=47`)를 기준으로 잡으면 한쪽으로 쏠린다 —
 * 실제로 그렇게 났다 (2026-09-03 사용자). 좌우 대칭이라 `.pg-stage--flip`으로
 * 뒤집혀도 가운데를 지킨다.
 *
 * **두 곳에서 쓴다.** 챔피언의 허리(`.pg--yacha-champ`)와 미녀가 **팔로 든**
 * 벨트(`.pg-belt--held`). 그림은 한 벌이고, 든 자리로 옮기는 것은 CSS다.
 */
function ChampionBelt() {
  return (
    <g className="pg-belt">
      <rect x="28.5" y="81" width="43" height="12" rx="3" fill={BELT_GOLD_DARK} />
      <ellipse cx="50" cy="87" rx="11.5" ry="10" fill={BELT_GOLD} />
      <ellipse cx="50" cy="87" rx="6.5" ry="5.4" fill={BELT_GOLD_DARK} opacity="0.5" />
    </g>
  );
}

/** 화남 표시(💢) — 만화의 분노 핏줄. **채운 갈매기꼴 넷**을 0°·90°·180°·270°로.
 *
 * **가는 선으로 그리면 타격이 아니라 "번쩍임"으로 읽힌다.** 그래서 채움만 쓴다
 * (`stroke` 없음). 그리고 **가운데가 비어 있는 것이 핵심이다** — 꼭짓점을 중심
 * 쪽으로 밀면 넷이 겹쳐 메워져 그냥 X 표시가 된다 (실제로 한 번 그렇게 났다).
 *
 * **때린 놈과 맞은 놈 사이에 뜬다** (맞은 쪽에 가깝게 0.36:0.64). 뭉쳐 싸우므로
 * 그 자리가 곧 둘 사이라, 선도 화살표도 없이 짝이 읽힌다. 자리는 CSS가 정한다.
 */
function AngryMark() {
  const 갈매기 = "M-10 -19 L0 -10 L10 -19 L10 -12.5 L0 -3.5 L-10 -12.5 Z";
  return (
    <g className="pg-anger">
      <g transform="translate(86 30) scale(0.6)" fill={ANGER}>
        <path d={갈매기} />
        <path d={갈매기} transform="rotate(90)" />
        <path d={갈매기} transform="rotate(180)" />
        <path d={갈매기} transform="rotate(270)" />
      </g>
    </g>
  );
}
