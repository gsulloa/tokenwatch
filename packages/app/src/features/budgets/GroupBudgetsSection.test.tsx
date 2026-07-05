import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { GroupBudgetsSection } from "./GroupBudgetsSection";
import type { GroupBudgetsSnapshot } from "./types";

// Mock Tauri APIs (safeguard for any transitive import)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// ── Snapshot builders ─────────────────────────────────────────────────────────

function makeSnapshot(
  overrides: Partial<GroupBudgetsSnapshot> = {},
): GroupBudgetsSnapshot {
  return {
    rows: [],
    windowStart: "2026-07-05T10:00:00.000Z",
    origin: "session",
    ...overrides,
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("GroupBudgetsSection", () => {
  // ── Loading state ─────────────────────────────────────────────────────────

  it("shows 'Cargando…' when loading and no snapshot", () => {
    render(<GroupBudgetsSection snapshot={null} loading={true} />);
    expect(screen.getByRole("status")).toHaveTextContent("Cargando…");
  });

  // ── No groups: section omitted ────────────────────────────────────────────

  it("renders nothing when snapshot has no defined groups (only otros)", () => {
    const { container } = render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: null,
              name: "otros",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 1.5,
              localCostSharePct: 100,
              sessionWeightedPct: 42.0,
              measuredValue: null,
            },
          ],
        })}
        loading={false}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders nothing when snapshot is null and not loading", () => {
    const { container } = render(
      <GroupBudgetsSection snapshot={null} loading={false} />,
    );
    expect(container.firstChild).toBeNull();
  });

  // ── Empty state: groups exist but total == 0 ──────────────────────────────

  it("shows 'Sin consumo en esta ventana de 5h' when all costs are 0", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Cliente A",
              budgetBasis: "share",
              budgetValue: 30,
              windowCostUsd: 0,
              localCostSharePct: 0,
              sessionWeightedPct: 0,
              measuredValue: 0,
            },
          ],
        })}
        loading={false}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Sin consumo en esta ventana de 5h",
    );
  });

  // ── Session origin: share cap shows "% sesión" (weighted) ────────────────

  it("shows session-weighted secondary '~N% sesión' in session mode", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Cliente A",
              budgetBasis: "share",
              budgetValue: 30,
              windowCostUsd: 2.5,
              localCostSharePct: 87,
              sessionWeightedPct: 29.58,
              measuredValue: 29.58,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );

    // Group name and leading cost
    expect(screen.getByText("Cliente A")).toBeInTheDocument();
    expect(screen.getByText("$2.50")).toBeInTheDocument();
    // Secondary shows weighted "% sesión", NOT "% costo local"
    expect(screen.getByText(/~30% sesión/)).toBeInTheDocument();
    // CapMeter readout uses sessionWeightedPct: "30% / 30% sesión (est.)"
    expect(screen.getByText(/30% \/ 30% sesión \(est\.\)/)).toBeInTheDocument();
  });

  it("renders the 87%×34%→~30% scenario under a 30% share cap (no red)", () => {
    // local share = 87%, session.utilization = 34% → weighted = 29.58% ≈ 30% → under cap
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "MiGrupo",
              budgetBasis: "share",
              budgetValue: 30,
              windowCostUsd: 8.7,
              localCostSharePct: 87,
              sessionWeightedPct: 29.58,
              measuredValue: 29.58,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );

    // Secondary is the weighted value, not raw local share
    expect(screen.getByText(/~30% sesión/)).toBeInTheDocument();
    // CapMeter readout: "30% / 30% sesión (est.)" — under cap, no danger color
    const readout = screen.getByText(/30% \/ 30% sesión \(est\.\)/);
    expect(readout).toBeInTheDocument();
    // NOT over cap: readout should not have danger color
    expect(readout).not.toHaveStyle({ color: "var(--danger)" });
  });

  // ── Rolling origin: shows "% costo local" and no share cap meter ─────────

  it("shows 'N% costo local' secondary in rolling mode", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Proyecto X",
              budgetBasis: "share",
              budgetValue: 30,
              windowCostUsd: 5.0,
              localCostSharePct: 87,
              sessionWeightedPct: null, // rolling — no session
              measuredValue: null, // rolling — no share-cap measured value
            },
          ],
          origin: "rolling",
        })}
        loading={false}
      />,
    );

    expect(screen.getByText("Proyecto X")).toBeInTheDocument();
    // Rolling: secondary is "% costo local", not "% sesión"
    expect(screen.getByText(/87% costo local/)).toBeInTheDocument();
    // No CapMeter for share in rolling mode (measuredValue is null)
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  // ── With cap (usd) ────────────────────────────────────────────────────────

  it("renders group row with usd cap and shows CapMeter $ readout", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 2,
              name: "Cliente B",
              budgetBasis: "usd",
              budgetValue: 2.0,
              windowCostUsd: 2.1,
              localCostSharePct: 60,
              sessionWeightedPct: 25.2,
              measuredValue: 2.1,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );

    expect(screen.getByText("Cliente B")).toBeInTheDocument();
    // CapMeter readout: "$2.10 / $2.00"
    expect(screen.getByText("$2.10 / $2.00")).toBeInTheDocument();
  });

  // ── Without cap ───────────────────────────────────────────────────────────

  it("renders group row without cap (no CapMeter progressbar)", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 3,
              name: "Interno",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 1.0,
              localCostSharePct: 25,
              sessionWeightedPct: 10.5,
              measuredValue: null,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );

    expect(screen.getByText("Interno")).toBeInTheDocument();
    // No progressbar (CapMeter) should be present
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  // ── "otros" row last ──────────────────────────────────────────────────────

  it("renders 'otros' row last after named groups", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Cliente A",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 3.0,
              localCostSharePct: 60,
              sessionWeightedPct: 25.2,
              measuredValue: null,
            },
            {
              groupId: null,
              name: "otros",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 2.0,
              localCostSharePct: 40,
              sessionWeightedPct: 16.8,
              measuredValue: null,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );

    const names = screen
      .getAllByTitle(/./)
      .map((el) => el.textContent ?? "");
    const clienteIdx = names.findIndex((n) => n.includes("Cliente A"));
    const otrosIdx = names.findIndex((n) => n.includes("otros"));
    expect(clienteIdx).toBeLessThan(otrosIdx);
  });

  // ── Rolling origin caption ────────────────────────────────────────────────

  it("shows rolling subtitle when origin is rolling", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Proyecto X",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 1.0,
              localCostSharePct: 100,
              sessionWeightedPct: null,
              measuredValue: null,
            },
          ],
          origin: "rolling",
        })}
        loading={false}
      />,
    );
    expect(
      screen.getByText(/costo local · ventana móvil de 5h/),
    ).toBeInTheDocument();
  });

  it("shows session subtitle when origin is session", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Proyecto X",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 1.0,
              localCostSharePct: 100,
              sessionWeightedPct: 42.0,
              measuredValue: null,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );
    expect(screen.getByText(/estimado sobre tu sesión de 5h/)).toBeInTheDocument();
  });

  // ── Section header ────────────────────────────────────────────────────────

  it("renders the section heading 'Uso por grupo'", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 1,
              name: "Cliente A",
              budgetBasis: null,
              budgetValue: null,
              windowCostUsd: 1.5,
              localCostSharePct: 100,
              sessionWeightedPct: 42.0,
              measuredValue: null,
            },
          ],
        })}
        loading={false}
      />,
    );
    expect(
      screen.getByRole("heading", { name: /uso por grupo/i }),
    ).toBeInTheDocument();
  });

  // ── Over-cap: bar clamp + red text ────────────────────────────────────────

  it("shows $ readout in red when over usd cap", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 2,
              name: "Cliente B",
              budgetBasis: "usd",
              budgetValue: 2.0,
              windowCostUsd: 2.5,
              localCostSharePct: 70,
              sessionWeightedPct: 29.4,
              measuredValue: 2.5,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );
    // Readout "$2.50 / $2.00"
    const readout = screen.getByText("$2.50 / $2.00");
    expect(readout).toBeInTheDocument();
    // The readout span should have danger color style (over cap)
    expect(readout).toHaveStyle({ color: "var(--danger)" });
  });

  it("shows % sesión readout in red when over share cap in session mode", () => {
    render(
      <GroupBudgetsSection
        snapshot={makeSnapshot({
          rows: [
            {
              groupId: 3,
              name: "Cliente C",
              budgetBasis: "share",
              budgetValue: 30,
              windowCostUsd: 3.0,
              localCostSharePct: 75,
              sessionWeightedPct: 31.5, // over 30% cap
              measuredValue: 31.5,
            },
          ],
          origin: "session",
        })}
        loading={false}
      />,
    );
    // Readout "32% / 30% sesión (est.)"
    const readout = screen.getByText(/32% \/ 30% sesión \(est\.\)/);
    expect(readout).toBeInTheDocument();
    // Over cap: danger color
    expect(readout).toHaveStyle({ color: "var(--danger)" });
  });
});
