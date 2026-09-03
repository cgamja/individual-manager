/** 이 앱의 모든 색.
 *
 * 값을 바꾸면 `assets.test.ts`의 렌더 스냅샷이 빨개진다 — 색을 바꾸는 것은
 * 그림을 바꾸는 것이고, 스냅샷 갱신이 그 선언이다.
 */

// ── 펭귄 ──

/** 몸통·머리·날개의 검정. 순검정보다 살짝 풀어야 투명 배경에서 덜 딱딱하다. */
export const INK = "#1b1f24";
/** 배·눈테의 흰색. */
export const SNOW = "#f7f9fb";
/** 부리 — 아델리 펭귄은 벽돌빛이 도는 검정이다. */
export const BEAK = "#4a2f2f";
/** 발. */
export const FOOT = "#e3a892";

// ── 펭귄이 드는 것 ──

/** 방망이·낚싯대의 나무결. **커서 방망이와 같은 색이다** — 밝아야 검은 몸통에서
 * 분리된다(전에는 `#a1712f`라 몸통 가장자리에 묻혔다). */
export const BAT_WOOD = "#d59a55";
/** 방망이 테두리. 커서처럼 배경 위에 뜰 때 실루엣이 뭉개지지 않게 준다. */
export const BAT_EDGE = "#6b4520";
/** 방망이 손잡이. */
export const BAT_GRIP = "#3a2a1a";
/** 얼음 구멍 속. */
export const HOLE = "#2f4a63";
/** 찌. */
export const FLOAT = "#d94f3d";
/** 잡은 물고기. */
export const FISH = "#8fb3c9";

// ── 훌라 차림 ──

/** 지푸라기(라피아) — 상의도 하의도 같은 재질이다.
 *
 * 흰 배(`SNOW`)와 밝기 차가 60을 넘어야 옷으로 읽힌다 —
 * `pet-css.test.ts`의 `옷_색이_몸_색과_대비된다`가 지킨다. */
export const STRAW = "#c8912e";
/** 라피아의 테두리·끈·결. */
export const STRAW_DARK = "#8f6414";
/** 목에 거는 레이(꽃목걸이). */
export const LEI = "#e8543f";
export const LEI_ALT = "#f2c14e";

// ── 볼링 공 ──

/** 공 몸통. */
export const BOWL_BALL = "#2b2f4a";
/** 공 테두리. */
export const BOWL_RIM = "#12142a";
/** 윗면에 비치는 빛. */
export const BOWL_SHEEN = "#6f76a8";
/** 손가락 구멍 셋. */
export const BOWL_HOLE = "#0d0f1e";

// ── 비치볼 ──

/** 흰 바탕. */
export const BEACH_WHITE = "#fdfcf7";
/** 조각 셋. */
export const BEACH_PINK = "#ff6f9c";
export const BEACH_BLUE = "#3fb8d8";
export const BEACH_YELLOW = "#ffd35c";
/** 안쪽 이음선. */
export const BEACH_SEAM = "#c9c2b0";
/** 바깥 테두리. */
export const BEACH_RIM = "#b9b1a0";
/** 위쪽 하이라이트. */
export const BEACH_SHEEN = "#ffffff";

// ── 비치발리볼 코트 ──

/** 모래 — 위에서 아래로. */
export const SAND_TOP = "#f4dfae";
export const SAND_BOTTOM = "#d9b878";
/** 물결선 — 여기가 펭귄의 발밑이다. */
export const SAND_FOAM = "#fff3d4";
/** 네트 기둥. */
export const NET_POST = "#a9743c";
/** 그물. */
export const NET_MESH = "#f4f1e8";

// ── 단체 야차 ──────────────────────────────────────────────

/** 복싱 장갑. */
export const GLOVE = "#c8242e";
export const GLOVE_DARK = "#8d1520";
/** 장갑 손목 끈. */
export const GLOVE_LACE = "#f2e4d0";

/** 챔피언 벨트의 가죽 띠. */
export const BELT_LEATHER = "#3a2a1a";
/** 벨트 가운데 금빛 원판. */
export const BELT_GOLD = "#e8c15a";
export const BELT_GOLD_DARK = "#a8842c";

/** 화장한 미녀 펭귄 — 입술과 볼터치. 속눈썹은 `INK`를 쓴다. */
export const LIP = "#e0466b";
export const BLUSH = "#f7a8bb";

/** 타격의 화남 표시(💢). */
export const ANGER = "#e03131";
