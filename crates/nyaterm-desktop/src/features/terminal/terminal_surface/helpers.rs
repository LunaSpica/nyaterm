pub(super) fn format_skipped_count(value: u64) -> String {
    // Lightweight thousands separators for the performance overlay.
    let raw = value.to_string();
    let mut out = String::new();
    for (i, ch) in raw.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}
