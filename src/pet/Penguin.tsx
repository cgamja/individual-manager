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
/** 지푸라기(라피아) — **상의도 하의도 같은 재질이다.** 평소에는 `display: none`.
 *
 * **몸보다 확실히 진하다.** 배가 흰색(`SNOW`)이라 옅은 살구·크림 계열을 쓰면
 * 옷이 아니라 살로 읽힌다 — 덮개형 상의가 "갑바"로 읽혔던 것이 정확히 그
 * 실패였다. **얇게 그리는 것과 옷으로 읽히는 것은 충돌하지 않는다**: 두께가
 * 아니라 **대비와 경계**로 푼다. 두께로 존재감을 만들려 한 것이 갑바였다. */
const STRAW = "#c8912e";
/** 라피아의 테두리·끈·결. **경계가 보여야 천으로 읽힌다.** */
const STRAW_DARK = "#8f6414";
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

          **옷으로 읽히는 것이 기준이다.** 핑크 비키니(적나라) → 덮개형(갑바) →
          지푸라기 비키니로 두 번 뒤집힌 자리다. 배운 것은 하나다: 문제는
          노출량이 아니라 **옷으로 읽히느냐**였고, 그건 두께가 아니라 대비와
          경계로 푼다. 자세한 근거는 아래 상의 주석과 `MOTIONS.md`에 있다.

          가닥을 가는 선으로 여러 개 안 그린 것도 근거가 있다: 창이 140px이고
          자세가 빠르게 바뀌어 얇은 선은 움직이면 뭉개지고, 뭉개지면 라피아가
          아니라 얼룩으로 읽힌다. **실루엣이 한눈에 읽히는 덩어리**로 잡고
          가닥은 몇 개만 암시한다.

          **`.pg-all` 안이고 몸통 위, 날개 아래다.** 밖에 두면 착지 포즈에서
          몸만 눌리고 옷이 허공에 남고, 날개 위에 두면 날개를 저을 때 어깨끈이
          날개를 덮는다. */}
      <g className="pg-luau">
        {/* ── 치마 — 암수 공통. 허리띠 + 톱니진 라피아 단 ── */}
        <g className="pg-luau-skirt">
          {/* **얇은 허리끈.** 예전에는 여기가 두꺼운 판이었다 — 치마가 아니라
              반바지로 보였다. 끈 하나로 줄이고 가닥이 거기 매달리게 한다. */}
          <path
            d="M31 87 C38 82.5 56 82.5 63 87"
            stroke={STRAW}
            strokeWidth="5"
            strokeLinecap="round"
            fill="none"
          />
          {/* 가닥 치마. 실루엣은 한 덩어리로 잡되 아랫단만 톱니져서 지푸라기가
              읽힌다 — 창이 140px이라 가닥을 낱낱이 그리면 움직일 때 뭉갠다. */}
          <path
            d="M32 88 C39 84.5 55 84.5 62 88
               L60.5 98 L58.5 105 L57 97 L54 110 L52 98 L50 111.5
               L48 99 L46 111 L44 99 L41.5 108 L39.5 98 L37 104 L35.5 96 L33.5 97 Z"
            fill={STRAW}
          />
          {/* 가닥을 **몇 개만** 암시한다 — 창이 140px이라 낱낱이 그리면
              움직일 때 뭉개져 지푸라기가 아니라 얼룩으로 읽힌다 */}
          <path
            d="M41 89 L40 104 M47 89.5 L47 107 M54 89 L54.5 106"
            stroke={STRAW_DARK}
            strokeWidth="1.3"
            strokeLinecap="round"
            fill="none"
            opacity="0.7"
          />
          {/* 허리선 — 경계가 보여야 옷으로 읽힌다 */}
          <path
            d="M31.5 88 C38.5 84 55.5 84 62.5 88"
            stroke={STRAW_DARK}
            strokeWidth="1.8"
            strokeLinecap="round"
            fill="none"
          />
        </g>

        {/* ── 상의 — 암컷만. **지푸라기 삼각형 두 개 + 끈** ──

            모양은 비키니, 재질은 지푸라기다 (2026-09-02 사용자 확정).
            **덮개형으로 갔다가 되돌아왔다**: 몸통을 넉넉히 덮는 상의를 그렸더니
            옷이 아니라 **맨가슴 근육("갑바")으로 읽혔다.** 문제는 노출량이 아니라
            **옷으로 읽히느냐**였다 — 색이 몸에 가깝고 경계가 없으면 아무리 덮어도
            살로 보인다.

            그래서 **얇게 그리되 옷임을 분명히** 한다. 셋이 함께 있어야 한다:
            (1) 흰 배와 뚜렷이 대비되는 마른 풀색, (2) **끈이 보이는 실루엣**
            (목뒤로 올라가는 V와 등뒤로 도는 가로줄), (3) 도형마다 테두리.
            **두께로 존재감을 만들지 않는다** — 그게 갑바였다.

            **그리는 순서가 곧 겹치는 순서다.** 끈을 삼각형보다 **먼저** 그리면
            컵 뒤로 숨어 실루엣이 사라진다 — 등뒤 끈은 컵 위쪽 모서리(y=60.5)
            **위로** 나오게 두고 삼각형 뒤에 깔되, 목뒤 V는 컵보다 위에서
            시작하므로 가려지지 않는다. */}
        <g className="pg-luau-top">
          {/* 등뒤로 도는 끈. **컵 위로 나온다** — 아래에 두면 삼각형에 완전히
              덮여 끈이 있다는 사실 자체가 안 보인다. */}
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
          {/* 삼각형 둘. **얕게** 잡는다 — 깊으면 다시 덮개가 된다 */}
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

        {/* ── 레이 — 암수 공통. 수컷의 상체를 채우는 것이 이것 하나다.
            **상의보다 뒤에 그린다** — 목걸이는 끈 위에 걸리지 밑으로
            지나가지 않는다. */}
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
