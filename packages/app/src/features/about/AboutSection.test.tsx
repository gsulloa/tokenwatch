import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AboutSectionView } from "./AboutSection";
import type { AboutSectionViewProps } from "./AboutSection";
import type { UseAppUpdateResult } from "@/features/updates/types";

const noop = () => {};

function makeUpdateState(
  overrides: Partial<UseAppUpdateResult> = {},
): UseAppUpdateResult {
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

function makeProps(
  overrides: Partial<AboutSectionViewProps> = {},
): AboutSectionViewProps {
  return {
    version: "1.0.0",
    updateState: makeUpdateState(),
    onOpenChangelog: noop,
    ...overrides,
  };
}

describe("AboutSectionView", () => {
  it("shows the current version formatted as vX.Y.Z", () => {
    render(<AboutSectionView {...makeProps({ version: "1.2.3" })} />);
    expect(screen.getByText("v1.2.3")).toBeInTheDocument();
  });

  it("shows placeholder v— when version is null", () => {
    render(<AboutSectionView {...makeProps({ version: null })} />);
    expect(screen.getByText("v—")).toBeInTheDocument();
  });

  it("idle state: shows 'App al día' and 'Buscar actualizaciones'", () => {
    render(<AboutSectionView {...makeProps()} />);
    expect(screen.getByText(/app al día/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /buscar actualizaciones/i }),
    ).toBeInTheDocument();
  });

  it("available state: shows update version and Instalar button", () => {
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({ status: "available", version: "2.0.0" }),
        })}
      />,
    );
    expect(screen.getByText(/actualización disponible.*v2\.0\.0/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /instalar/i })).toBeInTheDocument();
  });

  it("available state: calls installNow when Instalar is clicked", () => {
    const installNow = vi.fn();
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({
            status: "available",
            version: "2.0.0",
            installNow,
          }),
        })}
      />,
    );
    screen.getByRole("button", { name: /instalar/i }).click();
    expect(installNow).toHaveBeenCalledTimes(1);
  });

  it("ready state: shows 'Reiniciar para aplicar' button", () => {
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({ status: "ready" }),
        })}
      />,
    );
    expect(
      screen.getByRole("button", { name: /reiniciar para aplicar/i }),
    ).toBeInTheDocument();
  });

  it("ready state: calls relaunchApp when Reiniciar button is clicked", () => {
    const relaunchApp = vi.fn();
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({ status: "ready", relaunchApp }),
        })}
      />,
    );
    screen.getByRole("button", { name: /reiniciar para aplicar/i }).click();
    expect(relaunchApp).toHaveBeenCalledTimes(1);
  });

  it("checking state: shows 'Buscando actualizaciones' and no action buttons", () => {
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({ status: "checking" }),
        })}
      />,
    );
    expect(screen.getByText(/buscando actualizaciones/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /instalar/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /reiniciar/i })).toBeNull();
    // No 'Buscar actualizaciones' while busy
    expect(
      screen.queryByRole("button", { name: /buscar actualizaciones/i }),
    ).toBeNull();
  });

  it("downloading state: shows progress and no action buttons", () => {
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({ status: "downloading", progress: 55 }),
        })}
      />,
    );
    expect(screen.getByText(/instalando.*55%/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /instalar/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /reiniciar/i })).toBeNull();
  });

  it("calls onOpenChangelog when 'Ver changelog' button is clicked", () => {
    const onOpenChangelog = vi.fn();
    render(<AboutSectionView {...makeProps({ onOpenChangelog })} />);
    fireEvent.click(screen.getByRole("button", { name: /ver changelog/i }));
    expect(onOpenChangelog).toHaveBeenCalledTimes(1);
  });

  it("error state: shows error message", () => {
    render(
      <AboutSectionView
        {...makeProps({
          updateState: makeUpdateState({
            status: "error",
            error: "Network failure",
          }),
        })}
      />,
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/network failure/i)).toBeInTheDocument();
  });
});
