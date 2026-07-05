import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ChartControls } from "./ChartControls";
import type { ChartControlsValue } from "./ChartControls";

const DEFAULT_VALUE: ChartControlsValue = {
  bucket: "day",
  metric: "tokens",
  seriesBy: "model",
  dateFilter: { preset: "all" },
};

describe("ChartControls", () => {
  it("renders all three control groups", () => {
    render(<ChartControls value={DEFAULT_VALUE} onChange={() => {}} />);

    expect(screen.getByRole("group", { name: "Período" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Métrica" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Series" })).toBeInTheDocument();
  });

  it("calls onChange with updated bucket when a bucket option is clicked", () => {
    const onChange = vi.fn();
    render(<ChartControls value={DEFAULT_VALUE} onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "Semana" }));

    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith({
      ...DEFAULT_VALUE,
      bucket: "week",
    });
  });

  it("calls onChange with updated metric when a metric option is clicked", () => {
    const onChange = vi.fn();
    render(<ChartControls value={DEFAULT_VALUE} onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "Costo" }));

    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith({
      ...DEFAULT_VALUE,
      metric: "cost",
    });
  });

  it("calls onChange with updated seriesBy when a seriesBy option is clicked", () => {
    const onChange = vi.fn();
    render(<ChartControls value={DEFAULT_VALUE} onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "Proyecto" }));

    expect(onChange).toHaveBeenCalledOnce();
    expect(onChange).toHaveBeenCalledWith({
      ...DEFAULT_VALUE,
      seriesBy: "project",
    });
  });

  it("shows the currently selected values with aria-pressed=true", () => {
    render(<ChartControls value={DEFAULT_VALUE} onChange={() => {}} />);

    const diaButton = screen.getByRole("button", { name: "Día" });
    const semanaButton = screen.getByRole("button", { name: "Semana" });
    expect(diaButton).toHaveAttribute("aria-pressed", "true");
    expect(semanaButton).toHaveAttribute("aria-pressed", "false");
  });
});
