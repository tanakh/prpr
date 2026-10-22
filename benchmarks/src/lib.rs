//! JSON parsers written with prpr and with other libraries, all building the same [`Value`].

pub mod nom_json;
pub mod peg_json;
pub mod pest_json;
pub mod prpr_json;
pub mod winnow_json;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap()),
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(a) => Value::Array(a.into_iter().map(Value::from).collect()),
            serde_json::Value::Object(o) => {
                Value::Object(o.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Decodes the body of a JSON string literal, which the parsers have already validated.
pub fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next().unwrap() {
            'b' => out.push('\x08'),
            'f' => out.push('\x0c'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let hi = hex4(&mut chars);
                let c = if (0xd800..0xdc00).contains(&hi) {
                    // A surrogate pair: `\uXXXX\uXXXX`.
                    chars.by_ref().take(2).for_each(drop);
                    0x10000 + ((hi - 0xd800) << 10) + (hex4(&mut chars) - 0xdc00)
                } else {
                    hi
                };
                out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
            }
            c => out.push(c),
        }
    }
    out
}

fn hex4(chars: &mut std::str::Chars<'_>) -> u32 {
    u32::from_str_radix(&chars.take(4).collect::<String>(), 16).unwrap()
}
