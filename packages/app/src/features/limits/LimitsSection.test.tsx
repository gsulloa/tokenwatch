import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { LimitsSection } from "./LimitsSection";
import type { LimitsSnapshot, LimitsWindow } from "./types";

function win(utilization: number, hoursAhead: number): LimitsWindow {
  return {
    label: null,
    utilization,
    resetsAt: new Date(Date.now() + hoursAhead * 3_600_000).toISOString(),
  };
}

function makeSnapshot(overrides: Partial<LimitsSnapshot> = {}): LimitsSnapshot {
  return {
    session: null,
    weekly: null,
    weeklyByModel: [],
    fetchedAt: new Date().toISOString(),
    status: { kind: "ok" },
    ...overrides,
  };
}

describe("LimitsSection", () => {
  it("shows 'Sin sesión activa' and the weekly gauge when session is null", () => {
    render(<LimitsSection snapshot={makeSnapshot({ weekly: win(56, 48) })} />);
    expect(screen.getByText("Sin sesión activa")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Semana" })).toBeInTheDocument();
  });

  it("does not show 'Sin sesión activa' when both windows exist", () => {
    render(
      <LimitsSection
        snapshot={makeSnapshot({ session: win(20, 3), weekly: win(56, 48) })}
      />,
    );
    expect(screen.queryByText("Sin sesión activa")).not.toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Sesión 5h" })).toBeInTheDocument();
  });

  it("shows the empty message when there are no windows", () => {
    render(<LimitsSection snapshot={makeSnapshot()} />);
    expect(screen.getByText("Sin datos de límites disponibles")).toBeInTheDocument();
    expect(screen.queryByText("Sin sesión activa")).not.toBeInTheDocument();
  });
});
