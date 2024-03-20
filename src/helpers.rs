pub fn name_to_id(mut x: &str) -> String {
    if let Some((l, r)) = x.split_once('{') {
        if let Some((_, r)) = r.split_once('#') {
            let (l, _) = r.split_once('}').unwrap();

            let val = l
                .split_once(|c: char| c.is_whitespace())
                .map(|(l, _)| l)
                .unwrap_or(l);

            return val.to_string();
        } else {
            x = l
        }
    }

    let mut s = String::with_capacity(x.len());
    let mut ws_skip = false;

    for c in x.chars() {
        if c.is_alphanumeric() {
            s.extend(c.to_lowercase())
        } else if c.is_whitespace() {
            if !ws_skip {
                s.push('-');
            }
            ws_skip = true;
            continue;
        } else if c == '_' || c == '-' {
            s.push(c)
        }
        ws_skip = false;
    }

    s
}
