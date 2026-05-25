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
            return Some((val * 1_000_000.0) as u64);
        }
    }
    None
}

pub fn parse_quantization_bits(quant_str: &str) -> Option<f64> {
    let s = quant_str.trim().to_uppercase();

    let tokens = [
        ("F32", 32.0),
        ("F16", 16.0),
        ("BF16", 16.0),
        ("Q8_0", 8.5),
        ("Q6_K", 6.6),
        ("Q5_K_M", 5.5),
        ("Q5_K_S", 5.5),
        ("Q5_0", 5.5),
        ("Q5_1", 5.5),
        ("Q4_K_M", 4.5),
        ("Q4_K_S", 4.5),
        ("Q4_0", 4.5),
        ("Q4_1", 4.5),
        ("Q3_K_L", 3.5),
        ("Q3_K_M", 3.5),
        ("Q3_K_S", 3.5),
        ("Q2_K", 2.6),
        ("IQ4_NL", 4.5),
        ("IQ4_XS", 4.25),
        ("IQ3_M", 3.7),
        ("IQ3_S", 3.5),
        ("IQ3_XXS", 3.0),
        ("IQ2_M", 2.7),
        ("IQ2_S", 2.5),
        ("IQ2_XS", 2.3),
        ("IQ2_XXS", 2.1),
        ("IQ1_M", 1.7),
        ("IQ1_S", 1.5),
        ("MXFP4", 4.0),
    ];

    for (token, bits) in tokens {
        if s.contains(token) {
            return Some(bits);
        }
    }

    if let Some(pos) = s.find('Q') {
        let chars: Vec<char> = s.chars().collect();
        if pos + 1 < chars.len()
            && let Some(digit) = chars[pos + 1].to_digit(10)
        {
            let is_single_digit = if pos + 2 < chars.len() {
                !chars[pos + 2].is_ascii_digit()
            } else {
                true
            };

            if is_single_digit {
                return Some(digit as f64 + 0.5);
            }
        }
    }

    None
}

pub fn sanitize_model_name(name: &str) -> String {
    let part = name.split('/').next_back().unwrap_or(name);

    let base = part.split(':').next().unwrap_or(part);

    let mut clean = base.to_string();
    let patterns = [
        "-GGUF", ".GGUF", "GGUF-", "GGUF.", "-MXFP4", "MXFP4-", "MXFP-", "-MXFP", "-IQ", "-Q4",
        "-Q5", "-Q6", "-Q8", "-i1", "-v1", "gguf-",
    ];

    for p in patterns {
        let upper_clean = clean.to_uppercase();
        if let Some(pos) = upper_clean.find(&p.to_uppercase()) {
            clean.replace_range(pos..pos + p.len(), "");
        }
    }

    clean
        .trim_matches(|c| c == '-' || c == '_' || c == '.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_model_name() {
        assert_eq!(
            sanitize_model_name("hf.co/Felladrin/gguf-MXFP4-gpt-oss-20b-Derestricted:MXFP4_MOE"),
            "gpt-oss-20b-Derestricted"
        );
        assert_eq!(
            sanitize_model_name("hf.co/dphn/Dolphin3.0-Llama3.1-8B-GGUF:Q4_K_M"),
            "Dolphin3.0-Llama3.1-8B"
        );
        assert_eq!(sanitize_model_name("mipan/gpt-oss:20b-v1"), "gpt-oss");
        assert_eq!(
            sanitize_model_name("csalab/sahabatai1:llama3_instruct_Q4_K_M"),
            "sahabatai1"
        );
        assert_eq!(sanitize_model_name("llama3:latest"), "llama3");
    }

    #[test]
    fn test_parse_parameter_size() {
        assert_eq!(parse_parameter_size("7B"), Some(7_000_000_000));
        assert_eq!(parse_parameter_size("7b"), Some(7_000_000_000));
        assert_eq!(parse_parameter_size("1.7b"), Some(1_700_000_000));
        assert_eq!(parse_parameter_size("135m"), Some(135_000_000));
        assert_eq!(parse_parameter_size("Invalid"), None);
    }

    #[test]
    fn test_parse_quantization_bits() {
        assert_eq!(
            parse_quantization_bits(
                "hf.co/Felladrin/gguf-MXFP4-gpt-oss-20b-Derestricted:MXFP4_MOE"
            ),
            Some(4.0)
        );
        assert_eq!(
            parse_quantization_bits("hf.co/dphn/Dolphin3.0-Llama3.1-8B-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(parse_quantization_bits("mipan/gpt-oss:20b-v1"), None);
        assert_eq!(
            parse_quantization_bits("mipan/llama3-instruct:8b-heretic"),
            None
        );
        assert_eq!(
            parse_quantization_bits("hf.co/Jinx-org/Jinx-gpt-oss-20b-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits("hf.co/mradermacher/gpt-oss-20B-jail-broke-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits(
                "hf.co/mradermacher/Qwen3-30B-A3B-abliterated-erotic-GGUF:Q4_K_M"
            ),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits("hf.co/mradermacher/gpt-oss-20b-base-i1-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits("hf.co/mradermacher/gpt-oss-20B-jail-broke-i1-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits("danielsheep/gpt-oss-20b-Unsloth:UD-Q6_K_XL"),
            Some(6.6)
        );
        assert_eq!(parse_quantization_bits("smollm2:1.7b"), None);
        assert_eq!(parse_quantization_bits("phi3:mini"), None);
        assert_eq!(
            parse_quantization_bits("hf.co/mradermacher/Lumimaid-v0.2-8B-i1-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits("hf.co/mradermacher/Harmonic-Lumina-12B-i1-GGUF:Q4_K_M"),
            Some(4.5)
        );
        assert_eq!(
            parse_quantization_bits(
                "hf.co/mradermacher/Dirty-Muse-Writer-v01-Uncensored-Erotica-NSFW-i1-GGUF:Q6_K"
            ),
            Some(6.6)
        );
        assert_eq!(
            parse_quantization_bits("csalab/sahabatai1:llama3_instruct_Q4_K_M"),
            Some(4.5)
        );

        assert_eq!(parse_quantization_bits("smollm2:1.7b"), None);
        assert_eq!(parse_quantization_bits("smollm2:135m"), None);

        assert_eq!(parse_quantization_bits("Q8_0"), Some(8.5));
        assert_eq!(parse_quantization_bits("IQ2_XS"), Some(2.3));
        assert_eq!(parse_quantization_bits("model:Q5"), Some(5.5));
    }
}
