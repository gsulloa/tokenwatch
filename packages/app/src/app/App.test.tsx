import { render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { App } from "@/app/App";
import { APP_DISPLAY_NAME } from "@/platform/app-identity";

// Mock Tauri APIs so App doesn't crash in jsdom
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("App", () => {
  it("renders the application name", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: APP_DISPLAY_NAME }),
    ).toBeInTheDocument();
  });
});
