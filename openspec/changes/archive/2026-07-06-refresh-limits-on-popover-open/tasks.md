## 1. Refresh throttle in useLimits

- [x] 1.1 In `packages/app/src/features/limits/useLimits.ts`, add a ref tracking the timestamp of the last successful fetch, updated inside `fetchData` on success.
- [x] 1.2 Add a throttled refresh path (e.g. `refresh(force?: boolean)` or a separate `refreshIfStale(minIntervalMs)`) that skips the fetch when the last successful fetch is within the minimum interval (~10s), and returns/exposes it from the hook.
- [x] 1.3 Ensure `fetchData` keeps the prior `snapshot` while loading (no null-out) — confirm existing stale-while-revalidate behavior is preserved.

## 2. Wire on-open refresh in Popover

- [x] 2.1 In `packages/app/src/app/Popover.tsx`, extend the existing `popover-shown` event handler to call the throttled limits refresh (in addition to resetting scroll to top).
- [x] 2.2 Route the mount-time refresh (lines ~94-98) through the same throttled path so the first open right after startup doesn't double-fetch.
- [x] 2.3 (Optional, per design D-open question) also refresh today-by-project on `popover-shown` for consistency; keep limits as the required scope.
- [x] 2.4 Fix effect dependency arrays so the `popover-shown` listener references the stable `refresh` callback without re-subscribing, satisfying `react-hooks/exhaustive-deps`.

## 3. Verify & QA

- [x] 3.1 Confirm no backend/Rust changes are needed (`query_limits` already fetches fresh; `popover-shown` already emitted per show).
- [ ] 3.2 Manually verify: change usage, wait <5min, reopen popover → gauges show updated % on open (not the stale poll value).
- [ ] 3.3 Manually verify: rapid open/close within the throttle window issues only one fetch (observe network/logs).
- [ ] 3.4 Manually verify: opening the popover shows last-known values immediately with no blank/flash while the fresh fetch resolves.
- [x] 3.5 Run `pnpm typecheck && pnpm lint && pnpm test:run` and ensure all pass.
