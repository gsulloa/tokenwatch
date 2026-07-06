import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Markdown } from "./Markdown";

// ── Helpers ───────────────────────────────────────────────────────────────────

function renderMd(source: string) {
  return render(<Markdown source={source} />);
}

// ── Basic element rendering ───────────────────────────────────────────────────

describe("Markdown component — elements", () => {
  it("renders an ## heading as h2", () => {
    renderMd("## Release Notes");
    expect(screen.getByRole("heading", { level: 2 })).toBeInTheDocument();
    expect(screen.getByText("Release Notes")).toBeInTheDocument();
  });

  it("renders a ### heading as h3", () => {
    renderMd("### Added");
    expect(screen.getByRole("heading", { level: 3 })).toBeInTheDocument();
    expect(screen.getByText("Added")).toBeInTheDocument();
  });

  it("renders bullet list items as <li>", () => {
    renderMd("- first\n- second\n- third");
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(3);
    expect(screen.getByText("first")).toBeInTheDocument();
  });

  it("renders a paragraph", () => {
    renderMd("This is a paragraph.");
    expect(screen.getByText("This is a paragraph.")).toBeInTheDocument();
  });

  it("renders **bold** as <strong>", () => {
    renderMd("**important**");
    const strong = document.querySelector("strong");
    expect(strong).not.toBeNull();
    expect(strong?.textContent).toBe("important");
  });

  it("renders *italic* as <em>", () => {
    renderMd("*note*");
    const em = document.querySelector("em");
    expect(em).not.toBeNull();
    expect(em?.textContent).toBe("note");
  });

  it("renders `code` as <code>", () => {
    renderMd("`npm install`");
    const code = document.querySelector("code");
    expect(code).not.toBeNull();
    expect(code?.textContent).toBe("npm install");
  });

  it("renders nested list items in a nested <ul>", () => {
    renderMd("- parent\n  - child one\n  - child two");
    const lists = document.querySelectorAll("ul");
    expect(lists.length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("child one")).toBeInTheDocument();
  });

  it("returns null for empty source", () => {
    const { container } = renderMd("");
    expect(container.firstChild).toBeNull();
  });

  it("returns null for whitespace-only source", () => {
    const { container } = renderMd("   \n\n  ");
    expect(container.firstChild).toBeNull();
  });
});

// ── Link handling ─────────────────────────────────────────────────────────────

describe("Markdown component — links", () => {
  let windowOpenSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    windowOpenSpy = vi.spyOn(window, "open").mockImplementation(() => null);
  });

  afterEach(() => {
    windowOpenSpy.mockRestore();
  });

  it("renders a valid link as <a> with target=_blank and rel=noopener noreferrer", () => {
    renderMd("[docs](https://example.com)");
    const link = screen.getByRole("link", { name: "docs" });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "https://example.com");
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
  });

  it("calls window.open with the URL when a link is clicked", () => {
    renderMd("[docs](https://example.com)");
    const link = screen.getByRole("link", { name: "docs" });
    fireEvent.click(link);
    expect(windowOpenSpy).toHaveBeenCalledWith(
      "https://example.com",
      "_blank",
      "noopener,noreferrer",
    );
  });

  it("does NOT create a link for javascript: scheme — renders as text", () => {
    renderMd("[click](javascript:alert(1))");
    expect(screen.queryByRole("link")).toBeNull();
    // The raw markdown text should appear as plain text
    expect(screen.getByText(/\[click\]/)).toBeInTheDocument();
  });
});

// ── Security: XSS / HTML injection ───────────────────────────────────────────

describe("Markdown component — security", () => {
  it("does NOT create an executable <script> element for <script> in source", () => {
    renderMd("<script>alert(1)</script>");
    // React escapes HTML — no real script element should exist
    const scripts = document.querySelectorAll("script");
    // Filter out Vite/test infrastructure scripts — look only for ones with inline alert
    const injectedScript = Array.from(scripts).find((s) =>
      s.textContent?.includes("alert(1)"),
    );
    expect(injectedScript).toBeUndefined();
    // The text content should be visible as escaped text
    expect(screen.getByText(/<script>/)).toBeInTheDocument();
  });

  it("does NOT create an <img> with onerror for <img onerror=...> in source", () => {
    renderMd('<img src=x onerror=alert(1)>');
    // No img element with an onerror attribute should be in the DOM
    const imgs = document.querySelectorAll("img[onerror]");
    expect(imgs).toHaveLength(0);
  });
});
