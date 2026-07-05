import { describe, it, expect } from "vitest";
import { formatTokens, formatCost, formatTokensExact, formatPercent } from "./format";

describe("formatTokens", () => {
  it("formats values under 1K with no suffix", () => {
    expect(formatTokens(0)).toBe("0");
    expect(formatTokens(500)).toBe("500");
    expect(formatTokens(999)).toBe("999");
  });

  it("formats values 1K–999K with K suffix", () => {
    expect(formatTokens(1000)).toBe("1K");
    expect(formatTokens(1500)).toBe("1.5K");
    expect(formatTokens(10000)).toBe("10K");
    expect(formatTokens(999000)).toBe("999K");
  });

  it("formats values >= 1M with M suffix", () => {
    expect(formatTokens(1_000_000)).toBe("1M");
    expect(formatTokens(1_200_000)).toBe("1.2M");
    expect(formatTokens(10_000_000)).toBe("10M");
  });

  it("rounds fractional values in the sub-K range", () => {
    expect(formatTokens(500.7)).toBe("501");
  });
});

describe("formatCost", () => {
  it("formats cost with USD sign and 2 decimal places", () => {
    expect(formatCost(1.23)).toBe("$1.23");
    expect(formatCost(0)).toBe("$0.00");
    expect(formatCost(100)).toBe("$100.00");
    expect(formatCost(0.005)).toBe("$0.01");
    expect(formatCost(0.004)).toBe("$0.00");
  });

  it("formats large costs correctly", () => {
    expect(formatCost(1234.56)).toBe("$1234.56");
  });
});

describe("formatTokensExact", () => {
  it("formats zero", () => {
    expect(formatTokensExact(0)).toBe("0");
  });

  it("formats small numbers without separators", () => {
    expect(formatTokensExact(500)).toBe("500");
    expect(formatTokensExact(999)).toBe("999");
  });

  it("formats thousands with comma separator", () => {
    expect(formatTokensExact(1000)).toBe("1,000");
    expect(formatTokensExact(1500)).toBe("1,500");
    expect(formatTokensExact(10000)).toBe("10,000");
  });

  it("formats millions with comma separators", () => {
    expect(formatTokensExact(1_200_000)).toBe("1,200,000");
    expect(formatTokensExact(1_000_000)).toBe("1,000,000");
  });

  it("rounds fractional values", () => {
    expect(formatTokensExact(500.7)).toBe("501");
    expect(formatTokensExact(1499.4)).toBe("1,499");
  });
});

describe("formatPercent", () => {
  it("handles total=0 gracefully", () => {
    expect(formatPercent(0, 0)).toBe("0%");
    expect(formatPercent(50, 0)).toBe("0%");
  });

  it("formats simple percentages with one decimal", () => {
    expect(formatPercent(25, 100)).toBe("25.0%");
    expect(formatPercent(50, 100)).toBe("50.0%");
    expect(formatPercent(100, 100)).toBe("100.0%");
  });

  it("formats fractional percentages", () => {
    expect(formatPercent(1, 3)).toBe("33.3%");
    expect(formatPercent(2, 3)).toBe("66.7%");
  });

  it("formats zero value", () => {
    expect(formatPercent(0, 100)).toBe("0.0%");
  });
});
