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
/** 비치발리볼 지푸라기 걸침 — 마른 풀색. 평소에는 `display: none`이다. */
const STRAW = "#d8b25f";
/** 지푸라기의 그늘진 결. 실루엣 안에서 결이 읽힐 만큼만 어둡다. */
const STRAW_DARK = "#a8822f";

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

      {/* **지푸라기 걸침 — 상의만 있고 하의는 없다.**

          비키니를 접은 이유는 취향이 아니라 **적나라해서**다. 수영복 하의는
          펭귄 몸에 속옷처럼 얹혀 오히려 눈에 띄었고, 지우면 원래의 펭귄 몸으로
          돌아갈 뿐이라 그쪽이 덜 노출된다.

          그림의 기준은 **가릴 것을 제대로 가리는 쪽**이다 — 끈이나 조각이 아니라
          몸통을 넉넉히 덮는 **덮개**로 잡았다. 가는 선을 여러 개 그리지 않은
          것도 같은 이유다: 창이 140px이고 자세가 빠르게 바뀌어서 얇은 선은
          움직이면 뭉개지고, 뭉개지면 "지푸라기"가 아니라 얼룩으로 읽힌다.

          **`.pg-all` 안이고 몸통 위, 날개 아래다.** 밖에 두면 착지 포즈에서
          몸만 눌리고 옷이 허공에 남고, 날개 위에 두면 날개를 저을 때 어깨끈이
          날개를 덮는다. */}
      <g className="pg-straw">
        {/* 어깨에 걸치는 끈 — 덮개가 흘러내리지 않는다는 것만 읽히면 된다 */}
        <path
          d="M38 66 L43 55 M57 66 L52 55"
          stroke={STRAW}
          strokeWidth="4"
          strokeLinecap="round"
          fill="none"
        />
        {/* 덮개 본체. 배의 흰 부분을 가슴부터 배까지 넉넉히 덮는다 */}
        <path
          d="M33 64 C40 58 55 58 61 64 C62 78 58 92 47 94 C36 92 32 78 33 64 Z"
          fill={STRAW}
        />
        {/* 아랫단을 지푸라기 끝으로 톱니지게 — 실루엣만으로 짚이 읽힌다 */}
        <path
          d="M33.4 84 L37 95 L40 85 L43.5 97 L47 86 L50.5 97 L54 85 L57 95 L60.6 84 Z"
          fill={STRAW}
        />
        {/* 엮은 결 둘. 셋 이상 그리면 이 크기에서 뭉갠다 */}
        <path
          d="M35 72 C41 69 53 69 59 72 M34.5 80 C41 77 53 77 59.5 80"
          stroke={STRAW_DARK}
          strokeWidth="2"
          strokeLinecap="round"
          fill="none"
          opacity="0.75"
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
