pub(super) fn parse_similarity(value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| "similarity must be a number between 0.0 and 1.0".to_owned())?;
    validate_similarity(parsed)
}

pub(crate) fn validate_similarity(value: f64) -> Result<f64, String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err("similarity must be a finite value between 0.0 and 1.0".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_similarity;

    #[test]
    fn rejects_non_finite_and_out_of_range_values() {
        for value in ["NaN", "inf", "-0.1", "1.1"] {
            assert!(parse_similarity(value).is_err(), "accepted {value}");
        }
        assert_eq!(parse_similarity("0.8"), Ok(0.8));
    }
}
