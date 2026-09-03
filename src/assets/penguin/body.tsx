/** 펭귄의 몸 — 그림자·얼음구멍·꼬리·날개·발·몸통·머리·눈·부리.
 *
 * 훌라 차림이 몸통과 가까운쪽 날개 **사이**에 끼므로 셋으로 나뉘어 나간다.
 */

import { BEAK, FOOT, HOLE, INK, SNOW } from "../palette";

/** 그림자와 얼음 구멍. **`.pg-all` 밖이다** — 착지 포즈에 같이 눌리면 안 된다. */
export function Ground() {
  return (
    <>
      <ellipse
        className="pg-shadow"
        cx="50"
        cy="123"
        rx="23"
        ry="4.5"
        fill={INK}
        opacity="0.18"
      />

      <ellipse className="pg-hole" cx="88" cy="121" rx="12" ry="3.8" fill={HOLE} />
    </>
  );
}

/** 몸의 뒤쪽 절반 — 꼬리부터 머리까지. 훌라 차림이 이 위에 얹힌다. */
export function BodyBack() {
  return (
    <>
      <path className="pg-tail" d="M33 98 L16 111 L35 106 Z" fill={INK} />

      <path
        className="pg-wing-far"
        d="M36 52 C27 59 24 76 29 88 C31 92 35 91 36 87 C38 75 39 61 39 54 Z"
        fill={INK}
        opacity="0.72"
      />

      <g className="pg-foot--far">
        <rect x="44" y="104" width="4" height="9" rx="2" fill={FOOT} opacity="0.85" />
        <path d="M46 112 L38 120 L52 120 Z" fill={FOOT} opacity="0.85" />
      </g>
      <g className="pg-foot--near">
        <rect x="54" y="104" width="4" height="9" rx="2" fill={FOOT} />
        <path d="M56 112 L48 120 L63 120 Z" fill={FOOT} />
      </g>

      <g className="pg-body">
        <path
          d="M50 40 C64 40 71 58 71 80 C71 101 62 113 50 113 C38 113 29 101 29 80 C29 58 36 40 50 40 Z"
          fill={INK}
        />
        <ellipse cx="47" cy="82" rx="14.5" ry="26" fill={SNOW} />
      </g>

      <g className="pg-head">
        <circle cx="50" cy="30" r="16" fill={INK} />
        <path d="M64 29 L77 32.5 L64 36.5 Z" fill={BEAK} />
        <path className="pg-beak-lower" d="M64 33 L76 33.5 L64 38.5 Z" fill={BEAK} />
        <ellipse cx="57" cy="27" rx="5.2" ry="6.2" fill={SNOW} />
        <g className="pg-gaze">
          <circle className="pg-eye" cx="57.5" cy="27.5" r="2.5" fill={INK} />
        </g>
        {/* 야차에서 쓰러졌을 때의 X자 눈. 평소에는 숨고, 뜰 때는 위의
            `pg-eye`가 대신 숨는다 — 둘이 겹치면 X 위에 점이 남는다. */}
        <path
          className="pg-eye-x"
          d="M54.6 24.6 L60.4 30.4 M60.4 24.6 L54.6 30.4"
          stroke={INK}
          strokeWidth="1.9"
          strokeLinecap="round"
          fill="none"
        />
      </g>
    </>
  );
}

/** 가까운쪽 날개. 훌라 차림보다 뒤에 그린다 — 앞이면 어깨끈이 날개를 덮는다. */
export function WingNear() {
  return (
      <path
        className="pg-wing-near"
        d="M66 50 C76 57 80 75 74 89 C72 93 68 91 67 87 C64 74 63 60 64 52 Z"
        fill={INK}
      />
  );
}
