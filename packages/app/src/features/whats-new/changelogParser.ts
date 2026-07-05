/**
 * Pure functions for parsing Keep-a-Changelog formatted markdown.
 * No dependencies, fully testable in isolation.
 */

/**
 * Extracts the body of a specific version section from a changelog.
 *
 * Matches headings of the form:
 *   ## [X.Y.Z]
 *   ## [X.Y.Z] - YYYY-MM-DD
 *   ## [vX.Y.Z]          (v-prefix tolerated)
 *   ## [vX.Y.Z] - YYYY-MM-DD
 *
 * Returns the section body (trimmed) up to the next `## ` heading,
 * or null if the version is not found.
 */
export function extractVersionSection(
  markdown: string,
  version: string,
): string | null {
  // Normalise: strip a leading 'v' from the requested version for matching.
  const bare = version.replace(/^v/, "");

  // Build a regex that matches the section heading.
  // The heading may or may not have a 'v' prefix and may optionally end with
  // a date (` - YYYY-MM-DD`).
  const escapedBare = bare.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const headingPattern = new RegExp(
    `^##\\s+\\[v?${escapedBare}\\](?:\\s+-\\s+\\d{4}-\\d{2}-\\d{2})?\\s*$`,
    "m",
  );

  const headingMatch = headingPattern.exec(markdown);
  if (!headingMatch) return null;

  // Body starts after the heading line.
  const bodyStart = headingMatch.index + headingMatch[0].length;
  const rest = markdown.slice(bodyStart);

  // Body ends at the next `## ` heading (or end of string).
  const nextHeadingMatch = /^## /m.exec(rest);
  const body =
    nextHeadingMatch !== null
      ? rest.slice(0, nextHeadingMatch.index)
      : rest;

  const trimmed = body.trim();
  return trimmed.length > 0 ? trimmed : null;
}

/**
 * Returns the first dated version section in the changelog, skipping
 * `## [Unreleased]`. Returns null if no dated version section is found.
 */
export function getLatestVersionSection(
  markdown: string,
): { version: string; body: string } | null {
  // Match dated version headings: ## [X.Y.Z] - YYYY-MM-DD
  const headingPattern =
    /^##\s+\[v?([\d]+\.[\d]+\.[\d]+)\]\s+-\s+\d{4}-\d{2}-\d{2}\s*$/m;

  const headingMatch = headingPattern.exec(markdown);
  if (!headingMatch) return null;

  const version = headingMatch[1] ?? "";
  const body = extractVersionSection(markdown, version);

  return { version, body: body ?? "" };
}
