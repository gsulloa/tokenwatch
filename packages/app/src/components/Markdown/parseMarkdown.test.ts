import { describe, it, expect } from "vitest";
import { parseMarkdown, parseInline } from "./parseMarkdown";
import type { MdBlock, InlineNode } from "./parseMarkdown";

// ── Helpers ───────────────────────────────────────────────────────────────────

function heading(level: 2 | 3, ...children: InlineNode[]): MdBlock {
  return { type: "heading", level, children };
}

function text(value: string): InlineNode {
  return { type: "text", value };
}

// ── parseInline ───────────────────────────────────────────────────────────────

describe("parseInline", () => {
  it("returns a single text node for plain text", () => {
    expect(parseInline("hello world")).toEqual([text("hello world")]);
  });

  it("parses **bold** into a strong node", () => {
    const nodes = parseInline("say **hello** now");
    expect(nodes).toEqual([
      text("say "),
      { type: "strong", children: [text("hello")] },
      text(" now"),
    ]);
  });

  it("parses *italic* into an em node", () => {
    const nodes = parseInline("say *hello* now");
    expect(nodes).toEqual([
      text("say "),
      { type: "em", children: [text("hello")] },
      text(" now"),
    ]);
  });

  it("parses `code` into a code node", () => {
    const nodes = parseInline("run `npm install` now");
    expect(nodes).toEqual([
      text("run "),
      { type: "code", value: "npm install" },
      text(" now"),
    ]);
  });

  it("parses a valid https link into a link node", () => {
    const nodes = parseInline("see [docs](https://example.com) here");
    expect(nodes).toEqual([
      text("see "),
      {
        type: "link",
        url: "https://example.com",
        children: [text("docs")],
      },
      text(" here"),
    ]);
  });

  it("parses a valid http link", () => {
    const nodes = parseInline("[link](http://example.com)");
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toMatchObject({ type: "link", url: "http://example.com" });
  });

  it("parses a valid mailto link", () => {
    const nodes = parseInline("[mail](mailto:user@example.com)");
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toMatchObject({ type: "link", url: "mailto:user@example.com" });
  });

  it("degrades javascript: scheme links to plain text", () => {
    const raw = "[click](javascript:alert(1))";
    const nodes = parseInline(raw);
    // No link node should be emitted
    expect(nodes.every((n) => n.type !== "link")).toBe(true);
    // All text content combined must contain the key parts of the raw input
    const combined = nodes
      .map((n) => (n.type === "text" ? n.value : ""))
      .join("");
    expect(combined).toContain("[click]");
    expect(combined).toContain("javascript:");
  });

  it("degrades file: scheme links to plain text", () => {
    const raw = "[file](file:///etc/passwd)";
    const nodes = parseInline(raw);
    expect(nodes.every((n) => n.type !== "link")).toBe(true);
    const combined = nodes.map((n) => (n.type === "text" ? n.value : "")).join("");
    expect(combined).toContain("[file]");
    expect(combined).toContain("file:");
  });

  it("treats <script>...</script> as plain text", () => {
    const nodes = parseInline("before <script>alert(1)</script> after");
    // No link, strong, em, code nodes — just plain text
    expect(nodes.every((n) => n.type === "text")).toBe(true);
    const combined = nodes.map((n) => (n.type === "text" ? n.value : "")).join("");
    expect(combined).toContain("<script>");
    expect(combined).toContain("</script>");
  });

  it("treats <img onerror=...> as plain text", () => {
    const src = '<img src=x onerror=alert(1)>';
    const nodes = parseInline(src);
    expect(nodes.every((n) => n.type === "text")).toBe(true);
    const combined = nodes.map((n) => (n.type === "text" ? n.value : "")).join("");
    expect(combined).toContain("<img");
  });

  it("handles multiple inline nodes in sequence", () => {
    const nodes = parseInline("**A** and *B* and `C`");
    expect(nodes).toEqual([
      { type: "strong", children: [text("A")] },
      text(" and "),
      { type: "em", children: [text("B")] },
      text(" and "),
      { type: "code", value: "C" },
    ]);
  });
});

// ── parseMarkdown — headings ──────────────────────────────────────────────────

describe("parseMarkdown — headings", () => {
  it("parses a ## heading as level 2", () => {
    const blocks = parseMarkdown("## Added");
    expect(blocks).toEqual([heading(2, text("Added"))]);
  });

  it("parses a ### heading as level 3", () => {
    const blocks = parseMarkdown("### Fixed");
    expect(blocks).toEqual([heading(3, text("Fixed"))]);
  });

  it("does NOT treat #### as a special heading", () => {
    const blocks = parseMarkdown("#### Nope");
    // Falls through to paragraph
    const b = blocks[0];
    expect(b).toBeDefined();
    expect(b!.type).toBe("paragraph");
  });

  it("includes inline formatting in headings", () => {
    const blocks = parseMarkdown("## **Bold** heading");
    const b = blocks[0];
    expect(b).toBeDefined();
    expect(b).toMatchObject({
      type: "heading",
      level: 2,
      children: [
        { type: "strong", children: [text("Bold")] },
        text(" heading"),
      ],
    });
  });
});

// ── parseMarkdown — paragraphs ────────────────────────────────────────────────

describe("parseMarkdown — paragraphs", () => {
  it("parses a simple paragraph", () => {
    const blocks = parseMarkdown("This is a paragraph.");
    expect(blocks).toEqual([
      { type: "paragraph", children: [text("This is a paragraph.")] },
    ]);
  });

  it("joins consecutive non-blank lines into one paragraph", () => {
    const blocks = parseMarkdown("line one\nline two\nline three");
    expect(blocks).toHaveLength(1);
    const b = blocks[0];
    expect(b).toBeDefined();
    expect(b!.type).toBe("paragraph");
    const p = b as Extract<MdBlock, { type: "paragraph" }>;
    const combined = p.children.map((n) => (n.type === "text" ? n.value : "")).join("");
    expect(combined).toContain("line one");
    expect(combined).toContain("line two");
  });

  it("separates paragraphs with blank lines", () => {
    const blocks = parseMarkdown("First.\n\nSecond.");
    expect(blocks).toHaveLength(2);
    expect(blocks[0]!.type).toBe("paragraph");
    expect(blocks[1]!.type).toBe("paragraph");
  });
});

// ── parseMarkdown — unordered lists ──────────────────────────────────────────

describe("parseMarkdown — unordered lists", () => {
  it("parses a simple bullet list", () => {
    const blocks = parseMarkdown("- item one\n- item two\n- item three");
    expect(blocks).toHaveLength(1);
    const b = blocks[0];
    expect(b).toBeDefined();
    const list = b as Extract<MdBlock, { type: "list" }>;
    expect(list.type).toBe("list");
    expect(list.items).toHaveLength(3);
    expect(list.items[0]!.children).toEqual([text("item one")]);
    expect(list.items[2]!.children).toEqual([text("item three")]);
  });

  it("parses inline formatting inside list items", () => {
    const blocks = parseMarkdown("- **bold** item");
    const list = blocks[0] as Extract<MdBlock, { type: "list" }>;
    expect(list.items[0]!.children).toEqual([
      { type: "strong", children: [text("bold")] },
      text(" item"),
    ]);
  });

  it("parses nested list items (2-space indent)", () => {
    const src = "- parent\n  - child one\n  - child two";
    const blocks = parseMarkdown(src);
    expect(blocks).toHaveLength(1);
    const list = blocks[0] as Extract<MdBlock, { type: "list" }>;
    expect(list.items).toHaveLength(1);
    const firstItem = list.items[0]!;
    expect(firstItem.sublist).toBeDefined();
    expect(firstItem.sublist!.items).toHaveLength(2);
    expect(firstItem.sublist!.items[0]!.children).toEqual([text("child one")]);
  });

  it("handles multiple top-level items with nested items", () => {
    const src = [
      "- alpha",
      "  - alpha-child",
      "- beta",
      "  - beta-child",
    ].join("\n");
    const blocks = parseMarkdown(src);
    const list = blocks[0] as Extract<MdBlock, { type: "list" }>;
    expect(list.items).toHaveLength(2);
    const alphaChildren = list.items[0]!.sublist?.items[0]!.children;
    expect(alphaChildren).toEqual([text("alpha-child")]);
    const betaChildren = list.items[1]!.sublist?.items[0]!.children;
    expect(betaChildren).toEqual([text("beta-child")]);
  });
});

// ── parseMarkdown — mixed content ─────────────────────────────────────────────

describe("parseMarkdown — mixed content", () => {
  it("parses a typical changelog section", () => {
    const src = `## [1.2.0] - 2024-06-01

### Added

- Cool new feature with **bold**
- Another feature

### Fixed

- A bug fix`;

    const blocks = parseMarkdown(src);
    expect(blocks[0]).toMatchObject({ type: "heading", level: 2 });
    expect(blocks[1]).toMatchObject({ type: "heading", level: 3 });
    expect(blocks[2]!.type).toBe("list");
    expect(blocks[3]).toMatchObject({ type: "heading", level: 3 });
    expect(blocks[4]!.type).toBe("list");
  });

  it("ignores extra blank lines between blocks", () => {
    const blocks = parseMarkdown("## H\n\n\n\n- item");
    expect(blocks).toHaveLength(2);
    expect(blocks[0]!.type).toBe("heading");
    expect(blocks[1]!.type).toBe("list");
  });
});

// ── Security: raw HTML treated as text ───────────────────────────────────────

describe("parseMarkdown — security", () => {
  it("treats <script> tags as paragraph text, not executed HTML", () => {
    const blocks = parseMarkdown("<script>alert(1)</script>");
    // Should be a paragraph (or some block), not a special node
    const allTypes = blocks.map((b) => b.type);
    expect(allTypes).not.toContain("heading");
    expect(allTypes).not.toContain("list");
    // The text content should contain the raw <script> marker as a string
    const p = blocks[0] as Extract<MdBlock, { type: "paragraph" }>;
    const raw = p.children.map((n) => (n.type === "text" ? n.value : "")).join("");
    expect(raw).toContain("<script>");
  });

  it("treats <img onerror=...> as text in a list item", () => {
    const blocks = parseMarkdown('- <img src=x onerror=alert(1)>');
    const list = blocks[0] as Extract<MdBlock, { type: "list" }>;
    const itemText = list.items[0]!.children
      .map((n) => (n.type === "text" ? n.value : ""))
      .join("");
    expect(itemText).toContain("<img");
  });
});
