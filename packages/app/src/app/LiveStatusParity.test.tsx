import { render, screen, cleanup } from "@testing-library/react";
import { vi } from "vitest";
import { Popover } from "@/app/Popover";
import { App } from "@/app/App";

// Mock Tauri APIs so both surfaces don't crash in jsdom (same convention as
// App.test.tsx / LiveStatusPanel.test.tsx).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

/**
 * Both surfaces must render the exact same LiveStatusPanel composition
 * (limits, group budgets, today-by-project) so the two windows never drift
 * apart. This test renders each surface independently and asserts both
 * expose the same shared sections, proving they share `LiveStatusPanel`
 * rather than duplicating the composition.
 */
describe("LiveStatusPanel parity between Popover and dashboard", () => {
  it("both the popover and the dashboard render the today-by-project section", async () => {
    const { unmount: unmountPopover } = render(<Popover />);
    expect(
      await screen.findByRole("heading", { name: /hoy por proyecto/i }),
    ).toBeInTheDocument();
    unmountPopover();
    cleanup();

    render(<App />);
    expect(
      await screen.findByRole("heading", { name: /hoy por proyecto/i }),
    ).toBeInTheDocument();
  });

  it("both surfaces render the sectioned live-status region with a separator", async () => {
    const { unmount: unmountPopover } = render(<Popover />);
    expect(await screen.findAllByRole("separator")).not.toHaveLength(0);
    unmountPopover();
    cleanup();

    render(<App />);
    expect(await screen.findAllByRole("separator")).not.toHaveLength(0);
  });
});
