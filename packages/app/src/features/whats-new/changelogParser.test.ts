import { describe, it, expect } from "vitest";
import {
  extractVersionSection,
  getLatestVersionSection,
} from "./changelogParser";

// ── Fixtures ──────────────────────────────────────────────────────────────────

const SAMPLE_CHANGELOG = `# Changelog

## [Unreleased]

## [1.2.0] - 2024-06-01

### Added
- Cool new feature

### Fixed
- Some bug

## [1.1.0] - 2024-05-01

### Added
- Earlier feature

## [1.0.0] - 2024-04-01

Initial release.
`;

const CHANGELOG_WITH_V_PREFIX = `# Changelog

## [v2.0.0] - 2024-07-01

### Added
- Big release

## [v1.0.0] - 2024-06-01

- Initial.
`;

const CHANGELOG_EMPTY_SECTION = `# Changelog

## [Unreleased]

## [1.5.0] - 2024-08-01

## [1.4.0] - 2024-07-01

### Added
- Something
`;

const CHANGELOG_NO_VERSIONS = `# Changelog

## [Unreleased]

Nothing yet.
`;

// ── extractVersionSection ─────────────────────────────────────────────────────

describe("extractVersionSection", () => {
  it("returns the body for an existing version", () => {
    const body = extractVersionSection(SAMPLE_CHANGELOG, "1.2.0");
    expect(body).not.toBeNull();
    expect(body).toContain("Cool new feature");
    expect(body).toContain("Some bug");
  });

  it("does NOT include the next section body", () => {
    const body = extractVersionSection(SAMPLE_CHANGELOG, "1.2.0");
    expect(body).not.toContain("Earlier feature");
  });

  it("returns null for a missing version", () => {
    const body = extractVersionSection(SAMPLE_CHANGELOG, "9.9.9");
    expect(body).toBeNull();
  });

  it("accepts a v-prefixed requested version", () => {
    const body = extractVersionSection(SAMPLE_CHANGELOG, "v1.2.0");
    expect(body).not.toBeNull();
    expect(body).toContain("Cool new feature");
  });

  it("matches headings that have a v-prefix in the changelog", () => {
    const body = extractVersionSection(CHANGELOG_WITH_V_PREFIX, "2.0.0");
    expect(body).not.toBeNull();
    expect(body).toContain("Big release");
  });

  it("returns null for an empty section body", () => {
    const body = extractVersionSection(CHANGELOG_EMPTY_SECTION, "1.5.0");
    expect(body).toBeNull();
  });

  it("handles a heading with a date suffix", () => {
    const body = extractVersionSection(SAMPLE_CHANGELOG, "1.1.0");
    expect(body).not.toBeNull();
    expect(body).toContain("Earlier feature");
  });

  it("does NOT match the Unreleased section", () => {
    // "Unreleased" is not a semver — should not be returned
    const body = extractVersionSection(SAMPLE_CHANGELOG, "Unreleased");
    expect(body).toBeNull();
  });

  it("returns null for a changelog with no version sections", () => {
    const body = extractVersionSection(CHANGELOG_NO_VERSIONS, "1.0.0");
    expect(body).toBeNull();
  });
});

// ── getLatestVersionSection ───────────────────────────────────────────────────

describe("getLatestVersionSection", () => {
  it("returns the first dated version (skipping Unreleased)", () => {
    const result = getLatestVersionSection(SAMPLE_CHANGELOG);
    expect(result).not.toBeNull();
    expect(result?.version).toBe("1.2.0");
    expect(result?.body).toContain("Cool new feature");
  });

  it("works when headings use a v-prefix", () => {
    const result = getLatestVersionSection(CHANGELOG_WITH_V_PREFIX);
    expect(result).not.toBeNull();
    expect(result?.version).toBe("2.0.0");
    expect(result?.body).toContain("Big release");
  });

  it("returns null when there are no dated version sections", () => {
    const result = getLatestVersionSection(CHANGELOG_NO_VERSIONS);
    expect(result).toBeNull();
  });

  it("returns an empty body string when the section is empty", () => {
    const result = getLatestVersionSection(CHANGELOG_EMPTY_SECTION);
    expect(result).not.toBeNull();
    expect(result?.version).toBe("1.5.0");
    expect(result?.body).toBe("");
  });
});
