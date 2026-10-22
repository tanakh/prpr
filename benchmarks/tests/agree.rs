//! All parsers build the same syntax tree.

use prpr_benchmarks::*;

#[test]
fn agree() {
    let dir = std::env::var("PRPR_BENCH_DATA").unwrap_or_else(|_| "data".into());
    let mut docs = vec![
        r#"{"a": [1, -2.5e3, true, false, null], "b\né😀": {"c": ""}}"#.to_string(),
        "[]".to_string(),
        " {} ".to_string(),
    ];
    for name in ["canada", "citm_catalog", "twitter"] {
        if let Ok(text) = std::fs::read_to_string(format!("{dir}/{name}.json")) {
            docs.push(text);
        }
    }
    for (i, doc) in docs.iter().enumerate() {
        let expected = prpr_json::parse(doc);
        for (name, actual) in [
            ("peg", peg_json::parse(doc)),
            ("pest", pest_json::parse(doc)),
            ("nom", nom_json::parse(doc)),
            ("winnow", winnow_json::parse(doc)),
        ] {
            assert!(actual == expected, "{name} disagrees on document {i}");
        }
        // serde_json sorts keys and does not always round floats correctly.
        let serde = Value::from(serde_json::from_str::<serde_json::Value>(doc).unwrap());
        assert!(
            close(&sort_keys(serde), &sort_keys(expected)),
            "serde_json disagrees on document {i}"
        );
    }
}

fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y || (x - y).abs() <= x.abs() * 1e-15,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((k, a), (l, b))| k == l && close(a, b))
        }
        (a, b) => a == b,
    }
}

fn sort_keys(v: Value) -> Value {
    match v {
        Value::Array(a) => Value::Array(a.into_iter().map(sort_keys).collect()),
        Value::Object(o) => {
            let mut o: Vec<_> = o.into_iter().map(|(k, v)| (k, sort_keys(v))).collect();
            o.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(o)
        }
        v => v,
    }
}
