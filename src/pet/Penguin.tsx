/** 레이어드 SVG 펭귄 — `assets/penguin-icon.png`의 아델리 펭귄을 옮겨 그렸다 (KTD7). */

/** 몸통·머리·날개의 검정. 순검정보다 살짝 풀어야 투명 배경에서 덜 딱딱하다. */
const INK = "#1b1f24";
/** 배·눈테의 흰색. */
const SNOW = "#f7f9fb";
/** 부리 — 아델리 펭귄은 벽돌빛이 도는 검정이다. */
const BEAK = "#4a2f2f";
/** 발 — 분홍빛 살색. */
const FOOT = "#e3a892";
/** 방망이 나무결. */
const BAT_WOOD = "#a1712f";
/** 방망이 손잡이. */
const BAT_GRIP = "#26262b";
/** 얼음 구멍 속 — 물빛. */
const HOLE = "#2f4a63";
/** 찌 — 눈에 띄어야 "지금 저기를 보고 있다"가 읽힌다. */
const FLOAT = "#d94f3d";
/** 잡은 물고기. */
const FISH = "#8fb3c9";
/** 비치발리볼 비키니 — 핑크. 평소에는 `display: none`이다. */
const BIKINI = "#ff5f9e";

type PenguinProps = React.ComponentPropsWithoutRef<"svg">;

export function Penguin({ className = "penguin", ...rest }: PenguinProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 100 130"
      role="img"
      aria-label="펭귄"
      xmlns="http://www.w3.org/2000/svg"
      {...rest}
    >
      <g className="pg-halo" aria-hidden="true">
        <Shapes />
      </g>
      <Shapes />
    </svg>
  );
}

/** 펭귄을 이루는 도형들. 본체와 후광이 같은 것을 그린다. */
function Shapes() {
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

      <g className="pg-all">
      <path className="pg-tail" d="M33 98 L16 111 L35 106 Z" fill={INK} />

      <path
        className="pg-wing-far"
        d="M36 52 C27 59 24 76 29 88 C31 92 35 91 36 87 C38 75 39 61 39 54 Z"
        fill={INK}
        opacity="0.72"
      />

      <g className="pg-foot pg-foot--far">
        <rect x="44" y="104" width="4" height="9" rx="2" fill={FOOT} opacity="0.85" />
        <path d="M46 112 L38 120 L52 120 Z" fill={FOOT} opacity="0.85" />
      </g>
      <g className="pg-foot pg-foot--near">
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
      </g>

      {/* **`.pg-all` 안이고 몸통 위, 날개 아래다.** 밖에 두면 착지 포즈에서
          몸만 눌리고 수영복이 허공에 남고, 날개 위에 두면 날개를 저을 때
          어깨끈이 날개를 덮는다. */}
      <g className="pg-bikini">
        <path
          d="M36 68 C41 63 53 63 58 68 C56 74 51 76 47 76 C43 76 38 74 36 68 Z"
          fill={BIKINI}
        />
        <path
          d="M37 66 L41 58 M57 66 L52 58"
          stroke={BIKINI}
          strokeWidth="2.4"
          strokeLinecap="round"
          fill="none"
        />
        <path
          d="M35 92 C40 88 54 88 59 92 C58 100 52 104 47 104 C42 104 36 100 35 92 Z"
          fill={BIKINI}
        />
      </g>

      <path
        className="pg-wing-near"
        d="M66 50 C76 57 80 75 74 89 C72 93 68 91 67 87 C64 74 63 60 64 52 Z"
        fill={INK}
      />

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
      </g>
    </>
  );
}
