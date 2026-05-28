pub(crate) fn string_byte_len(value: &str) -> usize {
    value.len()
}

pub(crate) fn escape_llvm_string(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out
}

pub(crate) fn sanitize_symbol(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn normalize_nominal_name(value: &str) -> String {
    value
        .split_once('[')
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| value.to_string())
}

pub(crate) fn strip_root_namespace(value: &str) -> Option<String> {
    let mut parts = value.split('.');
    let _root = parts.next()?;
    let second = parts.next()?;
    Some(parts.fold(second.to_string(), |mut acc, segment| {
        acc.push('.');
        acc.push_str(segment);
        acc
    }))
}
