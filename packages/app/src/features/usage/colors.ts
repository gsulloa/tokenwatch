/**
 * Stable color palette for chart series.
 * Colors are chosen to be distinguishable on both light and dark backgrounds.
 */
const PALETTE: string[] = [
  "#7C3AED", // violet
  "#2563EB", // blue
  "#16A34A", // green
  "#D97706", // amber
  "#DC2626", // red
  "#0D9488", // teal
  "#DB2777", // pink
  "#6B7280", // gray
  "#7C2D12", // brown
  "#1D4ED8", // indigo
];

/**
 * Deterministic hash of a string to a non-negative integer.
 */
function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash; // Convert to 32-bit integer
  }
  return Math.abs(hash);
}

/**
 * Returns a stable color for a given series name.
 * The same name always maps to the same color across renders.
 */
export function colorForSeries(name: string): string {
  const index = hashString(name) % PALETTE.length;
  return PALETTE[index] ?? PALETTE[0] ?? "#7C3AED";
}
