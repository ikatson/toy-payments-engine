use atoi::FromRadix10Checked;

// 4 decimal places.
const PLACES: usize = 4;
const PLACES_MOD: u64 = 10u64.pow(PLACES as u32);

/// A decimal amount, stores both whole and fractional part in a u64.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct Amount(u64);

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.0 / PLACES_MOD;
        let mut fract = self.0 % PLACES_MOD;
        write!(f, "{whole}")?;
        if fract > 0 {
            while fract % 10 == 0 {
                fract /= 10;
            }
            write!(f, ".{fract}")?;
        }
        Ok(())
    }
}

impl Amount {
    pub const fn zero() -> Self {
        Amount(0)
    }

    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        let (whole, size) = u64::from_radix_10_checked(bytes);
        let whole = whole?.checked_mul(PLACES_MOD)?;
        match bytes.get(size).copied() {
            Some(b'.') => {
                let fract_b = &bytes[size + 1..];
                let mut fract = 0;
                for place in 0..PLACES {
                    let digit: u16 = match fract_b.get(place) {
                        Some(b) => atoi::ascii_to_digit(*b)?,
                        None => break,
                    };
                    fract += digit * 10u16.pow(PLACES as u32 - place as u32 - 1);
                }
                // check the remaining bytes that we truncated are valid ascii digits
                for byte in fract_b.get(PLACES..).unwrap_or_default() {
                    atoi::ascii_to_digit::<u8>(*byte)?;
                }

                Some(Amount(whole.checked_add(fract as u64)?))
            }
            Some(_) => None,
            None => Some(Amount(whole)),
        }
    }

    pub fn checked_add(self, rhs: Amount) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Amount)
    }

    pub fn checked_sub(self, rhs: Amount) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Amount)
    }
}

#[cfg(test)]
mod tests {
    use crate::amount::Amount;

    #[test]
    fn test_parse() {
        assert_eq!(Amount::parse(b"0").unwrap(), Amount(0));
        assert_eq!(Amount::parse(b"0.").unwrap(), Amount(0));
        assert_eq!(Amount::parse(b"0.0").unwrap(), Amount(0));
        assert_eq!(Amount::parse(b"0.00").unwrap(), Amount(0));

        assert_eq!(Amount::parse(b"1").unwrap(), Amount(10000));
        assert_eq!(Amount::parse(b"1.").unwrap(), Amount(10000));
        assert_eq!(Amount::parse(b"1.0").unwrap(), Amount(10000));
        assert_eq!(Amount::parse(b"1.1").unwrap(), Amount(11000));
        assert_eq!(Amount::parse(b"1.12").unwrap(), Amount(11200));
        assert_eq!(Amount::parse(b"1.123").unwrap(), Amount(11230));
        assert_eq!(Amount::parse(b"1.1234").unwrap(), Amount(11234));
        assert_eq!(Amount::parse(b"1.12345").unwrap(), Amount(11234));

        // overflow (u64::MAX)
        assert_eq!(Amount::parse(b"18446744073709551615"), None);
        assert_eq!(Amount::parse(b"18446744073709551"), None);
        assert_eq!(
            Amount::parse(b"1844674407370955.1615").unwrap(),
            Amount(u64::MAX)
        );
        assert_eq!(Amount::parse(b"1844674407370955.1616"), None);

        // invalid
        assert_eq!(Amount::parse(b"f"), None);
        assert_eq!(Amount::parse(b"1f"), None);
        assert_eq!(Amount::parse(b"1.f"), None);
        assert_eq!(Amount::parse(b"1.1f"), None);
        assert_eq!(Amount::parse(b"1.12f"), None);
        assert_eq!(Amount::parse(b"1.123f"), None);
        assert_eq!(Amount::parse(b"1.1234f"), None);
        assert_eq!(Amount::parse(b"1.12345f"), None);
    }

    #[test]
    fn test_fmt() {
        for amount in [
            "0",
            "1",
            "1.1",
            "1.12",
            "1.123",
            "1.1234",
            "1844674407370955.1615",
        ] {
            let value = Amount::parse(amount.as_bytes()).unwrap().to_string();
            assert_eq!(
                value, amount,
                "{amount}.parse().to_string(). Expected {amount}, got {value}"
            )
        }
    }
}
