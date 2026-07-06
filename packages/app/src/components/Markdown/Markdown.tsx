/**
 * Markdown component — renders the Keep-a-Changelog subset as React elements.
 * Uses design-system tokens from global.css. No dangerouslySetInnerHTML.
 */

import { parseMarkdown } from "./parseMarkdown";
import type {
  MdBlock,
  MdList,
  MdListItem,
  InlineNode,
} from "./parseMarkdown";

// ── Style tokens (matching existing modal inline-style patterns) ──────────────

const styles = {
  wrapper: {
    display: "flex",
    flexDirection: "column" as const,
    gap: "var(--space-xs)",
    fontFamily: "var(--font-ui)",
    fontSize: 12,
    color: "var(--text)",
  },
  h2: {
    margin: 0,
    fontSize: 14,
    fontWeight: 700 as const,
    color: "var(--text)",
    fontFamily: "var(--font-ui)",
  },
  h3: {
    margin: 0,
    fontSize: 12,
    fontWeight: 600 as const,
    color: "var(--text-muted)",
    textTransform: "uppercase" as const,
    letterSpacing: "0.04em",
    fontFamily: "var(--font-ui)",
  },
  ul: {
    margin: 0,
    paddingLeft: "var(--space-md)",
    display: "flex",
    flexDirection: "column" as const,
    gap: "var(--space-2xs)",
  },
  li: {
    margin: 0,
    color: "var(--text)",
    lineHeight: 1.5,
  },
  p: {
    margin: 0,
    lineHeight: 1.6,
    color: "var(--text)",
  },
  strong: {
    fontWeight: 700 as const,
  },
  em: {
    fontStyle: "italic" as const,
  },
  code: {
    fontFamily: "var(--font-mono)",
    fontSize: "0.9em",
    background: "rgba(255,255,255,0.08)",
    borderRadius: "var(--radius-sm)",
    padding: "1px 4px",
    color: "var(--text)",
  },
  a: {
    color: "var(--accent)",
    textDecoration: "underline",
    cursor: "pointer",
  },
};

// ── Inline renderer ───────────────────────────────────────────────────────────

function renderInline(nodes: InlineNode[], keyPrefix: string): React.ReactNode[] {
  return nodes.map((node, idx) => {
    const key = `${keyPrefix}-${idx}`;
    switch (node.type) {
      case "text":
        return node.value;
      case "strong":
        return (
          <strong key={key} style={styles.strong}>
            {renderInline(node.children, key)}
          </strong>
        );
      case "em":
        return (
          <em key={key} style={styles.em}>
            {renderInline(node.children, key)}
          </em>
        );
      case "code":
        return (
          <code key={key} style={styles.code}>
            {node.value}
          </code>
        );
      case "link": {
        const { url } = node;
        const handleClick = (e: React.MouseEvent) => {
          e.preventDefault();
          window.open(url, "_blank", "noopener,noreferrer");
        };
        return (
          <a
            key={key}
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            style={styles.a}
            onClick={handleClick}
          >
            {renderInline(node.children, key)}
          </a>
        );
      }
    }
  });
}

// ── List renderer ─────────────────────────────────────────────────────────────

function renderList(list: MdList, keyPrefix: string): React.ReactNode {
  return (
    <ul key={keyPrefix} style={styles.ul}>
      {list.items.map((item: MdListItem, idx) => {
        const itemKey = `${keyPrefix}-li-${idx}`;
        return (
          <li key={itemKey} style={styles.li}>
            {renderInline(item.children, `${itemKey}-inline`)}
            {item.sublist
              ? renderList(item.sublist, `${itemKey}-sub`)
              : null}
          </li>
        );
      })}
    </ul>
  );
}

// ── Block renderer ────────────────────────────────────────────────────────────

function renderBlock(block: MdBlock, idx: number): React.ReactNode {
  const key = `block-${idx}`;
  switch (block.type) {
    case "heading":
      if (block.level === 2) {
        return (
          <h2 key={key} style={styles.h2}>
            {renderInline(block.children, `${key}-inline`)}
          </h2>
        );
      }
      return (
        <h3 key={key} style={styles.h3}>
          {renderInline(block.children, `${key}-inline`)}
        </h3>
      );
    case "list":
      return renderList(block, key);
    case "paragraph":
      return (
        <p key={key} style={styles.p}>
          {renderInline(block.children, `${key}-inline`)}
        </p>
      );
  }
}

// ── Component ─────────────────────────────────────────────────────────────────

interface MarkdownProps {
  source: string;
}

export function Markdown({ source }: MarkdownProps) {
  if (!source || !source.trim()) return null;

  const blocks = parseMarkdown(source);
  if (blocks.length === 0) return null;

  return (
    <div style={styles.wrapper}>
      {blocks.map((block, idx) => renderBlock(block, idx))}
    </div>
  );
}
