pub fn estimate_vram_usage(params: u64, quant_bits: f64) -> u64 {
    // Formula: (Params * Quant_Bits / 8)
    // Result is in bytes.
    (params as f64 * quant_bits / 8.0) as u64
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
