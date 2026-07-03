/**
 * Format a raw token count into a human-readable string with K/M suffixes.
 * Examples: 500 → "500", 1500 → "1.5K", 1_200_000 → "1.2M"
 */
export function formatTokens(value: number): string {
  if (value >= 1_000_000) {
    const m = value / 1_000_000;
    return `${m % 1 === 0 ? m.toFixed(0) : m.toFixed(1)}M`;
  }
  if (value >= 1_000) {
    const k = value / 1_000;
    return `${k % 1 === 0 ? k.toFixed(0) : k.toFixed(1)}K`;
  }
  return String(Math.round(value));
}

/**
 * Format a cost value in USD.
 * Examples: 1.23 → "$1.23", 0.005 → "$0.01"
 */
export function formatCost(value: number): string {
  return `$${value.toFixed(2)}`;
}
