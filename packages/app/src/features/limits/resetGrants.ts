import type { ResetGrantSummary } from "./types";

export interface ResetGrantsSummary {
  count: number;
  nearestEndsAt: string | null;
}

/** Sum of resetsLeft (>0 only) and the earliest valid endsAt; null when count is 0. */
export function summarizeResetGrants(
  grants: ResetGrantSummary[],
): ResetGrantsSummary | null {
  let count = 0;
  let nearest: { iso: string; ms: number } | null = null;
  for (const g of grants) {
    if (g.resetsLeft <= 0) continue;
    count += g.resetsLeft;
    if (g.endsAt) {
      const ms = new Date(g.endsAt).getTime();
      if (!Number.isNaN(ms) && (nearest === null || ms < nearest.ms)) {
        nearest = { iso: g.endsAt, ms };
      }
    }
  }
  if (count === 0) return null;
  return { count, nearestEndsAt: nearest?.iso ?? null };
}

/** "30 nov" via toLocaleDateString("es"); "" if invalid. */
export function formatGrantDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString("es", { day: "numeric", month: "short" });
}

/** "1 reinicio disponible" | "N reinicios disponibles", + " · vence el <fecha>". */
export function formatResetGrantsLabel(summary: ResetGrantsSummary): string {
  const base =
    summary.count === 1
      ? "1 reinicio disponible"
      : `${summary.count} reinicios disponibles`;
  const date = summary.nearestEndsAt
    ? formatGrantDate(summary.nearestEndsAt)
    : "";
  return date ? `${base} · vence el ${date}` : base;
}
