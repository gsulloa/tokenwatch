import { render, screen } from "@testing-library/react";
import { App } from "@/app/App";
import { APP_DISPLAY_NAME } from "@/platform/app-identity";

describe("App", () => {
  it("renders the application name", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: APP_DISPLAY_NAME }),
    ).toBeInTheDocument();
  });
});
