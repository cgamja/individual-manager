import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ServiceIcon, serviceColor } from "./ServiceIcon";
import { DEFAULT_LAUNCHER, type LauncherServiceId } from "../lib/launcher";

afterEach(cleanup);

describe("서비스 아이콘", () => {
  it("4개_서비스_아이콘이_모두_그려진다", () => {
    for (const service of DEFAULT_LAUNCHER) {
      const { unmount } = render(<ServiceIcon id={service.id} />);
      expect(screen.getByRole("img")).toBeTruthy();
      unmount();
    }
  });

  it("알_수_없는_id는_아무것도_그리지_않는다", () => {
    const { container } = render(<ServiceIcon id={"slack" as LauncherServiceId} />);
    expect(container.querySelector("svg")).toBeNull();
  });

  it("서비스마다_다른_브랜드_색을_쓴다", () => {
    const colors = DEFAULT_LAUNCHER.map((s) => serviceColor(s.id));
    expect(new Set(colors).size).toBe(colors.length);
  });
});
