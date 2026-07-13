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

pub(super) fn format_bytes(value: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    let value_f = value as f64;
    if value_f >= MIB {
        format!("{:.1} MiB", value_f / MIB)
    } else if value_f >= KIB {
        format!("{:.1} KiB", value_f / KIB)
    } else {
        format!("{value} B")
    }
}
