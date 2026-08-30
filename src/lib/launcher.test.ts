import { describe, expect, it } from "vitest";
import {
  DEFAULT_LAUNCHER,
  fanOffset,
  isOpenableUrl,
  normalizeLauncher,
  type LauncherService,
} from "./launcher";

describe("기본 목록", () => {
  it("기본_런처_목록은_4개_서비스를_지정된_순서로_갖는다", () => {
    expect(DEFAULT_LAUNCHER.map((s) => s.id)).toEqual([
      "notion",
      "jira",
      "gcal",
      "pomodoro",
    ]);
  });

  it("뽀모도로만_URL_항목이_없다", () => {
    // 뽀모도로 2단은 URL 목록이 아니라 타이머 UI다 (R4)
    for (const service of DEFAULT_LAUNCHER) {
      if (service.id === "pomodoro") expect(service.items).toHaveLength(0);
      else expect(service.items.length).toBeGreaterThan(0);
    }
  });

  it("구글_캘린더만_기본_URL이_채워져_있다", () => {
    // 나머지는 사용자 계정에만 있는 주소라 빈 값으로 둔다 (A2)
    const gcal = DEFAULT_LAUNCHER.find((s) => s.id === "gcal");
    expect(gcal?.items.every((item) => isOpenableUrl(item.url))).toBe(true);
    const notion = DEFAULT_LAUNCHER.find((s) => s.id === "notion");
    expect(notion?.items.every((item) => item.url === "")).toBe(true);
  });
});

describe("팬 배치", () => {
  const COUNT = 4;

  it("팬_가로_밀림은_가운데가_가장_크고_양_끝이_작다", () => {
    const dx = Array.from({ length: COUNT }, (_, i) => fanOffset(i, COUNT).dx);
    // 호를 그리므로 가운데 두 장이 가장 많이 밀리고 양 끝이 안으로 접힌다
    expect(Math.abs(dx[1])).toBeLessThan(Math.abs(dx[0]));
    expect(Math.abs(dx[2])).toBeLessThan(Math.abs(dx[3]));
    expect(dx[0]).toBeCloseTo(dx[3]);
    expect(dx[1]).toBeCloseTo(dx[2]);
  });

  it("팬_회전각은_가운데를_기준으로_좌우_대칭이다", () => {
    const rotate = Array.from({ length: COUNT }, (_, i) => fanOffset(i, COUNT).rotate);
    expect(rotate[0]).toBeCloseTo(-rotate[3]);
    expect(rotate[1]).toBeCloseTo(-rotate[2]);
  });

  it("회전각은_위에서_아래로_단조_증가한다", () => {
    const rotate = Array.from({ length: COUNT }, (_, i) => fanOffset(i, COUNT).rotate);
    for (let i = 1; i < rotate.length; i += 1) {
      expect(rotate[i]).toBeGreaterThan(rotate[i - 1]);
    }
  });

  it("카드가_한_장뿐이면_밀지도_기울이지도_않는다", () => {
    expect(fanOffset(0, 1)).toEqual({ dx: 0, rotate: 0 });
  });
});

describe("URL 검증", () => {
  it("http와_https_URL만_열_수_있다", () => {
    expect(isOpenableUrl("https://calendar.google.com/calendar/r/week")).toBe(true);
    expect(isOpenableUrl("http://localhost:3000")).toBe(true);
  });

  it("빈_문자열과_공백만_있는_값은_열_수_없다", () => {
    expect(isOpenableUrl("")).toBe(false);
    expect(isOpenableUrl("   ")).toBe(false);
  });

  it("file과_javascript_스킴은_열_수_없다", () => {
    expect(isOpenableUrl("file:///etc/passwd")).toBe(false);
    expect(isOpenableUrl("javascript:alert(1)")).toBe(false);
    expect(isOpenableUrl("/Applications/Calculator.app")).toBe(false);
  });
});

describe("저장된 값 정규화", () => {
  it("저장된_런처_값이_배열이_아니면_기본_목록으로_수렴한다", () => {
    expect(normalizeLauncher(null)).toEqual(DEFAULT_LAUNCHER);
    expect(normalizeLauncher({ notion: "..." })).toEqual(DEFAULT_LAUNCHER);
    expect(normalizeLauncher([])).toEqual(DEFAULT_LAUNCHER);
  });

  it("모르는_id는_버리고_아는_것만_남긴다", () => {
    const saved = [
      { id: "slack", label: "SLACK", items: [] },
      { id: "gcal", label: "달력", items: [{ label: "주간", url: "https://x.test" }] },
    ];
    const result = normalizeLauncher(saved);
    expect(result.map((s) => s.id)).toEqual(["gcal"]);
    expect(result[0].label).toBe("달력");
  });

  it("저장된_항목의_URL이_비어_있어도_항목_자체는_유지된다", () => {
    // R7의 "URL이 아직 없어요" 표시를 위해 항목을 지우지 않는다
    const saved: LauncherService[] = [
      { id: "jira", label: "JIRA", items: [{ label: "내 티켓", url: "" }] },
    ];
    expect(normalizeLauncher(saved)).toEqual(saved);
  });

  it("항목_모양이_깨진_서비스는_통째로_버린다", () => {
    const saved = [{ id: "jira", label: "JIRA", items: [{ label: 3, url: null }] }];
    expect(normalizeLauncher(saved)).toEqual(DEFAULT_LAUNCHER);
  });
});
