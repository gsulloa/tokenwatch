#!/usr/bin/env node
// generate-changelog.mjs — Auto-generates the `## [Unreleased]` section of the
// root CHANGELOG.md from Conventional Commits since the last git tag.
//
// Must be run BEFORE bump-version.mjs in the release flow so that
// promoteUnreleased() promotes real commit-derived content instead of an empty
// section.
//
// Usage:
//   node scripts/generate-changelog.mjs
//
// Writes directly to the root CHANGELOG.md. No arguments needed — the script
// locates the repo root the same way sync-changelog.mjs does
// (scripts/ → packages/app/ → packages/ → repo root).

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

// ---------------------------------------------------------------------------
// Category mapping: Conventional Commit type → Keep a Changelog heading
// ---------------------------------------------------------------------------

/**
 * Maps a Conventional Commit type to a Keep a Changelog category heading.
 * Types that map to null are ignored (no user-facing entry).
 *
 * @type {Record<string, string | null>}
 */
const TYPE_TO_CATEGORY = {
  feat:     "Added",
  fix:      "Fixed",
  perf:     "Changed",
  refactor: "Changed",
  // explicitly ignored — no user-facing notes
  chore:  null,
  ci:     null,
  docs:   null,
  test:   null,
  build:  null,
  style:  null,
};

/**
 * Fixed display order of Keep a Changelog categories.
 * Only categories that have at least one entry are emitted.
 */
const CATEGORY_ORDER = ["Added", "Changed", "Fixed"];

// ---------------------------------------------------------------------------
// Pure: commitsToChangelog
// ---------------------------------------------------------------------------

/**
 * Pure function: convert an array of Conventional Commit subject strings into
 * a Keep-a-Changelog-formatted markdown string for the `[Unreleased]` body.
 *
 * Parsing rules:
 *   - Subject format: `type(scope)?!?: description`
 *     - `type` must be a recognised key in TYPE_TO_CATEGORY.
 *     - `scope` is optional and may include letters, digits, hyphens, and slashes.
 *     - `!` (breaking-change marker) is accepted but does not change the category.
 *   - The entry text is the description after `type(scope)?: ` (the part after ": ").
 *   - Types that map to `null` are silently excluded.
 *   - Subjects that do not match the Conventional Commit pattern are excluded.
 *
 * Output format:
 *   ### Added
 *   - description one
 *   - description two
 *
 *   ### Changed
 *   - description
 *
 * Categories appear in the order defined by CATEGORY_ORDER. Entries within a
 * category preserve their original commit order (stable, no sorting).
 *
 * Returns an empty string when there are no relevant commits (caller leaves
 * `[Unreleased]` body empty, which is valid Keep a Changelog).
 *
 * @param {string[]} commits - array of commit subject strings
 * @returns {string} formatted markdown body for the Unreleased section
 */
export function commitsToChangelog(commits) {
  // Conventional Commit subject regex:
  //   ^<type>(\(<scope>\))?!?:\s+<description>$
  const CC_RE = /^([a-z]+)(?:\([^)]*\))?!?:\s+(.+)$/;

  /** @type {Record<string, string[]>} */
  const buckets = {};
  for (const cat of CATEGORY_ORDER) {
    buckets[cat] = [];
  }

  for (const subject of commits) {
    const m = CC_RE.exec(subject.trim());
    if (!m) continue; // non-conforming — skip

    const type = m[1];
    const description = m[2];

    if (!(type in TYPE_TO_CATEGORY)) continue; // unknown type — skip
    const category = TYPE_TO_CATEGORY[type];
    if (category === null) continue; // ignored type

    buckets[category].push(description);
  }

  const sections = [];
  for (const cat of CATEGORY_ORDER) {
    if (buckets[cat].length === 0) continue;
    sections.push(`### ${cat}`);
    for (const entry of buckets[cat]) {
      sections.push(`- ${entry}`);
    }
    sections.push(""); // blank line after each section
  }

  if (sections.length === 0) return "";

  // Trim trailing blank line added after the last section.
  while (sections.length > 0 && sections[sections.length - 1] === "") {
    sections.pop();
  }

  return sections.join("\n");
}

// ---------------------------------------------------------------------------
// Pure: writeUnreleased
// ---------------------------------------------------------------------------

/**
 * Pure function: replace only the body between the `## [Unreleased]` heading
 * and the next `## ` section with the provided content, returning the updated
 * changelog text.
 *
 * Behaviour:
 *   - If the text has no `## [Unreleased]` heading, returns it unchanged.
 *   - Preserves the header (lines before `## [Unreleased]`), the heading line
 *     itself, and every line from the next `## ` section onward.
 *   - A single blank line is placed between the heading and the body (and
 *     between the body and the next section) when the body is non-empty.
 *   - When `unreleasedBody` is empty (""), the heading is followed by a single
 *     blank line and then the next section, keeping the file valid.
 *
 * @param {string} changelogText   - full CHANGELOG.md text
 * @param {string} unreleasedBody  - formatted body lines (from commitsToChangelog)
 * @returns {string} updated changelog text
 */
export function writeUnreleased(changelogText, unreleasedBody) {
  const lines = changelogText.split("\n");

  // Find the `## [Unreleased]` heading line.
  const unreleasedIdx = lines.findIndex((l) => /^## \[Unreleased\]/i.test(l));
  if (unreleasedIdx === -1) {
    return changelogText; // no heading — return unchanged
  }

  // Find the start of the next `## ` section (first `## ` line after heading).
  let nextSectionIdx = lines.length;
  for (let i = unreleasedIdx + 1; i < lines.length; i++) {
    if (/^## /.test(lines[i])) {
      nextSectionIdx = i;
      break;
    }
  }

  const before = lines.slice(0, unreleasedIdx + 1); // includes the heading line
  const after  = lines.slice(nextSectionIdx);        // next section onward

  // Build the replacement body lines.
  const bodyLines = unreleasedBody.length > 0
    ? ["", ...unreleasedBody.split("\n"), ""]
    : [""];

  // Ensure a clean blank line separation before the next section (avoid doubles).
  const joinedAfter =
    after.length > 0 && after[0] !== "" ? ["", ...after] : after;

  return [...before, ...bodyLines, ...joinedAfter].join("\n");
}

// ---------------------------------------------------------------------------
// main (side-effects: git + filesystem)
// ---------------------------------------------------------------------------

function main() {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  // scripts/ → packages/app/ → packages/ → repo root
  const repoRoot = join(scriptDir, "..", "..", "..");
  const changelogPath = join(repoRoot, "CHANGELOG.md");

  if (!existsSync(changelogPath)) {
    process.stderr.write(
      `generate-changelog: ERROR: root CHANGELOG.md not found at ${changelogPath}\n`
    );
    process.exit(1);
  }

  // Determine the last tag. Fall back to an empty range (all commits) if none.
  let lastTag;
  try {
    lastTag = execSync("git describe --tags --abbrev=0", {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["pipe", "pipe", "pipe"],
    }).trim();
  } catch {
    // No tags yet — use an empty range marker; git log range becomes "..HEAD"
    // which git interprets as all commits (equivalent to --all without merges).
    lastTag = "";
  }

  // Build the git log range. When there is no last tag we list all commits.
  const range = lastTag ? `${lastTag}..HEAD` : "HEAD";

  let rawLog;
  try {
    rawLog = execSync(
      `git log ${range} --pretty=format:%s --no-merges`,
      {
        cwd: repoRoot,
        encoding: "utf8",
        stdio: ["pipe", "pipe", "pipe"],
      }
    ).trim();
  } catch {
    // git log failed (e.g. empty repo / no commits) — treat as no commits.
    rawLog = "";
  }

  const commits = rawLog.length > 0 ? rawLog.split("\n") : [];
  const body = commitsToChangelog(commits);

  const changelogText = readFileSync(changelogPath, "utf8");
  const updated = writeUnreleased(changelogText, body);
  writeFileSync(changelogPath, updated, "utf8");

  const entryCount = body ? body.split("\n").filter((l) => l.startsWith("- ")).length : 0;
  process.stderr.write(
    `generate-changelog: wrote ${entryCount} entr${entryCount === 1 ? "y" : "ies"} into [Unreleased]` +
    (lastTag ? ` (since ${lastTag})` : " (all commits)") +
    "\n"
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
