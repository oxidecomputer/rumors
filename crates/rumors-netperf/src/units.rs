//! The quantities a user writes on the command line and in grid files: link
//! rates, round-trip times, byte sizes, and message counts.
//!
//! Each parses from the same text in both places, so `--rate 100Mbit` and
//! `rate = ["100Mbit"]` mean one thing, and each prints back in a form that
//! parses to the same value.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A one-way link rate in bits per second, or no rate limit at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Rate {
    /// Shaped to this many bits per second.
    Bits(u64),
    /// Not rate-limited: the link adds delay only, and the kernel's own speed
    /// sets the pace.
    Uncapped,
}

impl Rate {
    /// The rate in bytes per second, or `None` when uncapped.
    pub fn bytes_per_second(self) -> Option<f64> {
        match self {
            Rate::Bits(bits) => Some(bits as f64 / 8.0),
            Rate::Uncapped => None,
        }
    }
}

/// Decimal multipliers for rates, as `tc` and network vendors write them.
const RATE_SUFFIXES: [(&str, u64); 4] = [
    ("gbit", 1_000_000_000),
    ("mbit", 1_000_000),
    ("kbit", 1_000),
    ("bit", 1),
];

impl FromStr for Rate {
    type Err = anyhow::Error;

    /// Parses `uncapped`, or a number followed by `bit`, `kbit`, `mbit`, or
    /// `gbit` (decimal multiples, case-insensitive), such as `100Mbit`.
    fn from_str(text: &str) -> Result<Self> {
        let lower = text.trim().to_ascii_lowercase();
        if lower == "uncapped" {
            return Ok(Rate::Uncapped);
        }
        let (number, multiplier) = RATE_SUFFIXES
            .iter()
            .find_map(|(suffix, multiplier)| {
                lower.strip_suffix(suffix).map(|number| (number, *multiplier))
            })
            .ok_or_else(|| {
                anyhow!("`{text}` is not a rate: write `uncapped` or a number with a unit, such as `100Mbit` or `1Gbit`")
            })?;
        let bits =
            scaled(number, multiplier).with_context(|| format!("`{text}` is not a usable rate"))?;
        if bits == 0 {
            bail!("`{text}` is not a usable rate: it must be at least 1 bit/s");
        }
        Ok(Rate::Bits(bits))
    }
}

impl fmt::Display for Rate {
    /// Prints the largest whole unit that divides the rate exactly.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Rate::Uncapped => f.write_str("uncapped"),
            Rate::Bits(bits) => {
                let (suffix, multiplier) = RATE_SUFFIXES
                    .iter()
                    .find(|(_, multiplier)| bits.is_multiple_of(*multiplier))
                    .expect("every rate is a whole number of bits, divisible by 1");
                let unit = match *suffix {
                    "gbit" => "Gbit",
                    "mbit" => "Mbit",
                    "kbit" => "kbit",
                    _ => "bit",
                };
                write!(f, "{}{unit}", bits / multiplier)
            }
        }
    }
}

/// A link's rate in each direction: from side A to side B, and back.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct LinkRate {
    /// The rate of the direction A sends on.
    pub a_to_b: Rate,
    /// The rate of the direction B sends on.
    pub b_to_a: Rate,
}

impl LinkRate {
    /// The same rate in both directions.
    pub fn symmetric(rate: Rate) -> Self {
        Self {
            a_to_b: rate,
            b_to_a: rate,
        }
    }

    /// The faster direction's rate in bytes per second, or `None` when either
    /// direction is uncapped.
    pub fn fastest_bytes_per_second(self) -> Option<f64> {
        Some(
            self.a_to_b
                .bytes_per_second()?
                .max(self.b_to_a.bytes_per_second()?),
        )
    }
}

impl FromStr for LinkRate {
    type Err = anyhow::Error;

    /// Parses one rate for both directions, or `A/B` for an asymmetric link
    /// whose direction from A runs at the first rate (`100Mbit/10Mbit`).
    fn from_str(text: &str) -> Result<Self> {
        match text.split_once('/') {
            None => Ok(Self::symmetric(text.parse()?)),
            Some((a_to_b, b_to_a)) => Ok(Self {
                a_to_b: a_to_b.parse()?,
                b_to_a: b_to_a.parse()?,
            }),
        }
    }
}

impl fmt::Display for LinkRate {
    /// Prints one rate when the directions agree, else `A/B`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a_to_b == self.b_to_a {
            write!(f, "{}", self.a_to_b)
        } else {
            write!(f, "{}/{}", self.a_to_b, self.b_to_a)
        }
    }
}

/// A round-trip time, kept to the microsecond.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Rtt(Duration);

impl Rtt {
    /// The round trip as a [`Duration`].
    pub fn duration(self) -> Duration {
        self.0
    }

    /// The round trip in seconds.
    pub fn seconds(self) -> f64 {
        self.0.as_secs_f64()
    }

    /// A round trip of `duration`, rounded to the microsecond.
    pub fn from_duration(duration: Duration) -> Self {
        Self(Duration::from_micros(
            u64::try_from(duration.as_micros()).unwrap_or(u64::MAX),
        ))
    }
}

/// Multipliers of a microsecond for the time units a round trip may carry.
const TIME_SUFFIXES: [(&str, f64); 3] = [("us", 1.0), ("ms", 1_000.0), ("s", 1_000_000.0)];

impl FromStr for Rtt {
    type Err = anyhow::Error;

    /// Parses a number followed by `us`, `ms`, or `s`, such as `0.1ms`.
    fn from_str(text: &str) -> Result<Self> {
        let lower = text.trim().to_ascii_lowercase();
        let (number, micros) = TIME_SUFFIXES
            .iter()
            .find_map(|(suffix, micros)| lower.strip_suffix(suffix).map(|n| (n, *micros)))
            .ok_or_else(|| {
                anyhow!("`{text}` is not a round-trip time: write a number with `us`, `ms`, or `s`, such as `10ms`")
            })?;
        let value: f64 = number
            .trim()
            .parse()
            .with_context(|| format!("`{text}`: `{number}` is not a number"))?;
        let total = value * micros;
        if !(total.is_finite() && total >= 0.0 && total <= u64::MAX as f64) {
            bail!("`{text}` is not a usable round-trip time");
        }
        Ok(Rtt(Duration::from_micros(total.round() as u64)))
    }
}

impl fmt::Display for Rtt {
    /// Prints whole milliseconds as `Nms` and anything finer in microseconds,
    /// so the text parses back exactly.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let micros = self.0.as_micros();
        if micros.is_multiple_of(1_000) {
            write!(f, "{}ms", micros / 1_000)
        } else {
            write!(f, "{micros}us")
        }
    }
}

/// Binary and decimal multipliers for byte sizes.
const BYTE_SUFFIXES: [(&str, u64); 7] = [
    ("gib", 1 << 30),
    ("mib", 1 << 20),
    ("kib", 1 << 10),
    ("gb", 1_000_000_000),
    ("mb", 1_000_000),
    ("kb", 1_000),
    ("b", 1),
];

/// Parses a byte count: a plain integer, or a number followed by `B`, `kB`,
/// `MB`, `GB` (decimal) or `KiB`, `MiB`, `GiB` (binary), case-insensitive.
pub fn parse_bytes(text: &str) -> Result<u64> {
    let lower = text.trim().to_ascii_lowercase();
    let (number, multiplier) = BYTE_SUFFIXES
        .iter()
        .find_map(|(suffix, multiplier)| lower.strip_suffix(suffix).map(|n| (n, *multiplier)))
        .unwrap_or((lower.as_str(), 1));
    scaled(number, multiplier).with_context(|| {
        format!(
            "`{text}` is not a byte size: write an integer or a number with a unit, such as `32MiB`"
        )
    })
}

/// The whole number `number × multiplier`, exact when `number` is an integer
/// and through floating point otherwise, where the product must still come out
/// whole.
fn scaled(number: &str, multiplier: u64) -> Result<u64> {
    let number = number.trim();
    if let Ok(integer) = number.parse::<u64>() {
        return integer
            .checked_mul(multiplier)
            .ok_or_else(|| anyhow!("{number} × {multiplier} overflows 64 bits"));
    }
    let value: f64 = number
        .parse()
        .with_context(|| format!("`{number}` is not a number"))?;
    let product = value * multiplier as f64;
    if !(product.is_finite() && product >= 0.0 && product < u64::MAX as f64) {
        bail!("{number} × {multiplier} is out of range");
    }
    if product.fract() != 0.0 {
        bail!("{number} × {multiplier} is not a whole number");
    }
    Ok(product as u64)
}

/// Prints a byte count in the largest binary unit that divides it exactly, so
/// the text parses back to the same count.
pub fn format_bytes(bytes: u64) -> String {
    for (suffix, multiplier) in [("GiB", 1u64 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)] {
        if bytes >= multiplier && bytes.is_multiple_of(multiplier) {
            return format!("{}{suffix}", bytes / multiplier);
        }
    }
    bytes.to_string()
}

/// Prints a byte count rounded to three significant figures in a binary unit,
/// for reading rather than for parsing back.
pub fn human_bytes(bytes: f64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes;
    let mut unit = 0;
    while value.abs() >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} B")
    } else {
        format!("{value:.3} {}", units[unit])
    }
}

/// Prints seconds with a unit chosen so the value reads at a glance.
pub fn human_seconds(seconds: f64) -> String {
    if seconds < 1e-3 {
        format!("{:.0} us", seconds * 1e6)
    } else if seconds < 1.0 {
        format!("{:.1} ms", seconds * 1e3)
    } else if seconds < 120.0 {
        format!("{seconds:.2} s")
    } else {
        format!("{:.1} min", seconds / 60.0)
    }
}

/// Parses a message count: a plain integer, or a number in scientific notation
/// such as `1e6`, which must still be a whole number.
pub fn parse_count(text: &str) -> Result<u64> {
    let trimmed = text.trim().replace('_', "");
    if let Ok(count) = trimmed.parse::<u64>() {
        return Ok(count);
    }
    let value: f64 = trimmed.parse().with_context(|| {
        format!("`{text}` is not a count: write an integer such as `100000` or `1e5`")
    })?;
    if !(value.is_finite() && value >= 0.0 && value <= u64::MAX as f64) || value.fract() != 0.0 {
        bail!("`{text}` is not a whole, non-negative count");
    }
    Ok(value as u64)
}

/// Implements serde for a type through its text form, accepting a TOML or JSON
/// integer as well where the type reads plain numbers.
macro_rules! serde_via_text {
    ($type:ty) => {
        impl Serialize for $type {
            /// Writes the text form.
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $type {
            /// Reads the text form.
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

serde_via_text!(Rate);
serde_via_text!(LinkRate);
serde_via_text!(Rtt);

#[cfg(test)]
mod tests;
