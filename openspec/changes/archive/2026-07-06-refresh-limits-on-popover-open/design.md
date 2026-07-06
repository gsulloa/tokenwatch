## Context

TokenWatch shows Claude session (5h) and weekly usage gauges in a menu-bar popover. Usage data flows as follows today:

- A background task (`spawn_limits_polling_task` in `packages/app/src-tauri/src/limits/mod.rs`) runs every `LIMITS_POLL_SECS = 300` (5 min). Each cycle calls `fetch_snapshot()` (hits `https://api.anthropic.com/api/oauth/usage`), caches the snapshot in `AppState.last_limits`, evaluates threshold alerts, fires notifications, and emits a `limits-updated` event.
- The frontend `useLimits` hook (`packages/app/src/features/limits/useLimits.ts`) fetches once on mount via the `query_limits` command and re-fetches whenever `limits-updated` is received.
- `query_limits` performs a **fresh** API fetch (not just a cache read) and seeds the cache.
- The popover window is a persistent non-activating `NSPanel` on macOS. The tray-click handler in `lib.rs` shows the panel and emits a `popover-shown` event on each show. Because the webview persists, React does **not** remount between shows — so the `useEffect(..., [])` "refresh on mount" in `Popover.tsx` runs only once at app startup.

Result: reopening the popover displays whatever the last poll/event produced, which can be up to ~5 minutes stale. The polling interval exists for background notifications, not for display freshness.

## Goals / Non-Goals

**Goals:**
- Every time the popover is shown, the usage gauges reflect a freshly fetched snapshot.
- Preserve the background polling cycle unchanged as the driver for threshold notifications.
- Avoid a jarring blank/loading flash on open — show last-known values while the fresh fetch is in flight.
- Avoid redundant API calls on rapid open/close toggling.

**Non-Goals:**
- Changing the polling interval or notification logic.
- Any Rust/backend changes — `query_limits` and the `popover-shown` emission already provide what we need.
- Refreshing charts/history beyond what the popover already refreshes (today-by-project already re-fetches on its own event; this change is scoped to limits freshness, though the same on-open hook may also refresh today-by-project for consistency).

## Decisions

### D1: Trigger the refresh from the `popover-shown` event, not from mount
The `popover-shown` event is emitted by the backend on every tray show, making it the correct signal for "the user just opened the popover." The existing handler in `Popover.tsx` (which resets scroll to top) is extended to also call `refreshLimits()`. This is the only reliable per-open hook given the persistent webview.

- Alternative considered: listen to a Tauri window `focus`/`show` event directly. Rejected — `popover-shown` is already emitted purpose-built for this and avoids ambiguity with focus changes that aren't opens.
- Alternative considered: backend-side immediate fetch on tray click that emits `limits-updated`. Rejected — adds backend complexity, and the frontend already has a clean fresh-fetch path via `query_limits`; keeps the change frontend-only.

### D2: Stale-while-revalidate display
`useLimits.fetchData` already keeps the previous `snapshot` in state and only replaces it on a successful fetch (it does not null it out while loading). The gauges therefore keep showing the last-known values during the on-open re-fetch. We keep this behavior; we do not blank the UI. The `loading` flag may be used for a subtle inline indicator but MUST NOT hide existing values.

### D3: Throttle on-open fetches with a minimum interval
To avoid hammering the API when the user rapidly toggles the popover, the on-open refresh is guarded by a minimum interval (e.g. skip the fetch if the last successful fetch completed within the last few seconds). A short window (~10s suggested) is enough to dedupe accidental double-opens while still feeling "always fresh" for normal use. The throttle lives in the frontend (a ref tracking the last fetch timestamp) so no backend state is needed.

- Alternative considered: no throttle. Rejected — rapid open/close could issue many API calls and the endpoint is rate-limited/keychain-gated.

### D4: Keep the mount-time refresh
The existing mount-time `refreshLimits()`/`refreshToday()` remains as the cold-start path (first show right after launch, before any poll has necessarily completed). The on-open path and the mount path share the same throttle guard so the first open right after startup doesn't double-fetch.

## Risks / Trade-offs

- [Rapid toggling still triggers a fetch after the throttle window] → The throttle window bounds worst-case call rate; tune the interval if the usage endpoint proves sensitive.
- [Fetch failure on open leaves stale values displayed] → Acceptable and preferable to blanking; the existing `error`/`status` handling already renders an explicit unavailable state when a snapshot is genuinely unavailable, and stale-but-labeled data is better than an empty popover.
- [`popover-shown` listener referencing `refreshLimits`] → `refreshLimits` (`fetchData`) is a stable `useCallback` with `[]` deps, so the listener effect can reference it safely without re-subscribing; ensure deps are declared correctly to satisfy lint.

## Migration Plan

Frontend-only, no data migration. Ship in the app package; behavior is additive (an extra fetch on open). Rollback is reverting the `Popover.tsx` handler change.

## Open Questions

- Exact throttle window value (proposed ~10s) — confirm during implementation/QA against perceived freshness vs. call volume.
- Whether the on-open refresh should also cover today-by-project (`refreshToday`) for full consistency, or limits-only per the user's stated intent. Leaning toward refreshing both since the popover shows both, but limits is the required scope.
