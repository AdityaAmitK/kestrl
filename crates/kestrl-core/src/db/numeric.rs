//! Decoding PostgreSQL NUMERIC binary values.

use postgres::types::{FromSql, Type};

pub(super) struct Numeric(pub(super) Vec<u8>);

impl<'a> FromSql<'a> for Numeric {
    fn from_sql(
        _kind: &Type,
        raw: &'a [u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw.to_vec()))
    }

    fn accepts(kind: &Type) -> bool {
        *kind == Type::NUMERIC
    }
}

pub(super) fn numeric_string(bytes: &[u8]) -> String {
    if bytes.len() < 8 {
        return String::new();
    }
    let read_i16 = |offset: usize| i16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
    let count = read_i16(0).max(0) as usize;
    let weight = read_i16(2);
    let sign = read_i16(4);
    let scale = read_i16(6).max(0) as usize;
    let mut digits = Vec::new();
    for index in 0..count {
        let offset = 8 + index * 2;
        if offset + 2 > bytes.len() {
            break;
        }
        digits.push(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
    }
    let integer_groups = usize::try_from(weight + 1).unwrap_or(0);
    let mut integer = String::new();
    for index in 0..integer_groups {
        let digit = digits.get(index).copied().unwrap_or(0);
        if index == 0 {
            integer.push_str(&digit.to_string());
        } else {
            integer.push_str(&format!("{digit:04}"));
        }
    }
    if integer.is_empty() {
        integer.push('0');
    }
    let fraction_groups = scale.div_ceil(4);
    let fraction = (0..fraction_groups)
        .map(|index| {
            format!(
                "{:04}",
                digits.get(integer_groups + index).copied().unwrap_or(0)
            )
        })
        .collect::<String>();
    let fraction = &fraction[..scale.min(fraction.len())];
    let sign = if sign == 0x4000 { "-" } else { "" };
    if scale == 0 {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}
