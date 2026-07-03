/**
 * Stable color palette for chart series.
 * Colors are chosen to be distinguishable on both light and dark backgrounds.
 * The first 10 entries match the original palette; ~6 more distinct hues added.
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
  // Additional 6 distinct hues
  "#EA580C", // orange
  "#0369A1", // sky blue
  "#15803D", // forest green
  "#9333EA", // purple
  "#B45309", // dark amber
  "#0F766E", // dark teal
];

/**
 * Deterministic hash of a string to a non-negative integer.
 * Kept for backward compatibility with colorForSeries.
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
 * Generate a color by index using golden-angle HSL rotation for indices beyond
 * the palette length. Fixed saturation/lightness work on both light and dark bg.
 */
function colorByIndex(index: number): string {
  if (index < PALETTE.length) {
    return PALETTE[index]!;
  }
  // Golden-angle step (~137.5°) ensures maximum separation between adjacent hues
  const GOLDEN_ANGLE = 137.508;
  const hue = (index * GOLDEN_ANGLE) % 360;
  // Saturation 65%, lightness 45% — readable on both light and dark backgrounds
  return `hsl(${hue.toFixed(1)}, 65%, 45%)`;
}

/**
 * Build a deterministic color map from an ordered list of series names.
 * Assignment is by position/index so adjacent series in the stacked chart
 * get maximally contrasting colors.
 *
 * Same ordered input always produces the same map.
 */
export function buildColorMap(orderedNames: string[]): Map<string, string> {
  const map = new Map<string, string>();
  orderedNames.forEach((name, index) => {
    map.set(name, colorByIndex(index));
  });
  return map;
}

/**
 * Returns a stable color for a given series name.
 * Uses a hash of the name, kept for backward compatibility.
 * Prefer buildColorMap for new code.
 */
export function colorForSeries(name: string): string {
  const index = hashString(name) % PALETTE.length;
  return PALETTE[index] ?? PALETTE[0] ?? "#7C3AED";
}
