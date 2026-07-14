//! Embedded price table for Claude models.
//!
//! Prices are in USD per **million** tokens. Cost is computed as:
//!   `cost = input*Pin + output*Pout + cache_creation*Pwrite + cache_read*Pread`
//!
//! Source: Anthropic Claude pricing page and LiteLLM
//! `model_prices_and_context_window.json` (verified 2026-07-03).
//!
//! Cache write multiplier: 1.25× input; cache read multiplier: 0.1× input —
//! standard Anthropic v1 rates (no split 1h/5m in this version).
//!
//! Models absent from the table return cost=0.0 and emit a tracing warning.

/// Token counts extracted from a single assistant message.
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

/// Per-model price row (USD per million tokens).
#[derive(Debug, Clone, Copy)]
struct PriceRow {
    input: f64,
    output: f64,
    cache_write: f64,
    cache_read: f64,
}

impl PriceRow {
    const fn new(input: f64, output: f64, cache_write: f64, cache_read: f64) -> Self {
        Self {
            input,
            output,
            cache_write,
            cache_read,
        }
    }
}

// Price constants (USD / million tokens)
const OPUS: PriceRow = PriceRow::new(5.00, 25.00, 6.25, 0.50);
const SONNET: PriceRow = PriceRow::new(3.00, 15.00, 3.75, 0.30);
const HAIKU: PriceRow = PriceRow::new(1.00, 5.00, 1.25, 0.10);

/// Return the price row for a given model string, or `None` if unknown.
fn price_row(model: &str) -> Option<PriceRow> {
    // Match by model family (Opus/Sonnet/Haiku) regardless of major version,
    // since price rows are shared across versions within a family.
    if model.contains("opus") {
        Some(OPUS)
    } else if model.contains("sonnet") {
        Some(SONNET)
    } else if model.contains("haiku") {
        Some(HAIKU)
    } else {
        None
    }
}

/// Compute the estimated cost in USD for a single message.
///
/// Returns `0.0` and emits a [`tracing::warn!`] when the model is not in the
/// embedded price table — the caller must NOT panic on unknown models.
pub fn cost(model: &str, usage: &Usage) -> f64 {
    let Some(row) = price_row(model) else {
        tracing::warn!(model, "unknown model — cost set to 0");
        return 0.0;
    };

    const PER_MILLION: f64 = 1_000_000.0;
    (usage.input_tokens as f64 * row.input
        + usage.output_tokens as f64 * row.output
        + usage.cache_creation_tokens as f64 * row.cache_write
        + usage.cache_read_tokens as f64 * row.cache_read)
        / PER_MILLION
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify pricing source: Anthropic Claude pricing / LiteLLM
    /// `model_prices_and_context_window.json` as of 2026-07-03.
    /// Opus-4: $5/M in, $25/M out, cache_write $6.25/M, cache_read $0.50/M.
    /// Sonnet-4: $3/M in, $15/M out, cache_write $3.75/M, cache_read $0.30/M.
    /// Haiku-4: $1/M in, $5/M out, cache_write $1.25/M, cache_read $0.10/M.
    #[test]
    fn test_pricing_source_documented() {
        // This test exists to document the pricing source.
        // If prices change, update both the table above and this test.
        let row = price_row("claude-opus-4-8").unwrap();
        assert!((row.input - 5.00).abs() < f64::EPSILON);
        assert!((row.output - 25.00).abs() < f64::EPSILON);
        assert!((row.cache_write - 6.25).abs() < f64::EPSILON);
        assert!((row.cache_read - 0.50).abs() < f64::EPSILON);

        let row = price_row("claude-sonnet-4-6").unwrap();
        assert!((row.input - 3.00).abs() < f64::EPSILON);
        assert!((row.output - 15.00).abs() < f64::EPSILON);

        let row = price_row("claude-haiku-4-5").unwrap();
        assert!((row.input - 1.00).abs() < f64::EPSILON);
        assert!((row.output - 5.00).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cost_opus_known_fixture() {
        // Hand-computed:
        // input=10000 * 5.0/1_000_000 = 0.05
        // output=200 * 25.0/1_000_000 = 0.005
        // cache_write=5000 * 6.25/1_000_000 = 0.03125
        // cache_read=2000 * 0.50/1_000_000 = 0.001
        // total = 0.08725
        let usage = Usage {
            input_tokens: 10_000,
            output_tokens: 200,
            cache_creation_tokens: 5_000,
            cache_read_tokens: 2_000,
        };
        let c = cost("claude-opus-4-8", &usage);
        assert!((c - 0.08725).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn test_cost_sonnet_known_fixture() {
        // input=1000 * 3.0/1_000_000 = 0.003
        // output=100 * 15.0/1_000_000 = 0.0015
        // total = 0.0045
        let usage = Usage {
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };
        let c = cost("claude-sonnet-4-5", &usage);
        assert!((c - 0.0045).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn test_cost_haiku_known_fixture() {
        // input=500 * 1.0/1_000_000 = 0.0005
        // output=50 * 5.0/1_000_000 = 0.00025
        // cache_write=100 * 1.25/1_000_000 = 0.000125
        // cache_read=200 * 0.10/1_000_000 = 0.00002
        // total = 0.000895
        let usage = Usage {
            input_tokens: 500,
            output_tokens: 50,
            cache_creation_tokens: 100,
            cache_read_tokens: 200,
        };
        let c = cost("claude-haiku-4-5", &usage);
        assert!((c - 0.000895).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn test_cost_sonnet_5_nonzero() {
        // input=1000 * 3.0/1_000_000 = 0.003
        // output=100 * 15.0/1_000_000 = 0.0015
        // total = 0.0045
        let usage = Usage {
            input_tokens: 1_000,
            output_tokens: 100,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };
        let c = cost("claude-sonnet-5", &usage);
        assert!((c - 0.0045).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn test_cost_unknown_model_returns_zero() {
        let usage = Usage {
            input_tokens: 1_000,
            output_tokens: 500,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        };
        let c = cost("gpt-4o", &usage);
        assert_eq!(c, 0.0, "unknown model must return 0");
    }

    #[test]
    fn test_cost_all_zero_usage() {
        let usage = Usage::default();
        let c = cost("claude-opus-4-7", &usage);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn test_all_model_variants_resolve() {
        let models = [
            "claude-opus-4-8",
            "claude-opus-4-7",
            "claude-opus-4-6",
            "claude-opus-4-5",
            "claude-sonnet-4-6",
            "claude-sonnet-4-5",
            "claude-haiku-4-5",
            "claude-sonnet-5",
            "claude-opus-5",
            "claude-haiku-5",
        ];
        for model in models {
            assert!(
                price_row(model).is_some(),
                "model {model} not found in price table"
            );
        }
    }
}
