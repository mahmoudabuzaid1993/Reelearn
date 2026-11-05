//! Phone normalisation to E.164. Saudi local formats are accepted because that is
//! what learners type: `05xxxxxxxx`, `5xxxxxxxx`, `9665xxxxxxxx`, `+9665xxxxxxxx`.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("phone number is not valid: {0}")]
pub struct PhoneError(pub String);

pub fn normalize(input: &str) -> Result<String, PhoneError> {
    let digits: String = input
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    let digits = digits.trim_start_matches("00").to_string();
    let bare = digits.trim_start_matches('+');
    if bare.is_empty() || !bare.chars().all(|c| c.is_ascii_digit()) {
        return Err(PhoneError(input.to_string()));
    }
    let e164 = if digits.starts_with('+') || (bare.starts_with("966") && bare.len() == 12) {
        format!("+{bare}")
    } else if bare.starts_with("05") && bare.len() == 10 {
        format!("+966{}", &bare[1..])
    } else if bare.starts_with('5') && bare.len() == 9 {
        format!("+966{bare}")
    } else if bare.len() >= 10 {
        format!("+{bare}")
    } else {
        return Err(PhoneError(input.to_string()));
    };
    let body = &e164[1..];
    if body.len() < 7 || body.len() > 15 || body.starts_with('0') {
        return Err(PhoneError(input.to_string()));
    }
    Ok(e164)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saudi_local_formats_become_e164() {
        assert_eq!(normalize("0500000010").as_deref(), Ok("+966500000010"));
        assert_eq!(normalize("500000010").as_deref(), Ok("+966500000010"));
        assert_eq!(normalize("966500000010").as_deref(), Ok("+966500000010"));
        assert_eq!(
            normalize("+966 50 000 0010").as_deref(),
            Ok("+966500000010")
        );
        assert_eq!(normalize("00966500000010").as_deref(), Ok("+966500000010"));
    }

    #[test]
    fn other_countries_pass_through() {
        assert_eq!(normalize("+971501234567").as_deref(), Ok("+971501234567"));
        assert_eq!(normalize("+923001234567").as_deref(), Ok("+923001234567"));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(normalize("").is_err());
        assert!(normalize("abc").is_err());
        assert!(normalize("123").is_err());
        assert!(normalize("+0123456789").is_err());
    }
}
