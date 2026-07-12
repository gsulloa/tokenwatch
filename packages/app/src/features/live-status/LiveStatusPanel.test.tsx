import { render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { LiveStatusPanel } from "./LiveStatusPanel";

// Mock Tauri APIs so the panel's hooks degrade to no-op in jsdom, mirroring
// App.test.tsx / GroupBudgetsSection.test.tsx conventions.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("LiveStatusPanel", () => {
  it("renders without throwing and shows the limits section's empty state", async () => {
    render(<LiveStatusPanel />);

    // With invoke mocked to resolve null, useLimits never sets a snapshot,
    // so LimitsSection settles on its "Sin datos de límites" empty state
    // once the mocked async invoke() promise resolves.
    expect(
      await screen.findByText("Sin datos de límites"),
    ).toBeInTheDocument();

    // TodayByProjectList is always rendered too.
    expect(
      screen.getByRole("heading", { name: /hoy por proyecto/i }),
    ).toBeInTheDocument();
  });
});
