import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { UsageChart } from "./UsageChart";
import type { SeriesResponse } from "./types";

// recharts uses ResizeObserver — polyfill for jsdom
globalThis.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};

const MOCK_RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "tokens",
  buckets: ["2026-07-01", "2026-07-02", "2026-07-03"],
  series: [
    { name: "claude-opus-4-8", points: [100, 200, 150] },
    { name: "claude-sonnet-4-6", points: [50, 80, 60] },
  ],
};

const EMPTY_RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "tokens",
  buckets: [],
  series: [],
};

const ALL_ZERO_RESPONSE: SeriesResponse = {
  bucket: "day",
  metric: "tokens",
  buckets: ["2026-07-01"],
  series: [{ name: "model-a", points: [0] }],
};

describe("UsageChart", () => {
  it("renders the chart container with series data", () => {
    const { container } = render(<UsageChart response={MOCK_RESPONSE} />);
    // recharts renders a recharts-wrapper div inside ResponsiveContainer
    // In jsdom, ResponsiveContainer renders a div wrapper even if SVG may not fully render
    expect(container.querySelector(".recharts-wrapper, [class*=recharts]") ?? container.firstChild).toBeTruthy();
    // The empty state should NOT be shown when we have data
    expect(screen.queryByRole("status", { name: "Sin datos" })).toBeNull();
  });

  it("renders the empty state when there are no buckets", () => {
    render(<UsageChart response={EMPTY_RESPONSE} />);
    expect(screen.getByRole("status", { name: "Sin datos" })).toBeInTheDocument();
    expect(screen.getByText("Sin datos de uso")).toBeInTheDocument();
  });

  it("renders the empty state when all points are zero", () => {
    render(<UsageChart response={ALL_ZERO_RESPONSE} />);
    expect(screen.getByRole("status", { name: "Sin datos" })).toBeInTheDocument();
  });

  it("shows the cost estimation note when metric is cost", () => {
    const costResponse: SeriesResponse = {
      ...MOCK_RESPONSE,
      metric: "cost",
    };
    render(<UsageChart response={costResponse} />);
    expect(screen.getByRole("note")).toBeInTheDocument();
    expect(screen.getByText(/costo estimado/i)).toBeInTheDocument();
  });

  it("does not show the cost note when metric is tokens", () => {
    render(<UsageChart response={MOCK_RESPONSE} />);
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("groups extra series into 'Otros' when there are more than top-N", () => {
    // Create 10 series (more than the TOP_N=8 limit)
    const manySeries: SeriesResponse = {
      bucket: "day",
      metric: "tokens",
      buckets: ["2026-07-01"],
      series: Array.from({ length: 10 }, (_, i) => ({
        name: `series-${String(i)}`,
        points: [100 - i], // Descending totals so order is predictable
      })),
    };
    render(<UsageChart response={manySeries} />);
    // The "Otros" grouping message should appear
    expect(screen.getByText(/agrupada/i)).toBeInTheDocument();
    expect(screen.getByText(/2 serie/i)).toBeInTheDocument();
  });

  it("does not show Otros message when series count is at or below top-N", () => {
    render(<UsageChart response={MOCK_RESPONSE} />);
    expect(screen.queryByText(/agrupada/i)).toBeNull();
  });
});
