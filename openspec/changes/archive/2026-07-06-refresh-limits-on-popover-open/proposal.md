## Why

Currently the Claude session/weekly usage percentages shown in the popover are only as fresh as the last 5-minute poll (or the last `limits-updated` event). Because the popover webview persists between shows (it's a non-activating `NSPanel`), the mount-time refresh runs only once at app startup — so when the user reopens the popover, they can see values that are up to ~5 minutes stale. The polling cycle is designed for background notifications, but the user expects that opening the popover always shows an up-to-date value.

## What Changes

- On every popover show, the frontend fetches fresh usage limits (via the existing `query_limits` command) so the gauges always reflect current data when the popover opens.
- The background polling cycle (5-minute interval) is preserved unchanged — it continues to drive threshold notifications.
- The on-open refresh reuses the existing snapshot cache while fetching, so the popover shows the last-known values immediately without a jarring blank/flash, then updates in place when the fresh fetch resolves.
- Rapid open/close is guarded so we don't hammer the API on repeated toggles within a short window (a short minimum interval between on-open fetches).

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `menubar-popover`: Add a requirement that the usage gauges refresh on every popover open (in addition to the existing background-poll/event updates), with stale-while-revalidate display and a short throttle to avoid redundant fetches.

## Impact

- `packages/app/src/app/Popover.tsx` — extend the existing `popover-shown` event handler to trigger a limits refresh (and reuse it for the on-open path).
- `packages/app/src/features/limits/useLimits.ts` — expose/refine the refresh path so on-open fetches keep the prior snapshot visible while loading.
- No backend/Rust changes required: `query_limits` already fetches fresh from the Anthropic usage API and the backend already emits `popover-shown` on each tray show.
- No changes to the polling task or notification logic.
