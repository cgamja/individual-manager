/**
 * 레이어드 SVG 펭귄 — `assets/penguin-icon.png`의 아델리 펭귄을 옮겨 그렸다 (KTD7).
 *
 * 부위를 `<g>`로 나눈 이유는 CSS가 부위별로 독립 애니메이션할 수 있게 하기 위해서다.
 * 한 장짜리 래스터 사진으로는 날개·발이 따로 움직일 수 없다.
 * 애니메이션은 `pet.css`가 담당하고 여기서는 모양과 레이어 구조만 정의한다.
 *
 * 기본 방향은 **오른쪽**을 본다. 왼쪽은 루트의 `scaleX(-1)`로 뒤집는다 (R4).
 */

/** 몸통·머리·날개의 검정. 순검정보다 살짝 풀어야 투명 배경에서 덜 딱딱하다. */
const INK = "#1b1f24";
/** 배·눈테의 흰색. */
const SNOW = "#f7f9fb";
/** 부리 — 아델리 펭귄은 벽돌빛이 도는 검정이다. */
const BEAK = "#4a2f2f";
/** 발 — 분홍빛 살색. */
const FOOT = "#e3a892";

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
      {/* 바닥 그림자 — 착지·점프에서 크기가 변해 높이를 읽히게 한다 */}
      <ellipse id="shadow" className="pg-shadow" cx="50" cy="123" rx="23" ry="4.5" fill={INK} opacity="0.18" />

      {/* 꼬리 — 진행 방향 반대쪽으로 뻗는다 */}
      <path id="tail" className="pg-tail" d="M33 98 L16 111 L35 106 Z" fill={INK} />

      {/* 먼 쪽 날개 — 몸통 뒤에 깔려 약간 어둡다 */}
      <path
        id="wing-far"
        className="pg-wing-far"
        d="M36 52 C27 59 24 76 29 88 C31 92 35 91 36 87 C38 75 39 61 39 54 Z"
        fill={INK}
        opacity="0.72"
      />

      {/* 다리와 발 — 걷기에서 번갈아 움직인다 */}
      <g id="foot-far" className="pg-foot pg-foot--far">
        <rect x="44" y="104" width="4" height="9" rx="2" fill={FOOT} opacity="0.85" />
        <path d="M46 112 L38 120 L52 120 Z" fill={FOOT} opacity="0.85" />
      </g>
      <g id="foot-near" className="pg-foot pg-foot--near">
        <rect x="54" y="104" width="4" height="9" rx="2" fill={FOOT} />
        <path d="M56 112 L48 120 L63 120 Z" fill={FOOT} />
      </g>

      {/* 몸통 — 등은 검정, 배는 흰색 */}
      <g id="body" className="pg-body">
        <path
          d="M50 40 C64 40 71 58 71 80 C71 101 62 113 50 113 C38 113 29 101 29 80 C29 58 36 40 50 40 Z"
          fill={INK}
        />
        <ellipse cx="47" cy="82" rx="14.5" ry="26" fill={SNOW} />
      </g>

      {/* 머리 — 두리번·응시에서 회전한다. transform-origin은 목(50,44)이다 */}
      <g id="head" className="pg-head">
        <circle cx="50" cy="30" r="16" fill={INK} />
        {/* 부리 — 오른쪽을 향한다 */}
        <path d="M64 29 L77 32.5 L64 36.5 Z" fill={BEAK} />
        {/* 흰 눈테 — 아델리 펭귄의 식별 특징 */}
        <ellipse cx="57" cy="27" rx="5.2" ry="6.2" fill={SNOW} />
        <circle id="eye" className="pg-eye" cx="57.5" cy="27.5" r="2.5" fill={INK} />
      </g>

      {/* 가까운 쪽 날개 — 몸통 앞이라 기지개·놀람에서 가장 잘 보인다 */}
      <path
        id="wing-near"
        className="pg-wing-near"
        d="M66 50 C76 57 80 75 74 89 C72 93 68 91 67 87 C64 74 63 60 64 52 Z"
        fill={INK}
      />
    </svg>
  );
}
