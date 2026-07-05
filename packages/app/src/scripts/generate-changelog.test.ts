import { describe, it, expect } from "vitest";
// Import from the sibling scripts/ directory (two levels up from src/scripts/).
// The .mjs extension must be explicit because the file is ESM with that name.
import {
  commitsToChangelog,
  writeUnreleased,
} from "../../scripts/generate-changelog.mjs";

// ---------------------------------------------------------------------------
// commitsToChangelog
// ---------------------------------------------------------------------------

describe("commitsToChangelog", () => {
  describe("grouping by type", () => {
    it("maps feat → Added", () => {
      const result = commitsToChangelog(["feat(app): add token monitor"]);
      expect(result).toContain("### Added");
      expect(result).toContain("- add token monitor");
    });

    it("maps fix → Fixed", () => {
      const result = commitsToChangelog(["fix(app): correct chart range"]);
      expect(result).toContain("### Fixed");
      expect(result).toContain("- correct chart range");
    });

    it("maps perf → Changed", () => {
      const result = commitsToChangelog(["perf(app): reduce re-renders"]);
      expect(result).toContain("### Changed");
      expect(result).toContain("- reduce re-renders");
    });

    it("maps refactor → Changed", () => {
      const result = commitsToChangelog(["refactor(core): split hook"]);
      expect(result).toContain("### Changed");
      expect(result).toContain("- split hook");
    });

    it("groups multiple commits into correct categories", () => {
      const commits = [
        "feat(app): add X",
        "fix(app): fix Y",
        "perf(app): speed Z",
      ];
      const result = commitsToChangelog(commits);
      expect(result).toContain("### Added");
      expect(result).toContain("### Fixed");
      expect(result).toContain("### Changed");
    });
  });

  describe("category order is always Added → Changed → Fixed", () => {
    it("emits Added before Changed before Fixed regardless of commit order", () => {
      const commits = [
        "fix(app): fix last",
        "perf(app): perf middle",
        "feat(app): feat first",
      ];
      const result = commitsToChangelog(commits);
      const addedPos   = result.indexOf("### Added");
      const changedPos = result.indexOf("### Changed");
      const fixedPos   = result.indexOf("### Fixed");
      expect(addedPos).toBeLessThan(changedPos);
      expect(changedPos).toBeLessThan(fixedPos);
    });
  });

  describe("stable entry order within a category", () => {
    it("preserves commit order within Added", () => {
      const commits = [
        "feat(app): feature one",
        "feat(app): feature two",
        "feat(app): feature three",
      ];
      const result = commitsToChangelog(commits);
      const onePos   = result.indexOf("- feature one");
      const twoPos   = result.indexOf("- feature two");
      const threePos = result.indexOf("- feature three");
      expect(onePos).toBeLessThan(twoPos);
      expect(twoPos).toBeLessThan(threePos);
    });
  });

  describe("ignored types are excluded", () => {
    it("excludes chore commits", () => {
      const result = commitsToChangelog(["chore(release): bump version"]);
      expect(result).toBe("");
    });

    it("excludes ci commits", () => {
      const result = commitsToChangelog(["ci: add release workflow"]);
      expect(result).toBe("");
    });

    it("excludes docs commits", () => {
      const result = commitsToChangelog(["docs: update README"]);
      expect(result).toBe("");
    });

    it("excludes test commits", () => {
      const result = commitsToChangelog(["test: add snapshot tests"]);
      expect(result).toBe("");
    });

    it("excludes build commits", () => {
      const result = commitsToChangelog(["build: upgrade vite"]);
      expect(result).toBe("");
    });

    it("excludes style commits", () => {
      const result = commitsToChangelog(["style: reformat"]);
      expect(result).toBe("");
    });

    it("returns empty string when all commits are ignored types", () => {
      const commits = ["chore: bump", "ci: deploy", "docs: readme"];
      expect(commitsToChangelog(commits)).toBe("");
    });
  });

  describe("non-conforming subjects are excluded", () => {
    it("excludes plain prose commit messages", () => {
      const result = commitsToChangelog(["Cleaned up old files"]);
      expect(result).toBe("");
    });

    it("excludes subjects with no colon", () => {
      const result = commitsToChangelog(["feat add something"]);
      expect(result).toBe("");
    });

    it("excludes subjects with type not in the map", () => {
      const result = commitsToChangelog(["wip(app): half-done feature"]);
      expect(result).toBe("");
    });

    it("excludes empty string subjects", () => {
      const result = commitsToChangelog([""]);
      expect(result).toBe("");
    });
  });

  describe("no commits → empty string", () => {
    it("returns empty string for an empty array", () => {
      expect(commitsToChangelog([])).toBe("");
    });
  });

  describe("mixed valid and invalid commits", () => {
    it("includes only valid commits", () => {
      const commits = [
        "feat(app): new dashboard",
        "Merge branch 'dev'",
        "chore: release v1.0",
        "fix: broken link",
      ];
      const result = commitsToChangelog(commits);
      expect(result).toContain("- new dashboard");
      expect(result).toContain("- broken link");
      expect(result).not.toContain("Merge branch");
      expect(result).not.toContain("release v1.0");
    });
  });

  describe("scope and breaking-change markers", () => {
    it("accepts commits without scope", () => {
      const result = commitsToChangelog(["feat: global feature"]);
      expect(result).toContain("- global feature");
    });

    it("accepts breaking change (!) marker", () => {
      const result = commitsToChangelog(["feat(app)!: breaking redesign"]);
      expect(result).toContain("### Added");
      expect(result).toContain("- breaking redesign");
    });

    it("accepts scoped breaking change without parentheses scope", () => {
      const result = commitsToChangelog(["fix!: critical security fix"]);
      expect(result).toContain("### Fixed");
      expect(result).toContain("- critical security fix");
    });
  });
});

// ---------------------------------------------------------------------------
// writeUnreleased
// ---------------------------------------------------------------------------

describe("writeUnreleased", () => {
  const HEADER = `# Changelog\n\nAll notable changes to this project are documented here.\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).\n`;

  const makeChangelog = (unreleasedBody = "", older = "") => {
    const olderSection = older
      ? `## [0.1.0] - 2026-01-01\n\n${older}`
      : "";
    return (
      HEADER +
      "\n## [Unreleased]\n" +
      (unreleasedBody ? `\n${unreleasedBody}\n` : "") +
      (olderSection ? `\n${olderSection}` : "")
    );
  };

  it("replaces the Unreleased body with provided content", () => {
    const original = makeChangelog();
    const body = "### Added\n- new feature";
    const result = writeUnreleased(original, body);
    expect(result).toContain("## [Unreleased]");
    expect(result).toContain("### Added");
    expect(result).toContain("- new feature");
  });

  it("preserves the header (lines before [Unreleased])", () => {
    const original = makeChangelog();
    const result = writeUnreleased(original, "### Added\n- x");
    expect(result).toContain("# Changelog");
    expect(result).toContain("All notable changes");
    expect(result).toContain("Keep a Changelog");
  });

  it("preserves content in older version sections below Unreleased", () => {
    const original = makeChangelog("", "### Fixed\n- old bug");
    const result = writeUnreleased(original, "### Added\n- new thing");
    expect(result).toContain("## [0.1.0] - 2026-01-01");
    expect(result).toContain("- old bug");
  });

  it("keeps the [Unreleased] heading itself", () => {
    const original = makeChangelog();
    const result = writeUnreleased(original, "");
    expect(result).toContain("## [Unreleased]");
  });

  it("returns text unchanged when there is no [Unreleased] heading", () => {
    const text = "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- something\n";
    const result = writeUnreleased(text, "### Added\n- x");
    expect(result).toBe(text);
  });

  it("handles empty body gracefully (empty Unreleased section)", () => {
    const original = makeChangelog("### Added\n- something old");
    const result = writeUnreleased(original, "");
    // The [Unreleased] heading must still be present
    expect(result).toContain("## [Unreleased]");
    // The old body should be replaced (not present)
    expect(result).not.toContain("### Added\n- something old");
  });

  it("does not duplicate [Unreleased] heading", () => {
    const original = makeChangelog();
    const result = writeUnreleased(original, "### Added\n- x");
    const count = (result.match(/## \[Unreleased\]/g) ?? []).length;
    expect(count).toBe(1);
  });

  it("produces output that promoteUnreleased can still process", () => {
    // After writeUnreleased, the file must still contain ## [Unreleased]
    // so that bump-version.mjs::promoteUnreleased can promote it.
    const original = makeChangelog();
    const body = "### Added\n- something";
    const result = writeUnreleased(original, body);
    // promoteUnreleased looks for /^## \[Unreleased\]/i
    expect(result).toMatch(/^## \[Unreleased\]/m);
  });
});
