/// Parses a parameter size string (e.g., "7B", "70B") into a raw number of parameters.
pub fn parse_parameter_size(size_str: &str) -> Option<u64> {
    let s = size_str.trim().to_uppercase();
    if let Some(idx) = s.find('B') {
        let num_part = &s[..idx];
        if let Ok(val) = num_part.parse::<f64>() {
            return Some((val * 1_000_000_000.0) as u64);
        }
    }
    if let Some(idx) = s.find('M') {
        let num_part = &s[..idx];
        if let Ok(val) = num_part.parse::<f64>() {
            // M is Million
            return Some((val * 1_000_000.0) as u64);
        }
    }
    None
}

/// Parses a quantization string (e.g., "Q4_0", "F16") into bits per weight (approximate).
pub fn parse_quantization_bits(quant_str: &str) -> Option<f64> {
    let s = quant_str.trim().to_uppercase();

    if s == "F16" {
        return Some(16.0);
    } else if s == "F32" {
        return Some(32.0);
    } else if s.starts_with('Q') {
        // Q4_0, Q5_K_M, Q8_0
        if let Some(digit) = s.chars().nth(1)
            && let Some(d) = digit.to_digit(10)
        {
            return Some(d as f64);
        }
    }

    None
}
