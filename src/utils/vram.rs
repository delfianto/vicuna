use crate::api::modelfile;

pub fn estimate_vram_usage(params: u64, quant_bits: f64) -> u64 {
    // Formula: (Params * Quant_Bits / 8)
    // Result is in bytes.
    // Add 10% overhead for context (rough estimate without full config)
    let weights = (params as f64 * quant_bits / 8.0) as u64;
    weights + (weights / 10)
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn parse_model_params(s: &str) -> u64 {
    modelfile::parse_parameter_size(s).unwrap_or(0)
}

pub fn parse_quantization(s: &str) -> f64 {
    modelfile::parse_quantization_bits(s).unwrap_or(16.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1024 B");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_parse_model_params() {
        assert_eq!(parse_model_params("7B"), 7_000_000_000);
        assert_eq!(parse_model_params("13B"), 13_000_000_000);
        assert_eq!(parse_model_params("70B"), 70_000_000_000);
        assert_eq!(parse_model_params("7b"), 7_000_000_000);
    }

    #[test]
    fn test_parse_quantization() {
        assert_eq!(parse_quantization("Q4_0"), 4.5);
        assert_eq!(parse_quantization("F16"), 16.0);
        assert_eq!(parse_quantization("Unknown"), 16.0);
    }

    #[test]
    fn test_estimate_vram_usage() {
        let vram = estimate_vram_usage(7_000_000_000, 4.5);
        assert!(vram > 4_000_000_000);
        assert!(vram < 5_000_000_000);
    }
}
