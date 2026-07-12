import { forwardRef, useImperativeHandle } from "react";
import { LimitsSection } from "@/features/limits/LimitsSection";
import { useLimits } from "@/features/limits/useLimits";
import { GroupBudgetsSection } from "@/features/budgets/GroupBudgetsSection";
import { useGroupBudgets } from "@/features/budgets/useGroupBudgets";
import { TodayByProjectList } from "@/features/usage/TodayByProjectList";
import { useTodayByProject } from "@/features/usage/useTodayByProject";

/**
 * Imperative handle exposed by LiveStatusPanel so callers (popover / dashboard)
 * can trigger a refresh from their own lifecycle events without re-rendering
 * the panel itself.
 */
export interface LiveStatusPanelHandle {
  /** Unconditionally refresh limits and today-by-project. */
  refresh: () => void;
  /** Refresh limits only if stale (throttled), plus an unconditional today refresh. */
  refreshIfStale: () => void;
}

function Separator() {
  return (
    <div
      style={{
        height: 1,
        background: "var(--border)",
        margin: "0 calc(-1 * var(--space-xs))",
      }}
      role="separator"
    />
  );
}

/**
 * Shared composition of the "live status" sections — limits, group budgets,
 * and today-by-project — used by BOTH the menu-bar popover and the dashboard
 * to guarantee visual/behavioral parity between the two surfaces.
 *
 * Internally wires the three self-contained hooks; does not fix its own
 * width so it flows to whatever container hosts it (popover ~360px, or the
 * dashboard's sidebar rail).
 */
export const LiveStatusPanel = forwardRef<LiveStatusPanelHandle>(
  function LiveStatusPanel(_props, ref) {
    const {
      snapshot,
      loading: limitsLoading,
      refresh: refreshLimits,
      refreshIfStale: refreshLimitsIfStale,
    } = useLimits();

    const {
      data: todayData,
      loading: todayLoading,
      refresh: refreshToday,
    } = useTodayByProject();

    // Used to conditionally show the GroupBudgetsSection + its surrounding separators
    const { snapshot: budgetsSnapshot, loading: budgetsLoading } = useGroupBudgets();
    const hasDefinedGroups =
      budgetsLoading ||
      (budgetsSnapshot !== null &&
        budgetsSnapshot.rows.some((r) => r.groupId !== null));

    useImperativeHandle(
      ref,
      () => ({
        refresh: () => {
          refreshLimits();
          void refreshToday();
        },
        refreshIfStale: () => {
          refreshLimitsIfStale();
          void refreshToday();
        },
      }),
      [refreshLimits, refreshLimitsIfStale, refreshToday],
    );

    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "var(--space-md)",
        }}
      >
        <LimitsSection snapshot={snapshot} loading={limitsLoading} />

        <Separator />

        {hasDefinedGroups && (
          <>
            <GroupBudgetsSection snapshot={budgetsSnapshot} loading={budgetsLoading} />
            <Separator />
          </>
        )}

        <TodayByProjectList data={todayData} loading={todayLoading} />
      </div>
    );
  },
);
