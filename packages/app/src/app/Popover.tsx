import { useState, useEffect, useCallback } from "react";
import { LimitsSection } from "@/features/limits/LimitsSection";
import { TodayByProjectList } from "@/features/usage/TodayByProjectList";
import { GroupBudgetsSection } from "@/features/budgets/GroupBudgetsSection";
import { useGroupBudgets } from "@/features/budgets/useGroupBudgets";
import { useLimits } from "@/features/limits/useLimits";
import { useTodayByProject } from "@/features/usage/useTodayByProject";
import { UpdateBanner } from "@/features/updates/UpdateBanner";
import { AboutSection } from "@/features/about/AboutSection";
import { openChangelogInDashboard } from "@/features/about/changelogChannel";

/**
 * Safely invoke a Tauri command. Returns null in non-Tauri environments.
 */
async function safeTauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return null;
  }
}

interface AlertsMuteToggleProps {
  muted: boolean;
  onChange: (muted: boolean) => void;
  disabled?: boolean;
}

function AlertsMuteToggle({ muted, onChange, disabled }: AlertsMuteToggleProps) {
  return (
    <label
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-xs)",
        cursor: disabled ? "default" : "pointer",
        userSelect: "none",
      }}
    >
      <input
        type="checkbox"
        checked={muted}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
        style={{ cursor: disabled ? "default" : "pointer" }}
      />
      <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Silenciar alertas
      </span>
    </label>
  );
}

/**
 * The main popover content: limits, today-by-project, and the alerts toggle.
 * The chart and controls are rendered in App.tsx below this component.
 */
export function Popover() {
  const {
    snapshot,
    loading: limitsLoading,
    refresh: refreshLimits,
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

  const [alertsMuted, setAlertsMuted] = useState(false);
  const [muteLoading, setMuteLoading] = useState(false);
  const [showAbout, setShowAbout] = useState(false);

  // Load initial mute state
  useEffect(() => {
    void safeTauriInvoke<boolean>("get_alerts_muted").then((result) => {
      if (result !== null) setAlertsMuted(result);
    });
  }, []);

  // Trigger immediate refresh of both hooks on mount
  useEffect(() => {
    void refreshLimits();
    void refreshToday();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The popover webview persists between shows (it's a non-activating NSPanel),
  // and becoming key can leave it scrolled to the bottom. On each show the
  // backend emits "popover-shown"; pin the scroll back to the top.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen("popover-shown", () => {
          requestAnimationFrame(() => {
            (document.activeElement as HTMLElement | null)?.blur?.();
            window.scrollTo(0, 0);
            document.scrollingElement?.scrollTo(0, 0);
          });
        });
      } catch {
        // Non-Tauri environment — nothing to listen to.
      }
    })();
    return () => unlisten?.();
  }, []);

  const handleMuteChange = useCallback(async (muted: boolean) => {
    setMuteLoading(true);
    try {
      await safeTauriInvoke<void>("set_alerts_muted", { muted });
      setAlertsMuted(muted);
    } finally {
      setMuteLoading(false);
    }
  }, []);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-md)",
      }}
    >
      <LimitsSection snapshot={snapshot} loading={limitsLoading} />

      <div
        style={{
          height: 1,
          background: "var(--border)",
          margin: "0 calc(-1 * var(--space-xs))",
        }}
        role="separator"
      />

      {hasDefinedGroups && (
        <>
          <GroupBudgetsSection snapshot={budgetsSnapshot} loading={budgetsLoading} />

          <div
            style={{
              height: 1,
              background: "var(--border)",
              margin: "0 calc(-1 * var(--space-xs))",
            }}
            role="separator"
          />
        </>
      )}

      <TodayByProjectList data={todayData} loading={todayLoading} />

      <div
        style={{
          height: 1,
          background: "var(--border)",
          margin: "0 calc(-1 * var(--space-xs))",
        }}
        role="separator"
      />

      <UpdateBanner />

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <AlertsMuteToggle
          muted={alertsMuted}
          onChange={(v) => void handleMuteChange(v)}
          disabled={muteLoading}
        />
        <div style={{ display: "flex", gap: "var(--space-xs)", alignItems: "center" }}>
          <button
            onClick={() => setShowAbout((v) => !v)}
            style={{
              background: "none",
              border: "none",
              padding: "2px 4px",
              cursor: "pointer",
              fontSize: 12,
              color: "var(--text-muted)",
              textDecoration: "underline",
            }}
          >
            Acerca de
          </button>
          <button
            onClick={() => void safeTauriInvoke<void>("open_dashboard")}
            style={{
              background: "none",
              border: "none",
              padding: "2px 4px",
              cursor: "pointer",
              fontSize: 12,
              color: "var(--text-muted)",
              textDecoration: "underline",
            }}
          >
            Abrir dashboard de costos
          </button>
        </div>
      </div>

      {showAbout && (
        <>
          <div
            style={{
              height: 1,
              background: "var(--border)",
              margin: "0 calc(-1 * var(--space-xs))",
            }}
            role="separator"
          />
          <AboutSection
            onOpenChangelog={() => void openChangelogInDashboard()}
          />
        </>
      )}
    </div>
  );
}
