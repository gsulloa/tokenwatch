/**
 * Pure markdown parser for the Keep-a-Changelog subset.
 * Supports: headings (## / ###), unordered lists (- ), paragraphs,
 * bold (**), italic (*), inline code (`), links ([text](url)).
 * Emits typed node trees — no React import, no external dependencies.
 */

// ── Inline nodes ─────────────────────────────────────────────────────────────

export type InlineText = { type: "text"; value: string };
export type InlineStrong = { type: "strong"; children: InlineNode[] };
export type InlineEm = { type: "em"; children: InlineNode[] };
export type InlineCode = { type: "code"; value: string };
export type InlineLink = {
  type: "link";
  url: string;
  children: InlineNode[];
};

export type InlineNode =
  | InlineText
  | InlineStrong
  | InlineEm
  | InlineCode
  | InlineLink;

// ── Block nodes ──────────────────────────────────────────────────────────────

export type MdHeading = {
  type: "heading";
  level: 2 | 3;
  children: InlineNode[];
};

export type MdListItem = {
  type: "listItem";
  children: InlineNode[];
  sublist?: MdList;
};

export type MdList = {
  type: "list";
  items: MdListItem[];
};

export type MdParagraph = {
  type: "paragraph";
  children: InlineNode[];
};

export type MdBlock = MdHeading | MdList | MdParagraph;

// ── ALLOWED link schemes ──────────────────────────────────────────────────────

const ALLOWED_SCHEMES = ["http:", "https:", "mailto:"];

function isSafeUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return ALLOWED_SCHEMES.includes(parsed.protocol);
  } catch {
    return false;
  }
}

// ── Inline parser ─────────────────────────────────────────────────────────────

/**
 * Parse inline markdown within a single line of text.
 * Handles: **bold**, *italic*, `code`, [text](url).
 * Raw HTML (<...>) is left as plain text (React will escape it).
 */
export function parseInline(text: string): InlineNode[] {
  const nodes: InlineNode[] = [];

  // Regex tokens in priority order:
  //   1. **bold**
  //   2. *italic*
  //   3. `code`
  //   4. [text](url)
  const TOKEN_RE =
    /\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\)/gs;

  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = TOKEN_RE.exec(text)) !== null) {
    // Capture any plain text before this match
    if (match.index > lastIndex) {
      nodes.push({ type: "text", value: text.slice(lastIndex, match.index) });
    }

    const boldContent = match[1];
    const emContent = match[2];
    const codeContent = match[3];
    const linkText = match[4];
    const linkUrl = match[5];

    if (boldContent !== undefined) {
      // **bold**
      nodes.push({ type: "strong", children: parseInline(boldContent) });
    } else if (emContent !== undefined) {
      // *italic*
      nodes.push({ type: "em", children: parseInline(emContent) });
    } else if (codeContent !== undefined) {
      // `code`
      nodes.push({ type: "code", value: codeContent });
    } else if (linkText !== undefined && linkUrl !== undefined) {
      // [text](url)
      if (isSafeUrl(linkUrl)) {
        nodes.push({
          type: "link",
          url: linkUrl,
          children: parseInline(linkText),
        });
      } else {
        // Unsafe scheme → render the whole thing as plain text
        nodes.push({ type: "text", value: match[0] });
      }
    }

    lastIndex = match.index + match[0].length;
  }

  // Remaining plain text after last token
  if (lastIndex < text.length) {
    nodes.push({ type: "text", value: text.slice(lastIndex) });
  }

  return nodes;
}

// ── Helpers for safe line access ──────────────────────────────────────────────

function getLine(lines: string[], i: number): string | undefined {
  return lines[i];
}

// ── Block parser ─────────────────────────────────────────────────────────────

/**
 * Parse a markdown source string into an array of block nodes.
 * Supports Keep-a-Changelog subset: headings (## / ###),
 * unordered lists with nesting (2+ leading spaces = sublist), paragraphs.
 */
export function parseMarkdown(source: string): MdBlock[] {
  const blocks: MdBlock[] = [];
  const lines = source.split("\n");

  let i = 0;

  while (i < lines.length) {
    const line = getLine(lines, i);
    if (line === undefined) break;

    // Skip blank lines
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Heading ## (level 2)
    if (line.startsWith("## ")) {
      const text = line.slice(3).trim();
      blocks.push({ type: "heading", level: 2, children: parseInline(text) });
      i++;
      continue;
    }

    // Heading ### (level 3)
    if (line.startsWith("### ")) {
      const text = line.slice(4).trim();
      blocks.push({ type: "heading", level: 3, children: parseInline(text) });
      i++;
      continue;
    }

    // Unordered list: a line that starts with "- " (top-level, no leading spaces)
    if (/^- /.test(line)) {
      const list = parseList(lines, i, 0);
      blocks.push(list.list);
      i = list.nextIndex;
      continue;
    }

    // Paragraph: consecutive non-empty lines that aren't headings or list items
    const paragraphLines: string[] = [];
    while (i < lines.length) {
      const cur = getLine(lines, i);
      if (
        cur === undefined ||
        cur.trim() === "" ||
        cur.startsWith("## ") ||
        cur.startsWith("### ") ||
        /^- /.test(cur)
      ) {
        break;
      }
      paragraphLines.push(cur.trim());
      i++;
    }

    if (paragraphLines.length > 0) {
      const joined = paragraphLines.join(" ");
      blocks.push({ type: "paragraph", children: parseInline(joined) });
    }
  }

  return blocks;
}

// ── List parser (recursive for nesting) ──────────────────────────────────────

/**
 * Parse a list starting at `startIndex` with the given `baseIndent`.
 * Returns the parsed MdList and the next line index after the list.
 */
function parseList(
  lines: string[],
  startIndex: number,
  baseIndent: number,
): { list: MdList; nextIndex: number } {
  const items: MdListItem[] = [];
  let i = startIndex;

  while (i < lines.length) {
    const line = getLine(lines, i);
    if (line === undefined) break;

    // Blank line ends the current list level
    if (line.trim() === "") {
      break;
    }

    const indent = getIndent(line);
    const trimmed = line.trimStart();

    // If this line is a list item at the current base indent level
    if (indent === baseIndent && trimmed.startsWith("- ")) {
      const itemText = trimmed.slice(2);
      const item: MdListItem = { type: "listItem", children: parseInline(itemText) };
      i++;

      // Check if the next non-blank line is a nested list item (more indented)
      const nextLine = getLine(lines, i);
      if (
        i < lines.length &&
        nextLine !== undefined &&
        nextLine.trim() !== "" &&
        getIndent(nextLine) > baseIndent &&
        nextLine.trimStart().startsWith("- ")
      ) {
        const nested = parseList(lines, i, getIndent(nextLine));
        item.sublist = nested.list;
        i = nested.nextIndex;
      }

      items.push(item);
    } else if (indent > baseIndent && trimmed.startsWith("- ")) {
      // Deeper list item found unexpectedly at this level — stop and let parent handle
      break;
    } else {
      // Line is not a list item at this level — stop
      break;
    }
  }

  return { list: { type: "list", items }, nextIndex: i };
}

function getIndent(line: string): number {
  let count = 0;
  for (const ch of line) {
    if (ch === " ") count++;
    else if (ch === "\t") count += 2;
    else break;
  }
  return count;
}
