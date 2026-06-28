//! Fixed-point price representation, confidence intervals, and status.

use serde::{Deserialize, Serialize};

/// Whether a feed is currently usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceStatus {
    /// No reliable price is available.
    Unknown,
    /// The feed is live and tradeable.
    Trading,
    /// The market is halted; the last price may be stale.
    Halted,
}

impl PriceStatus {
    /// A stable, machine-readable code (shared across language bindings).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Trading => "trading",
            Self::Halted => "halted",
        }
    }

    /// Whether reads should be allowed in this status.
    #[must_use]
    pub fn is_tradeable(self) -> bool {
        matches!(self, Self::Trading)
    }
}

impl Default for PriceStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// A fixed-point price with an explicit confidence interval.
///
/// The real value is `mantissa × 10^expo`; the confidence interval half-width is
/// `conf × 10^expo`. Keeping the mantissa/exponent split (rather than a float)
/// preserves exact on-chain semantics across language bindings.
///
/// # Examples
/// ```
/// use pricefeed_types::Price;
/// // $65,000.00 with ±$5.00 at expo -2.
/// let p = Price::new(6_500_000, 500, -2);
/// assert!((p.value() - 65_000.0).abs() < 1e-9);
/// let (lo, hi) = p.confidence_interval();
/// assert!((lo - 64_995.0).abs() < 1e-9);
/// assert!((hi - 65_005.0).abs() < 1e-9);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    /// The price mantissa (signed).
    pub mantissa: i64,
    /// The confidence half-width, in the same `expo` units (unsigned).
    pub conf: u64,
    /// The power-of-ten exponent applied to `mantissa` and `conf`.
    pub expo: i32,
}

impl Price {
    /// Creates a price from its raw parts.
    #[must_use]
    pub fn new(mantissa: i64, conf: u64, expo: i32) -> Self {
        Self {
            mantissa,
            conf,
            expo,
        }
    }

    /// The scale factor `10^expo` as an `f64`.
    #[must_use]
    pub fn scale(&self) -> f64 {
        10f64.powi(self.expo)
    }

    /// The real-number value `mantissa × 10^expo`.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.mantissa as f64 * self.scale()
    }

    /// The confidence half-width as a real number `conf × 10^expo`.
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.conf as f64 * self.scale()
    }

    /// The inclusive confidence interval `(value - conf, value + conf)`.
    #[must_use]
    pub fn confidence_interval(&self) -> (f64, f64) {
        let v = self.value();
        let c = self.confidence();
        (v - c, v + c)
    }

    /// The relative confidence `conf / |mantissa|` (a unitless ratio).
    ///
    /// Returns `f64::INFINITY` when the mantissa is zero so callers can treat an
    /// undefined ratio as "maximally uncertain".
    #[must_use]
    pub fn confidence_ratio(&self) -> f64 {
        if self.mantissa == 0 {
            return f64::INFINITY;
        }
        self.conf as f64 / (self.mantissa.unsigned_abs() as f64)
    }

    /// Whether the relative confidence is within `max_ratio` (e.g. `0.01` = 1%).
    #[must_use]
    pub fn is_confident_within(&self, max_ratio: f64) -> bool {
        self.confidence_ratio() <= max_ratio
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Render with enough decimals to represent the exponent exactly.
        let decimals = if self.expo < 0 {
            self.expo.unsigned_abs() as usize
        } else {
            0
        };
        write!(
            f,
            "{:.*} ± {:.*}",
            decimals,
            self.value(),
            decimals,
            self.confidence()
        )
    }
}
