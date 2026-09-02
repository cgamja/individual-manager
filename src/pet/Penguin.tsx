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
/** 훌라 치마의 라피아 색. 평소에는 `display: none`이다.
 *
 * **몸보다 확실히 진하다.** 배가 흰색(`SNOW`)이라 옅은 살구·크림 계열을 쓰면
 * 옷이 아니라 살로 읽힌다 — 덮개형 상의가 "갑바"로 읽혔던 것이 정확히 그
 * 실패였다. */
const STRAW = "#c8912e";
/** 라피아의 그늘진 결. 실루엣 안에서 결이 읽힐 만큼만 어둡다. */
const STRAW_DARK = "#8f6414";
/** 비키니 — **옷으로 읽히게 하는 것이 유일한 기준이다.** 흰 배와도 검은 몸과도
 * 뚜렷이 대비되는 채도 높은 색. */
const BIKINI = "#e8446f";
/** 비키니의 테두리·끈. 경계가 보여야 천으로 읽힌다. */
const BIKINI_DARK = "#a82449";
/** 목에 거는 레이(꽃목걸이) — 여름 느낌을 내는 포인트. */
const LEI = "#e8543f";
const LEI_ALT = "#f2c14e";

interface PenguinOwnProps {
  /** 암컷인가 — 훌라 상의를 입힐지 정한다. 창 라벨에서 결정적으로 파생한다. */
  female?: boolean;
}

type PenguinProps = React.ComponentPropsWithoutRef<"svg"> & PenguinOwnProps;

export function Penguin({ className = "penguin", female = false, ...rest }: PenguinProps) {
  const cls = female ? `${className} pg-female` : className;
  return (
    <svg
      className={cls}
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

      {/* **훌라(luau) 차림 — 성별로 다르다.** `female`은 창 라벨에서 결정적으로
          파생한다 (`isFemalePet`) — 난수가 아니라 id 기반이라 앱을 껐다 켜도
          같은 펭귄은 같은 차림이고, 그래서 저장할 것도 없다.

          **수컷은 아래만, 암컷은 위아래 둘 다.** 전통 훌라 복식이 그렇다 —
          남자는 치마나 말로(malo)만 두르고 상체엔 레이·띠를 얹었고, 여자는
          치마에 상의를 갖췄다(무무). 코코넛 브라는 전통이 아니라 파티 코스튬의
          관용구라 안 쓴다.

          **처음엔 핑크 비키니였는데 적나라해서 접었다** (2026-09-02 사용자).
          그래서 암컷 상의는 끈이나 조각이 아니라 **몸통을 넉넉히 덮는 덮개**다 —
          얇게 그리면 접은 이유가 그대로 돌아온다.

          가닥을 가는 선으로 여러 개 안 그린 것도 근거가 있다: 창이 140px이고
          자세가 빠르게 바뀌어 얇은 선은 움직이면 뭉개지고, 뭉개지면 라피아가
          아니라 얼룩으로 읽힌다. **실루엣이 한눈에 읽히는 덩어리**로 잡고
          가닥은 몇 개만 암시한다.

          **`.pg-all` 안이고 몸통 위, 날개 아래다.** 밖에 두면 착지 포즈에서
          몸만 눌리고 옷이 허공에 남고, 날개 위에 두면 날개를 저을 때 어깨끈이
          날개를 덮는다. */}
      <g className="pg-luau">
        {/* ── 레이 — 암수 공통. 수컷의 상체를 채우는 것이 이것 하나다 ── */}
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

        {/* ── 치마 — 암수 공통. 허리띠 + 톱니진 라피아 단 ── */}
        <g className="pg-luau-skirt">
          <path
            d="M31 86 C38 81 56 81 63 86 C63 94 61 100 59 103 L35 103 C33 100 31 94 31 86 Z"
            fill={STRAW}
          />
          {/* 단을 톱니지게 — 실루엣만으로 라피아 가닥이 읽힌다 */}
          <path
            d="M31.5 98 L34 112 L37.5 100 L41 114 L44.5 101 L47 115 L49.5 101 L53 114 L56.5 100 L60 112 L62.5 98 Z"
            fill={STRAW}
          />
          {/* **허리선.** 치마도 상의와 같은 기준이다 — 경계가 보여야 옷으로
              읽힌다. 비키니와 같은 색으로 묶어 한 벌로 보이게 한다. */}
          <path
            d="M30.5 86.5 C38 82 56 82 63.5 86.5"
            stroke={BIKINI}
            strokeWidth="4"
            strokeLinecap="round"
            fill="none"
          />
          <path
            d="M31.5 93 C38 89.5 56 89.5 62.5 93"
            stroke={STRAW_DARK}
            strokeWidth="2.4"
            strokeLinecap="round"
            fill="none"
            opacity="0.8"
          />
        </g>

        {/* ── 상의 — 암컷만. **삼각형 두 개짜리 비키니** ──

            **덮개형으로 갔다가 되돌아왔다.** 몸통을 넉넉히 덮는 상의를 그렸더니
            옷이 아니라 **맨가슴 근육("갑바")으로 읽혔다** (2026-09-02 사용자).
            문제는 노출량이 아니라 **옷으로 읽히느냐**였다 — 색이 몸에 가깝고
            경계가 없으면 아무리 덮어도 살로 보인다.

            그래서 면적을 키우는 대신 **옷임을 분명히** 한다. 셋이 함께 있어야 한다:
            (1) 흰 배와 뚜렷이 대비되는 채도 높은 색, (2) **끈이 보이는 실루엣**
            (목뒤로 올라가는 V와 가슴 아래 가로줄), (3) 도형마다 테두리.
            이 셋이 있으면 작아도 옷으로 읽힌다. */}
        <g className="pg-luau-top">
          {/* 가슴 아래를 지나 등으로 도는 끈 */}
          <path
            d="M33 62 C40 66 55 66 62 62"
            stroke={BIKINI_DARK}
            strokeWidth="2.6"
            strokeLinecap="round"
            fill="none"
          />
          {/* 목뒤로 올라가는 V자 끈 */}
          <path
            d="M40 61 L47 49 L55 61"
            stroke={BIKINI_DARK}
            strokeWidth="2.4"
            strokeLinecap="round"
            strokeLinejoin="round"
            fill="none"
          />
          {/* 삼각형 두 개 */}
          <path
            d="M33.5 60.5 L47 60.5 L40.5 77 Z"
            fill={BIKINI}
            stroke={BIKINI_DARK}
            strokeWidth="1.6"
            strokeLinejoin="round"
          />
          <path
            d="M47 60.5 L60.5 60.5 L54 77 Z"
            fill={BIKINI}
            stroke={BIKINI_DARK}
            strokeWidth="1.6"
            strokeLinejoin="round"
          />
        </g>
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
