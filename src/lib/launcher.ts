import { invoke } from "@tauri-apps/api/core";

/**
 * 런처 코어 — Tauri에 의존하지 않는 순수 부분(목록·팬 배치·URL 검증)과
 * Rust 커맨드 래퍼가 함께 있다. 순수 부분만 테스트한다.
 */

export type LauncherServiceId = "notion" | "jira" | "gcal" | "pomodoro";

/** 팬 1단에 놓이는 서비스 순서. 저장된 값도 이 순서로 다시 세운다. */
const SERVICE_ORDER: readonly LauncherServiceId[] = ["notion", "jira", "gcal", "pomodoro"];

export interface LauncherItem {
  label: string;
  /** 비어 있을 수 있다 — 사용자 계정에만 있는 주소는 기본값을 알 수 없다 (R7). */
  url: string;
}

export interface LauncherService {
  id: LauncherServiceId;
  label: string;
  /** 2단에 펼칠 URL 항목. 뽀모도로는 타이머 UI를 펼치므로 비어 있다. */
  items: LauncherItem[];
}

/**
 * 기본 목록. 편집 UI는 후속 작업이라 지금은 이 상수가 유일한 원천이다.
 * Notion·Jira 주소는 사용자 계정에만 있어 빈 값으로 두고, 런처는 그 자리에
 * "URL이 아직 없어요"를 보여준다.
 */
export const DEFAULT_LAUNCHER: readonly LauncherService[] = [
  {
    id: "notion",
    label: "NOTION",
    items: [
      { label: "TODO 보드", url: "" },
      { label: "오늘 페이지", url: "" },
    ],
  },
  {
    id: "jira",
    label: "JIRA",
    items: [
      { label: "내 티켓", url: "" },
      { label: "스프린트 보드", url: "" },
    ],
  },
  {
    id: "gcal",
    label: "GOOGLE CALENDAR",
    items: [
      { label: "주간 뷰", url: "https://calendar.google.com/calendar/r/week" },
      { label: "오늘", url: "https://calendar.google.com/calendar/r/day" },
    ],
  },
  { id: "pomodoro", label: "POMODORO", items: [] },
];

/** 호가 오른쪽으로 부푸는 정도(px)와 카드 사이 기울기 차(deg). */
const ARC_BULGE_PX = 3.2;
const TILT_STEP_DEG = 1.4;

export interface FanOffset {
  /** 가로 밀림 — 가운데가 가장 많이 밀리고 양 끝이 안으로 접힌다. */
  dx: number;
  /** 기울기 — 위에서 아래로 커지며 가운데를 기준으로 대칭이다. */
  rotate: number;
}

/**
 * 팬에서 i번째 카드의 배치. 세로 리듬은 CSS flex `gap`이 맡고,
 * 여기서는 호를 만드는 가로 밀림과 기울기만 계산한다 (KTD2).
 */
export function fanOffset(index: number, count: number): FanOffset {
  if (count <= 1) return { dx: 0, rotate: 0 };
  const center = (count - 1) / 2;
  const d = index - center;
  return { dx: -(d * d) * ARC_BULGE_PX, rotate: d * TILT_STEP_DEG };
}

/**
 * 열어도 되는 URL인가. `http`/`https`만 통과시킨다 — 그 외 스킴이나 파일 경로를
 * 그대로 넘기면 `open -a`에 임의 대상을 태울 수 있다 (KTD5).
 * Rust도 같은 검증을 다시 한다. 웹뷰의 판단을 믿지 않기 위해서다.
 */
export function isOpenableUrl(url: string): boolean {
  const trimmed = url.trim();
  if (trimmed === "") return false;
  try {
    const scheme = new URL(trimmed).protocol;
    return scheme === "http:" || scheme === "https:";
  } catch {
    return false;
  }
}

function isItem(value: unknown): value is LauncherItem {
  const item = value as Partial<LauncherItem> | null;
  return typeof item?.label === "string" && typeof item?.url === "string";
}

function isService(value: unknown): value is LauncherService {
  const service = value as Partial<LauncherService> | null;
  return (
    typeof service?.label === "string" &&
    SERVICE_ORDER.includes(service?.id as LauncherServiceId) &&
    Array.isArray(service?.items) &&
    service.items.every(isItem)
  );
}

/**
 * 저장된 값을 쓸 수 있는 목록으로 만든다. 깨져 있거나 아무것도 남지 않으면
 * 기본 목록으로 수렴시킨다 — 런처가 조용히 비어 버리지 않게 (`loadTaunts`와 같은 태도).
 * URL이 빈 항목은 그대로 남긴다: 그게 "아직 안 채웠다"는 상태다.
 */
export function normalizeLauncher(value: unknown): LauncherService[] {
  if (!Array.isArray(value)) return DEFAULT_LAUNCHER.map(cloneService);
  const kept = value.filter(isService);
  if (kept.length === 0) return DEFAULT_LAUNCHER.map(cloneService);
  return SERVICE_ORDER.flatMap((id) => {
    const found = kept.find((service) => service.id === id);
    return found ? [cloneService(found)] : [];
  });
}

function cloneService(service: LauncherService): LauncherService {
  return { ...service, items: service.items.map((item) => ({ ...item })) };
}

/** 서비스 URL을 Chrome으로 연다. 검증은 Rust가 한 번 더 한다. */
export async function openInBrowser(url: string): Promise<void> {
  await invoke("launcher_open", { url });
}
