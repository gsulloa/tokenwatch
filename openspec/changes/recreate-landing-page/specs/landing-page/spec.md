## ADDED Requirements

### Requirement: Marketing content reflects the real product

The landing page SHALL describe TokenWatch as it is actually implemented: a
native macOS menu-bar app that monitors Claude token usage, cost, and limits.
It SHALL NOT contain copy describing any other product (e.g. database clients,
SQL editors, or data-grid tooling). Every headline, feature card, and section
label SHALL map to a shipped capability of the app.

#### Scenario: Hero communicates the token-monitoring value proposition

- **WHEN** a visitor loads the landing page
- **THEN** the hero headline and lede describe monitoring Claude token usage,
  cost, and limits from the macOS menu bar
- **AND** no text references databases, SQL, Postgres/MySQL/SQL Server/DynamoDB/
  CloudWatch/Athena, or generic "data inspection"

#### Scenario: Feature sections map to implemented capabilities

- **WHEN** a visitor reads the features/sources sections
- **THEN** each described feature corresponds to a real implemented capability:
  automatic JSONL ingestion, per-project/per-model cost and token analytics,
  interactive time-series charts, the menu-bar popover with live session and
  weekly usage limits, per-model weekly breakdowns, project-group budgets with
  caps, threshold notifications, and auto-updates
- **AND** no described feature is a stub or unimplemented capability

#### Scenario: Product mockup depicts the real UI

- **WHEN** a visitor views the in-page product mockup(s)
- **THEN** the mockup depicts TokenWatch's actual UI surfaces (the cost/usage
  dashboard with charts and/or the menu-bar popover with usage limits)
- **AND** it does not depict a database grid, SQL editor, or row inspector

### Requirement: Accurate platform and download messaging

The landing page SHALL present TokenWatch as a macOS application and SHALL keep
the download experience consistent with the platforms the app is actually
distributed for.

#### Scenario: Platform is presented as macOS

- **WHEN** a visitor views the download section and CTAs
- **THEN** the primary offering is the macOS build
- **AND** any OS-detection or download labels reflect the macOS-first reality
  rather than promising unsupported platforms as first-class

#### Scenario: Version and build date load at runtime

- **WHEN** the page loads
- **THEN** it fetches the release manifest (`/releases/download.json`) and
  displays the current version and build date as it does today
- **AND** shows graceful loading/skeleton state until the manifest resolves

### Requirement: Legal pages are preserved unchanged

The change SHALL preserve the existing legal content and its routing. The
`PrivacyPolicy` and `TermsOfService` components in `src/legal.tsx`, the
`/privacy` and `/terms` routes, and the footer links to them SHALL remain
functional and their legal text SHALL NOT be rewritten.

#### Scenario: Privacy and Terms routes still work

- **WHEN** a visitor navigates to `/privacy` or `/terms`
- **THEN** the corresponding legal document renders with its existing content
- **AND** the "back to home" navigation still works

#### Scenario: Footer links to legal pages remain

- **WHEN** a visitor views the footer of the landing page
- **THEN** links to the Privacy Policy and Terms of Service are present and
  point to `/privacy` and `/terms`

### Requirement: Branding and build/deploy contract unchanged

The rewritten landing SHALL keep the existing visual identity and continue to
build and deploy through the current infrastructure without changes to the CDK
`LandingStack`.

#### Scenario: Visual identity is preserved

- **WHEN** the new landing renders
- **THEN** it uses the existing dark theme, violet accent, Geist/Geist Mono
  typography, and TokenWatch logo
- **AND** it remains responsive across desktop and mobile breakpoints

#### Scenario: Landing builds and deploys via existing pipeline

- **WHEN** the landing is built with `pnpm run landing:build`
- **THEN** the build succeeds and produces the static SPA in the existing `dist`
  output consumed by `LandingStack`
- **AND** no infrastructure or deployment mechanism is modified

### Requirement: README reflects the shipped app

The root `README.md` SHALL describe TokenWatch's current, implemented state
rather than claiming the token-monitoring features are unimplemented.

#### Scenario: README lists implemented features

- **WHEN** a developer reads `README.md`
- **THEN** it describes the app's shipped capabilities (menu-bar monitoring,
  dashboard analytics, usage limits, project budgets, auto-updates)
- **AND** it does not state that the token-monitoring features are "NOT
  implemented yet" or that the repo is "scaffold only"
- **AND** it preserves accurate repository structure, conventions, and
  build/release instructions
