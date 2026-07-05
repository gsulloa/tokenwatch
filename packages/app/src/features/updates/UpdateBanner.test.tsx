import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { UpdateBannerView } from "./UpdateBanner";
import type { UseAppUpdateResult } from "./types";

const noop = () => {};

function makeProps(overrides: Partial<UseAppUpdateResult>): UseAppUpdateResult {
  return {
    status: "idle",
    version: null,
    notes: null,
    progress: null,
    error: null,
    checkNow: noop,
    installNow: noop,
    relaunchApp: noop,
    ...overrides,
  };
}

describe("UpdateBannerView", () => {
  it("renders nothing when status is idle", () => {
    const { container } = render(<UpdateBannerView {...makeProps({ status: "idle" })} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders checking state with spinner text and no action buttons", () => {
    render(<UpdateBannerView {...makeProps({ status: "checking" })} />);
    expect(screen.getByText(/buscando actualizaciones/i)).toBeInTheDocument();
    // Action buttons (Instalar, Reiniciar) should NOT be present while checking
    expect(screen.queryByRole("button", { name: /instalar/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /reiniciar/i })).toBeNull();
  });

  it("renders available state with version and install button", () => {
    render(
      <UpdateBannerView
        {...makeProps({ status: "available", version: "1.5.0" })}
      />,
    );
    expect(screen.getByText(/actualización disponible.*v1\.5\.0/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /instalar/i })).toBeInTheDocument();
  });

  it("available state: calls installNow when install button is clicked", () => {
    const installNow = vi.fn();
    render(
      <UpdateBannerView
        {...makeProps({ status: "available", version: "1.5.0", installNow })}
      />,
    );
    screen.getByRole("button", { name: /instalar/i }).click();
    expect(installNow).toHaveBeenCalledTimes(1);
  });

  it("renders downloading state with progress", () => {
    render(
      <UpdateBannerView {...makeProps({ status: "downloading", progress: 42 })} />,
    );
    expect(screen.getByText(/instalando.*42%/i)).toBeInTheDocument();
    // Instalar and Reiniciar buttons should not be present
    expect(screen.queryByRole("button", { name: /instalar/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /reiniciar/i })).toBeNull();
  });

  it("renders ready state with relaunch button", () => {
    render(<UpdateBannerView {...makeProps({ status: "ready" })} />);
    expect(screen.getByRole("button", { name: /reiniciar para aplicar/i })).toBeInTheDocument();
  });

  it("ready state: calls relaunchApp when relaunch button is clicked", () => {
    const relaunchApp = vi.fn();
    render(
      <UpdateBannerView {...makeProps({ status: "ready", relaunchApp })} />,
    );
    screen.getByRole("button", { name: /reiniciar para aplicar/i }).click();
    expect(relaunchApp).toHaveBeenCalledTimes(1);
  });

  it("renders error state with error message", () => {
    render(
      <UpdateBannerView
        {...makeProps({ status: "error", error: "Network failure" })}
      />,
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/network failure/i)).toBeInTheDocument();
  });

  it("shows Buscar actualizaciones button in non-busy states", () => {
    render(
      <UpdateBannerView {...makeProps({ status: "available", version: "2.0.0" })} />,
    );
    expect(
      screen.getByRole("button", { name: /buscar actualizaciones/i }),
    ).toBeInTheDocument();
  });

  it("does NOT show Buscar actualizaciones while checking", () => {
    render(<UpdateBannerView {...makeProps({ status: "checking" })} />);
    expect(
      screen.queryByRole("button", { name: /buscar actualizaciones/i }),
    ).toBeNull();
  });

  it("does NOT show Buscar actualizaciones while downloading", () => {
    render(<UpdateBannerView {...makeProps({ status: "downloading", progress: 10 })} />);
    expect(
      screen.queryByRole("button", { name: /buscar actualizaciones/i }),
    ).toBeNull();
  });
});
