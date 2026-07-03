import { describe, it, expect } from "vitest";
import { formatTokens, formatCost } from "./format";

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
