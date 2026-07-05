/**
 * Type declarations for generate-changelog.mjs.
 * Consumed by src/scripts/generate-changelog.test.ts via the TypeScript
 * compiler when running typecheck and tests.
 */

/**
 * Convert an array of Conventional Commit subject strings into a
 * Keep-a-Changelog-formatted markdown string for the `[Unreleased]` body.
 * Returns an empty string when there are no relevant commits.
 */
export declare function commitsToChangelog(commits: string[]): string;

/**
 * Replace only the body of `## [Unreleased]` in the provided changelog text
 * with `unreleasedBody`, returning the updated text.
 * Returns the original text unchanged when no `## [Unreleased]` heading is found.
 */
export declare function writeUnreleased(
  changelogText: string,
  unreleasedBody: string,
): string;
