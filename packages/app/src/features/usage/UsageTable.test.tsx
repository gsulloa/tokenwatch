import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { UsageTable } from "./UsageTable";
import { buildColorMap } from "./colors";
import type { SeriesResponse } from "./types";

const RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "tokens",
  buckets: ["2026-07-01", "2026-07-02"],
  series: [
    { name: "claude-opus-4-8", points: [1000, 2000] },
    { name: "claude-sonnet-4-6", points: [500, 800] },
  ],
};

// Ordered by total descending: opus (3000) > sonnet (1300)
const ORDERED_NAMES = ["claude-opus-4-8", "claude-sonnet-4-6"];
const COLOR_MAP = buildColorMap(ORDERED_NAMES);

const COST_RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "cost",
  buckets: ["2026-07-01", "2026-07-02"],
  series: [
    { name: "model-a", points: [1.5, 2.25] },
    { name: "model-b", points: [0.75, 1.0] },
  ],
};
const COST_ORDERED = ["model-a", "model-b"];
const COST_MAP = buildColorMap(COST_ORDERED);

describe("UsageTable", () => {
  it("renders bucket header columns", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    expect(screen.getByText("2026-07-01")).toBeInTheDocument();
    expect(screen.getByText("2026-07-02")).toBeInTheDocument();
  });

  it("renders all series names in order", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    const cells = screen.getAllByText(/claude/i);
    // Both series names should appear
    expect(cells.some((c) => c.textContent?.includes("claude-opus-4-8"))).toBe(
      true,
    );
    expect(
      cells.some((c) => c.textContent?.includes("claude-sonnet-4-6")),
    ).toBe(true);
  });

  it("renders exact token values with thousands separators", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    // Individual bucket values
    expect(screen.getByText("1,000")).toBeInTheDocument();
    expect(screen.getByText("2,000")).toBeInTheDocument();
    expect(screen.getByText("500")).toBeInTheDocument();
    expect(screen.getByText("800")).toBeInTheDocument();
  });

  it("renders correct row totals per series", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    // opus: 1000+2000=3000, sonnet: 500+800=1300
    expect(screen.getByText("3,000")).toBeInTheDocument();
    expect(screen.getByText("1,300")).toBeInTheDocument();
  });

  it("renders correct per-bucket column totals in footer", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    // 2026-07-01: 1000+500=1500, 2026-07-02: 2000+800=2800
    expect(screen.getByText("1,500")).toBeInTheDocument();
    expect(screen.getByText("2,800")).toBeInTheDocument();
  });

  it("renders the grand total in the corner", () => {
    render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={COLOR_MAP}
      />,
    );
    // Grand total: 1000+2000+500+800 = 4300
    expect(screen.getByText("4,300")).toBeInTheDocument();
  });

  it("renders cost values with $ sign", () => {
    render(
      <UsageTable
        response={COST_RESPONSE}
        orderedNames={COST_ORDERED}
        colorMap={COST_MAP}
      />,
    );
    // $2.25 appears both as model-a bucket value and as column total (1.50+0.75=2.25)
    expect(screen.getAllByText("$1.50").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("$2.25").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("$0.75").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("$1.00").length).toBeGreaterThanOrEqual(1);
  });

  it("respects orderedNames ordering for row rendering", () => {
    const reversedOrder = [...ORDERED_NAMES].reverse();
    const { container } = render(
      <UsageTable
        response={RESPONSE}
        orderedNames={reversedOrder}
        colorMap={COLOR_MAP}
      />,
    );
    const rows = container.querySelectorAll("tbody tr");
    expect(rows[0]?.textContent).toContain("claude-sonnet-4-6");
    expect(rows[1]?.textContent).toContain("claude-opus-4-8");
  });

  it("assigns colors from the color map to series dots", () => {
    const customMap = new Map([
      ["claude-opus-4-8", "#ff0000"],
      ["claude-sonnet-4-6", "#0000ff"],
    ]);
    const { container } = render(
      <UsageTable
        response={RESPONSE}
        orderedNames={ORDERED_NAMES}
        colorMap={customMap}
      />,
    );
    const dots = container.querySelectorAll(".usage-table__dot");
    const dotColors = Array.from(dots).map(
      (d) => (d as HTMLElement).style.background,
    );
    expect(dotColors).toContain("rgb(255, 0, 0)");
    expect(dotColors).toContain("rgb(0, 0, 255)");
  });
});
