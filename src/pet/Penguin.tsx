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
      {/*
       * 후광 — 같은 도형을 한 벌 더 뒤에 깐다 (어두운 배경에서 검은 몸통이 묻히는 문제).
       *
       * 클래스가 같으므로 CSS 애니메이션이 그대로 걸려 본체와 똑같이 움직인다 —
       * 정적 실루엣 하나로는 날개짓·고개돌림을 따라가지 못한다. 전부 한 색이라
       * 안쪽 겹침은 서로 묻히고 **바깥 실루엣만** 테두리로 남는다. 부위마다
       * stroke를 걸면 날개와 몸통 사이에도 선이 생겨 보기 힘들어진다.
       *
       * `filter: drop-shadow()`가 모양은 가장 예쁘지만, 애니메이션되는 서브트리
       * 전체를 매 프레임 오프스크린으로 다시 그려 CPU가 크게 뛴다.
       */}
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
      {/* 바닥 그림자 — 착지·점프에서 크기가 변해 높이를 읽히게 한다 */}
      <ellipse
        className="pg-shadow"
        cx="50"
        cy="123"
        rx="23"
        ry="4.5"
        fill={INK}
        opacity="0.18"
      />

      {/*
        얼음 구멍 — 낚시할 때만 보인다. **본체(`pg-all`) 밖**이다:
        앉을 때 몸을 누르는 변형에 구멍까지 딸려 들어가면 바닥에 뚫린 구멍이
        아니라 펭귄에 붙은 무늬가 된다. 그림자와 같은 층에 둔 이유가 그것이다.
      */}
      <ellipse className="pg-hole" cx="88" cy="121" rx="12" ry="3.8" fill={HOLE} />

      {/*
        펭귄 본체 전체. **착지 포즈(철푸덕·널브러짐)는 이 덩어리를 통째로 누른다.**
        부위마다 자기 축으로 변형하면 몸통만 납작해지고 머리와 날개는 제자리에 남아
        공중에 떠 버린다 — 실제로 그렇게 만들었다가 다시 갈아엎었다.
        그림자는 바닥에 있어야 하므로 이 밖에 둔다.
      */}
      <g className="pg-all">
      {/* 꼬리 — 진행 방향 반대쪽으로 뻗는다 */}
      <path className="pg-tail" d="M33 98 L16 111 L35 106 Z" fill={INK} />

      {/* 먼 쪽 날개 — 몸통 뒤에 깔려 약간 어둡다 */}
      <path
        className="pg-wing-far"
        d="M36 52 C27 59 24 76 29 88 C31 92 35 91 36 87 C38 75 39 61 39 54 Z"
        fill={INK}
        opacity="0.72"
      />

      {/* 다리와 발 — 걷기에서 번갈아 움직인다 */}
      <g className="pg-foot pg-foot--far">
        <rect x="44" y="104" width="4" height="9" rx="2" fill={FOOT} opacity="0.85" />
        <path d="M46 112 L38 120 L52 120 Z" fill={FOOT} opacity="0.85" />
      </g>
      <g className="pg-foot pg-foot--near">
        <rect x="54" y="104" width="4" height="9" rx="2" fill={FOOT} />
        <path d="M56 112 L48 120 L63 120 Z" fill={FOOT} />
      </g>

      {/* 몸통 — 등은 검정, 배는 흰색 */}
      <g className="pg-body">
        <path
          d="M50 40 C64 40 71 58 71 80 C71 101 62 113 50 113 C38 113 29 101 29 80 C29 58 36 40 50 40 Z"
          fill={INK}
        />
        <ellipse cx="47" cy="82" rx="14.5" ry="26" fill={SNOW} />
      </g>

      {/* 머리 — 두리번·응시에서 회전한다. transform-origin은 목(50,44)이다 */}
      <g className="pg-head">
        <circle cx="50" cy="30" r="16" fill={INK} />
        {/* 부리 — 오른쪽을 향한다 */}
        <path d="M64 29 L77 32.5 L64 36.5 Z" fill={BEAK} />
        {/* 흰 눈테 — 아델리 펭귄의 식별 특징 */}
        <ellipse cx="57" cy="27" rx="5.2" ry="6.2" fill={SNOW} />
        {/* 눈동자는 시선 추적용 그룹 안에 둔다 — 깜빡임과 회전축이 겹치지 않게 */}
        <g className="pg-gaze">
          <circle className="pg-eye" cx="57.5" cy="27.5" r="2.5" fill={INK} />
        </g>
      </g>

      {/* 가까운 쪽 날개 — 몸통 앞이라 기지개·놀람에서 가장 잘 보인다 */}
      <path
        className="pg-wing-near"
        d="M66 50 C76 57 80 75 74 89 C72 93 68 91 67 87 C64 74 63 60 64 52 Z"
        fill={INK}
      />

      {/*
       * 야구방망이 — 평소엔 숨어 있고 휘두를 때만 보인다.
       *
       * **날개의 연장선으로 그린다.** 손잡이 끝을 날개 끝(약 71,86)에 두고
       * 팔이 뻗은 방향으로 이어 놓아야, 같은 축(66,52)으로 돌 때 쥐고 휘두르는
       * 그림이 된다. 팔과 반대 방향으로 그리면 축은 같아도 각도가 어긋나 보인다.
       */}
      <g className="pg-bat">
        <path
          d="M69.2 85.8 L71.9 85.2 L76.2 122 C76.7 126.6 74.7 129.2 72.6 129.2 C70.5 129.2 68.7 126.6 69.2 122 Z"
          fill={BAT_WOOD}
        />
        <rect x="68.4" y="82.4" width="3.9" height="6.4" rx="1.7" fill={BAT_GRIP} />
      </g>

      {/*
       * 낚시 도구 — 낚싯대·낚싯줄·찌·물고기. 평소엔 숨어 있다.
       *
       * **본체 안**이다: 쥐고 있는 것이므로 앉는 변형·채는 동작을 몸과 함께
       * 받아야 한다. 밖에 두면 몸만 앉고 낚싯대는 공중에 남는다 —
       * 착지 포즈가 `pg-all`을 통째로 누르는 것과 같은 이유다.
       *
       * 낚싯대 손잡이 끝을 날개 끝(약 67,84)에 두어 팔의 연장선으로 그린다.
       * 방망이와 같은 규칙이다.
       */}
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
      {/*
        잡은 물고기는 **낚싯줄 끝에 걸려 있어야 한다.** 구멍 위 허공에 띄우면
        물고기가 저 혼자 떠 있는 그림이 된다 — 그래서 잡았을 때는 줄을
        `scaleY`로 줄여 끝(약 90,107)을 물고기 자리로 끌어올린다.
      */}
      <g className="pg-fish">
        <ellipse cx="85" cy="107" rx="7" ry="3.6" fill={FISH} />
        <path d="M92 107 L97 103.5 L97 110.5 Z" fill={FISH} />
        <circle cx="81" cy="106" r="0.9" fill={INK} />
      </g>
      </g>
    </>
  );
}
