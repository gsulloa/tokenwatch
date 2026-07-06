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

interface MenubarBadgeSelectProps {
  mode: string;
  onChange: (mode: string) => void;
  disabled?: boolean;
}

/**
 * Selects what the menu-bar text badge shows next to the tray icon.
 * `off` keeps the icon-only behaviour; the others render a live percentage.
 */
function MenubarBadgeSelect({ mode, onChange, disabled }: MenubarBadgeSelectProps) {
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
      <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
        Barra de menú
      </span>
      <select
        value={mode}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        style={{
          fontSize: 12,
          fontFamily: "var(--font-ui)",
          color: "var(--text)",
          background: "var(--raised)",
          border: "1px solid var(--border)",
          borderRadius: "var(--radius-sm)",
          padding: "2px 6px",
          cursor: disabled ? "default" : "pointer",
        }}
      >
        <option value="off">Solo icono</option>
        <option value="session">Sesión 5h</option>
        <option value="week">Semana</option>
        <option value="max">Mayor</option>
      </select>
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

  const [alertsMuted, setAlertsMuted] = useState(false);
  const [muteLoading, setMuteLoading] = useState(false);
  const [badgeMode, setBadgeMode] = useState("off");
  const [badgeLoading, setBadgeLoading] = useState(false);
  const [showAbout, setShowAbout] = useState(false);

  // Load initial mute + menu-bar badge state
  useEffect(() => {
    void safeTauriInvoke<boolean>("get_alerts_muted").then((result) => {
      if (result !== null) setAlertsMuted(result);
    });
    void safeTauriInvoke<string>("get_menubar_badge_mode").then((result) => {
      if (result !== null) setBadgeMode(result);
    });
  }, []);

  // Trigger immediate refresh of both hooks on mount.
  // Uses the throttled path for limits so the first popover open right after
  // startup doesn't double-fetch when the popover-shown event fires shortly after.
  useEffect(() => {
    refreshLimitsIfStale();
    void refreshToday();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The popover webview persists between shows (it's a non-activating NSPanel),
  // and becoming key can leave it scrolled to the bottom. On each show the
  // backend emits "popover-shown"; pin the scroll back to the top and trigger
  // a throttled limits refresh + today-by-project refresh so the gauges reflect
  // current data on every open.
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
          refreshLimitsIfStale();
          void refreshToday();
        });
      } catch {
        // Non-Tauri environment — nothing to listen to.
      }
    })();
    return () => unlisten?.();
  }, [refreshLimitsIfStale, refreshToday]);

  const handleMuteChange = useCallback(async (muted: boolean) => {
    setMuteLoading(true);
    try {
      await safeTauriInvoke<void>("set_alerts_muted", { muted });
      setAlertsMuted(muted);
    } finally {
      setMuteLoading(false);
    }
  }, []);

  const handleBadgeChange = useCallback(async (mode: string) => {
    setBadgeLoading(true);
    try {
      await safeTauriInvoke<void>("set_menubar_badge_mode", { mode });
      setBadgeMode(mode);
    } finally {
      setBadgeLoading(false);
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
          alignItems: "flex-start",
          justifyContent: "space-between",
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "var(--space-xs)",
          }}
        >
          <AlertsMuteToggle
            muted={alertsMuted}
            onChange={(v) => void handleMuteChange(v)}
            disabled={muteLoading}
          />
          <MenubarBadgeSelect
            mode={badgeMode}
            onChange={(v) => void handleBadgeChange(v)}
            disabled={badgeLoading}
          />
          <p
            style={{
              margin: 0,
              fontSize: 11,
              lineHeight: 1.3,
              color: "var(--text-subtle)",
              maxWidth: 200,
            }}
          >
            Para que no se oculte, arrastra el icono con ⌘ hacia la izquierda de
            la barra.
          </p>
        </div>
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
