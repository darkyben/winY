use ropey::Rope;
const RAINBOW: [u32; 8] = [
    0xe06c75, 0xe5a56b, 0xe5c07b, 0x98c379, 0x56b6c2, 0x61afef, 0xa78bfa, 0xc678dd,
];
fn fields(text: &str, separator: char) -> Vec<usize> {
    let mut quoted = false;
    let mut counts = Vec::new();
    let mut count = 0;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            if quoted && chars.peek() == Some(&'"') {
                chars.next();
            } else {
                quoted = !quoted;
            }
        } else if !quoted && c == separator {
            count += 1;
        } else if !quoted && c == '\n' {
            if count > 0 {
                counts.push(count);
            }
            count = 0;
        }
    }
    if count > 0 {
        counts.push(count);
    }
    counts
}
pub fn delimiter(text: &str) -> char {
    [',', ';', '\t', '|']
        .into_iter()
        .max_by_key(|&separator| {
            let counts = fields(text, separator);
            let mut frequencies = std::collections::HashMap::new();
            for n in counts {
                *frequencies.entry(n).or_insert(0usize) += 1;
            }
            frequencies
                .into_iter()
                .map(|(n, frequency)| frequency * 1000 + n.min(999))
                .max()
                .unwrap_or(0)
        })
        .filter(|&separator| !fields(text, separator).is_empty())
        .unwrap_or(',')
}
pub fn colors(text: &Rope, first: usize, end: usize) -> Vec<Vec<(usize, u32)>> {
    let sample = text.slice(..text.len_chars().min(32768)).to_string();
    let separator = delimiter(&sample);
    let mut quoted = false;
    let mut column = 0;
    let mut result = Vec::new();
    for (row, line) in text.lines().take(end).enumerate() {
        let mut runs: Vec<(usize, u32)> = Vec::new();
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            let color = RAINBOW[column % RAINBOW.len()];
            let mut bytes = c.len_utf8();
            if c == '"' {
                if quoted && chars.peek() == Some(&'"') {
                    chars.next();
                    bytes += 1;
                } else {
                    quoted = !quoted;
                }
            } else if !quoted && c == separator {
                column += 1;
            } else if !quoted && c == '\n' {
                column = 0;
            }
            if row >= first {
                if let Some(last) = runs.last_mut().filter(|r| r.1 == color) {
                    last.0 += bytes;
                } else {
                    runs.push((bytes, color));
                }
            }
        }
        if row >= first {
            result.push(runs);
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_separators_ignoring_quoted_content() {
        assert_eq!(delimiter("a;b;c\n1;\"x,y,z\";3\n"), ';');
        assert_eq!(delimiter("a\tb\n1\t2"), '\t');
        assert_eq!(delimiter("a,b\n1,2"), ',');
        assert_eq!(delimiter("a|b\n1|2"), '|');
    }
    #[test]
    fn quotes_and_multiline_fields_keep_column_colors() {
        let rope = Rope::from_str("a,b\n\"x,\"\"y\nmore\",z\n");
        let all = colors(&rope, 0, 3);
        assert_eq!(all[1].len(), 1);
        assert_eq!(all[2].last().unwrap().1, RAINBOW[1]);
        assert_eq!(colors(&rope, 2, 3), all[2..]);
    }
}
