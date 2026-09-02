/**
 * 포인터 드래그 한 번의 진행 상태 — **펭귄 창과 공 창이 함께 쓴다.**
 *
 * 규약은 하나다: 웹뷰는 이동량(델타)과 놓는 순간의 속도만 보내고, 창 위치의
 * 소유자는 Rust다. 공이 펭귄 창의 드래그를 "재사용한다"고 말하려면 던지기
 * 속도 계산만이 아니라 **궤적 샘플링까지** 같은 것을 써야 한다 — 두 벌을
 * 유지하면 한쪽만 고쳐진 채로 갈라진다.
 */

/** 속도 계산에 남겨 둘 궤적의 길이(ms). **개수로 자르지 않는다** — 포인터
 * 보고 주기(60Hz vs 120Hz)에 따라 재는 구간이 달라진다. */
export const SAMPLE_KEEP_MS = 400;

/** 궤적 한 점 — 화면 좌표와 시각. */
export interface DragSample {
  x: number;
  y: number;
  t: number;
}

export interface DragTrack {
  /** 이 드래그를 소유한 포인터. 다른 포인터의 이벤트는 무시한다. */
  pointerId: number;
  /** 직전 지점 (화면 좌표). 다음 이벤트와의 차이가 곧 이동량이다. */
  screenX: number;
  screenY: number;
  /** 지금까지 움직인 총량. 클릭과 드래그를 가르는 데 쓴다. */
  moved: number;
  /** 최근 궤적 — 놓는 순간의 속도를 재는 데 쓴다. */
  samples: DragSample[];
}

export const newDragTrack = (
  pointerId: number,
  screenX: number,
  screenY: number,
): DragTrack => ({
  pointerId,
  screenX,
  screenY,
  moved: 0,
  samples: [{ x: screenX, y: screenY, t: performance.now() }],
});

/** 궤적에 한 점을 더하고 오래된 점을 버린다. */
export function pushSample(track: DragTrack, x: number, y: number): void {
  const t = performance.now();
  track.samples.push({ x, y, t });
  while (track.samples.length > 2 && t - track.samples[0].t > SAMPLE_KEEP_MS) {
    track.samples.shift();
  }
}

/** 이번 이벤트의 이동량을 반영하고 궤적에 기록한다. 움직이지 않았으면 `null`. */
export function advance(
  track: DragTrack,
  screenX: number,
  screenY: number,
): { dx: number; dy: number } | null {
  const dx = screenX - track.screenX;
  const dy = screenY - track.screenY;
  if (dx === 0 && dy === 0) return null;
  track.screenX = screenX;
  track.screenY = screenY;
  track.moved += Math.abs(dx) + Math.abs(dy);
  pushSample(track, screenX, screenY);
  return { dx, dy };
}
