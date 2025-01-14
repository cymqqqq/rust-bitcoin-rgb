// SPDX-License-Identifier: CC0-1.0

//! Implements `FeeRate` and assoctiated features.

#[cfg(feature = "serde")]
pub mod serde;

use core::{fmt, ops};

#[cfg(feature = "arbitrary")]
use arbitrary::{Arbitrary, Unstructured};

use crate::amount::Amount;
use crate::weight::Weight;

/// Represents fee rate.
///
/// This is an integer newtype representing fee rate in `sat/kwu`. It provides protection against mixing
/// up the types as well as basic formatting features.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FeeRate(u64);

impl FeeRate {
    /// 0 sat/kwu.
    ///
    /// Equivalent to [`MIN`](Self::MIN), may better express intent in some contexts.
    pub const ZERO: FeeRate = FeeRate(0);

    /// Minimum possible value (0 sat/kwu).
    ///
    /// Equivalent to [`ZERO`](Self::ZERO), may better express intent in some contexts.
    pub const MIN: FeeRate = FeeRate::ZERO;

    /// Maximum possible value.
    pub const MAX: FeeRate = FeeRate(u64::MAX);

    /// Minimum fee rate required to broadcast a transaction.
    ///
    /// The value matches the default Bitcoin Core policy at the time of library release.
    pub const BROADCAST_MIN: FeeRate = FeeRate::from_sat_per_vb_unchecked(1);

    /// Fee rate used to compute dust amount.
    pub const DUST: FeeRate = FeeRate::from_sat_per_vb_unchecked(3);

    /// Constructs a new [`FeeRate`] from satoshis per 1000 weight units.
    pub const fn from_sat_per_kwu(sat_kwu: u64) -> Self { FeeRate(sat_kwu) }

    /// Constructs a new [`FeeRate`] from satoshis per virtual bytes.
    ///
    /// # Errors
    ///
    /// Returns [`None`] on arithmetic overflow.
    pub fn from_sat_per_vb(sat_vb: u64) -> Option<Self> {
        // 1 vb == 4 wu
        // 1 sat/vb == 1/4 sat/wu
        // sat_vb sat/vb * 1000 / 4 == sat/kwu
        Some(FeeRate(sat_vb.checked_mul(1000 / 4)?))
    }

    /// Constructs a new [`FeeRate`] from satoshis per virtual bytes without overflow check.
    pub const fn from_sat_per_vb_unchecked(sat_vb: u64) -> Self { FeeRate(sat_vb * (1000 / 4)) }

    /// Constructs a new [`FeeRate`] from satoshis per kilo virtual bytes (1,000 vbytes).
    pub const fn from_sat_per_kvb(sat_kvb: u64) -> Self { FeeRate(sat_kvb / 4) }

    /// Returns raw fee rate.
    ///
    /// Can be used instead of `into()` to avoid inference issues.
    pub const fn to_sat_per_kwu(self) -> u64 { self.0 }

    /// Converts to sat/vB rounding down.
    pub const fn to_sat_per_vb_floor(self) -> u64 { self.0 / (1000 / 4) }

    /// Converts to sat/vB rounding up.
    /// TODO: cargo-mutants will try to replace - with /, which results in 1000 / 4 / 1 which is also 250.
    /// Since we're addressing the mutants before introducing the cargo-mutants workflow, come back later
    /// and skip this function in the mutants.toml config file
    pub const fn to_sat_per_vb_ceil(self) -> u64 { (self.0 + (1000 / 4 - 1)) / (1000 / 4) }

    /// Checked multiplication.
    ///
    /// Computes `self * rhs` returning [`None`] if overflow occurred.
    #[must_use]
    pub const fn checked_mul(self, rhs: u64) -> Option<Self> {
        // No `map()` in const context.
        match self.0.checked_mul(rhs) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked division.
    ///
    /// Computes `self / rhs` returning [`None`] if `rhs == 0`.
    #[must_use]
    pub const fn checked_div(self, rhs: u64) -> Option<Self> {
        // No `map()` in const context.
        match self.0.checked_div(rhs) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked weight multiplication.
    ///
    /// Computes the absolute fee amount for a given [`Weight`] at this fee rate.
    /// When the resulting fee is a non-integer amount, the amount is rounded up,
    /// ensuring that the transaction fee is enough instead of falling short if
    /// rounded down.
    ///
    /// [`None`] is returned if an overflow occurred.
    #[must_use]
    pub const fn checked_mul_by_weight(self, weight: Weight) -> Option<Amount> {
        // No `?` operator in const context.
        match self.0.checked_mul(weight.to_wu()) {
            Some(mul_res) => match mul_res.checked_add(999) {
                Some(add_res) => Some(Amount::from_sat(add_res / 1000)),
                None => None,
            },
            None => None,
        }
    }

    /// Checked addition.
    ///
    /// Computes `self + rhs` returning [`None`] if overflow occurred.
    #[must_use]
    pub const fn checked_add(self, rhs: u64) -> Option<Self> {
        // No `map()` in const context.
        match self.0.checked_add(rhs) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked subtraction.
    ///
    /// Computes `self - rhs` returning [`None`] if overflow occurred.
    #[must_use]
    pub const fn checked_sub(self, rhs: u64) -> Option<Self> {
        // No `map()` in const context.
        match self.0.checked_sub(rhs) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Calculates the fee by multiplying this fee rate by weight, in weight units, returning [`None`]
    /// if an overflow occurred.
    ///
    /// This is equivalent to `Self::checked_mul_by_weight()`.
    #[must_use]
    pub fn fee_wu(self, weight: Weight) -> Option<Amount> { self.checked_mul_by_weight(weight) }

    /// Calculates the fee by multiplying this fee rate by weight, in virtual bytes, returning [`None`]
    /// if an overflow occurred.
    ///
    /// This is equivalent to converting `vb` to [`Weight`] using [`Weight::from_vb`] and then calling
    /// `Self::fee_wu(weight)`.
    #[must_use]
    pub fn fee_vb(self, vb: u64) -> Option<Amount> {
        Weight::from_vb(vb).and_then(|w| self.fee_wu(w))
    }
}

/// Alternative will display the unit.
impl fmt::Display for FeeRate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}.00 sat/vbyte", self.to_sat_per_vb_ceil())
        } else {
            fmt::Display::fmt(&self.0, f)
        }
    }
}

impl From<FeeRate> for u64 {
    fn from(value: FeeRate) -> Self { value.to_sat_per_kwu() }
}

impl ops::Add for FeeRate {
    type Output = FeeRate;

    fn add(self, rhs: FeeRate) -> Self::Output { FeeRate(self.0 + rhs.0) }
}
crate::internal_macros::impl_add_for_references!(FeeRate);
crate::internal_macros::impl_add_assign!(FeeRate);

impl ops::Sub for FeeRate {
    type Output = FeeRate;

    fn sub(self, rhs: FeeRate) -> Self::Output { FeeRate(self.0 - rhs.0) }
}
crate::internal_macros::impl_sub_for_references!(FeeRate);
crate::internal_macros::impl_sub_assign!(FeeRate);

/// Computes the ceiling so that the fee computation is conservative.
impl ops::Mul<FeeRate> for Weight {
    type Output = Amount;

    fn mul(self, rhs: FeeRate) -> Self::Output {
        Amount::from_sat((rhs.to_sat_per_kwu() * self.to_wu() + 999) / 1000)
    }
}

impl ops::Mul<Weight> for FeeRate {
    type Output = Amount;

    fn mul(self, rhs: Weight) -> Self::Output { rhs * self }
}

impl ops::Div<Weight> for Amount {
    type Output = FeeRate;

    /// Truncating integer division.
    ///
    /// This is likely the wrong thing for a user dividing an amount by a weight. Consider using
    /// `checked_div_by_weight` instead.
    fn div(self, rhs: Weight) -> Self::Output { FeeRate(self.to_sat() * 1000 / rhs.to_wu()) }
}

impl core::iter::Sum for FeeRate {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        FeeRate::from_sat_per_kwu(iter.map(FeeRate::to_sat_per_kwu).sum())
    }
}

impl<'a> core::iter::Sum<&'a FeeRate> for FeeRate {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a FeeRate>,
    {
        FeeRate::from_sat_per_kwu(iter.map(|f| FeeRate::to_sat_per_kwu(*f)).sum())
    }
}

crate::impl_parse_str_from_int_infallible!(FeeRate, u64, from_sat_per_kwu);

#[cfg(feature = "arbitrary")]
impl<'a> Arbitrary<'a> for FeeRate {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let choice = u.int_in_range(0..=4)?;
        match choice {
            0 => Ok(FeeRate::MIN),
            1 => Ok(FeeRate::BROADCAST_MIN),
            2 => Ok(FeeRate::DUST),
            3 => Ok(FeeRate::MAX),
            _ => Ok(FeeRate(u64::arbitrary(u)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_check() {
        let fee_rate: u64 = u64::from(FeeRate(100));
        assert_eq!(fee_rate, 100_u64);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn addition() {
        let one = FeeRate(1);
        let two = FeeRate(2);
        let three = FeeRate(3);

        assert!(one + two == three);
        assert!(&one + two == three);
        assert!(one + &two == three);
        assert!(&one + &two == three);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn subtract() {
        let three = FeeRate(3);
        let seven = FeeRate(7);
        let ten = FeeRate(10);

        assert_eq!(ten - seven, three);
        assert_eq!(&ten - seven, three);
        assert_eq!(ten - &seven, three);
        assert_eq!(&ten - &seven, three);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn multiply() {
        let two = FeeRate::from_sat_per_vb(2).unwrap();
        let three = Weight::from_vb(3).unwrap();
        let six = Amount::from_sat(6);

        assert_eq!(two * three, six);
    }

    #[test]
    fn add_assign() {
        let mut f = FeeRate(1);
        f += FeeRate(2);
        assert_eq!(f, FeeRate(3));

        let mut f = FeeRate(1);
        f += &FeeRate(2);
        assert_eq!(f, FeeRate(3));
    }

    #[test]
    fn sub_assign() {
        let mut f = FeeRate(3);
        f -= FeeRate(2);
        assert_eq!(f, FeeRate(1));

        let mut f = FeeRate(3);
        f -= &FeeRate(2);
        assert_eq!(f, FeeRate(1));
    }

    #[test]
    fn fee_rate_div_by_weight() {
        let fee_rate = Amount::from_sat(329) / Weight::from_wu(381);
        assert_eq!(fee_rate, FeeRate(863));
    }

    #[test]
    fn checked_add() {
        let f = FeeRate(1).checked_add(2).unwrap();
        assert_eq!(FeeRate(3), f);

        let f = FeeRate(u64::MAX).checked_add(1);
        assert!(f.is_none());
    }

    #[test]
    fn checked_sub() {
        let f = FeeRate(2).checked_sub(1).unwrap();
        assert_eq!(FeeRate(1), f);

        let f = FeeRate::ZERO.checked_sub(1);
        assert!(f.is_none());
    }

    #[test]
    fn fee_rate_const() {
        assert_eq!(0, FeeRate::ZERO.to_sat_per_kwu());
        assert_eq!(u64::MIN, FeeRate::MIN.to_sat_per_kwu());
        assert_eq!(u64::MAX, FeeRate::MAX.to_sat_per_kwu());
        assert_eq!(250, FeeRate::BROADCAST_MIN.to_sat_per_kwu());
        assert_eq!(750, FeeRate::DUST.to_sat_per_kwu());
    }

    #[test]
    fn fee_rate_from_sat_per_vb() {
        let fee_rate = FeeRate::from_sat_per_vb(10).expect("expected feerate in sat/kwu");
        assert_eq!(FeeRate(2500), fee_rate);
    }

    #[test]
    fn fee_rate_from_sat_per_kvb() {
        let fee_rate = FeeRate::from_sat_per_kvb(11);
        assert_eq!(FeeRate(2), fee_rate);
    }

    #[test]
    fn fee_rate_from_sat_per_vb_overflow() {
        let fee_rate = FeeRate::from_sat_per_vb(u64::MAX);
        assert!(fee_rate.is_none());
    }

    #[test]
    fn from_sat_per_vb_unchecked() {
        let fee_rate = FeeRate::from_sat_per_vb_unchecked(10);
        assert_eq!(FeeRate(2500), fee_rate);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic = "attempt to multiply with overflow"]
    fn from_sat_per_vb_unchecked_panic() { FeeRate::from_sat_per_vb_unchecked(u64::MAX); }

    #[test]
    fn raw_feerate() {
        let fee_rate = FeeRate(749);
        assert_eq!(749, fee_rate.to_sat_per_kwu());
        assert_eq!(2, fee_rate.to_sat_per_vb_floor());
        assert_eq!(3, fee_rate.to_sat_per_vb_ceil());
    }

    #[test]
    fn checked_mul() {
        let fee_rate = FeeRate(10).checked_mul(10).expect("expected feerate in sat/kwu");
        assert_eq!(FeeRate(100), fee_rate);

        let fee_rate = FeeRate(10).checked_mul(u64::MAX);
        assert!(fee_rate.is_none());
    }

    #[test]
    fn fee_wu() {
        let fee_overflow = FeeRate(10).fee_wu(Weight::MAX);
        assert!(fee_overflow.is_none());

        let fee_rate = FeeRate::from_sat_per_vb(2).unwrap();
        let weight = Weight::from_vb(3).unwrap();
        assert_eq!(fee_rate.fee_wu(weight).unwrap(), Amount::from_sat(6));
    }

    #[test]
    fn fee_vb() {
        let fee_overflow = FeeRate(10).fee_vb(Weight::MAX.to_wu());
        assert!(fee_overflow.is_none());

        let fee_rate = FeeRate::from_sat_per_vb(2).unwrap();
        assert_eq!(fee_rate.fee_vb(3).unwrap(), Amount::from_sat(6));
    }

    #[test]
    fn checked_weight_mul() {
        let weight = Weight::from_vb(10).unwrap();
        let fee: Amount = FeeRate::from_sat_per_vb(10)
            .unwrap()
            .checked_mul_by_weight(weight)
            .expect("expected Amount");
        assert_eq!(Amount::from_sat(100), fee);

        let fee = FeeRate(10).checked_mul_by_weight(Weight::MAX);
        assert!(fee.is_none());

        let weight = Weight::from_vb(3).unwrap();
        let fee_rate = FeeRate::from_sat_per_vb(3).unwrap();
        let fee = fee_rate.checked_mul_by_weight(weight).unwrap();
        assert_eq!(Amount::from_sat(9), fee);

        let weight = Weight::from_wu(381);
        let fee_rate = FeeRate::from_sat_per_kwu(864);
        let fee = fee_rate.checked_mul_by_weight(weight).unwrap();
        // 381 * 0.864 yields 329.18.
        // The result is then rounded up to 330.
        assert_eq!(fee, Amount::from_sat(330));
    }

    #[test]
    fn checked_div() {
        let fee_rate = FeeRate(10).checked_div(10).expect("expected feerate in sat/kwu");
        assert_eq!(FeeRate(1), fee_rate);

        let fee_rate = FeeRate(10).checked_div(0);
        assert!(fee_rate.is_none());
    }
}
