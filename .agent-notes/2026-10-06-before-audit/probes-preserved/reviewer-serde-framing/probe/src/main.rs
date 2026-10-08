//! Round-trips before's human-readable composites through ecosystem formats.
use before::{Clock, Ranked, Span};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

type R<T> = Result<T, String>;

fn show<T: PartialEq + Debug>(format: &str, ty: &str, enc: R<String>, dec: R<T>, want: &T) {
    let enc = enc.unwrap_or_else(|e| format!("<encode error {e}>"));
    let verdict = match dec {
        Ok(v) if &v == want => "ROUNDTRIP OK".to_string(),
        Ok(v) => format!("MISMATCH {v:?}"),
        Err(e) => format!("ERR {e}"),
    };
    println!("{format:<22} {ty:<7} {verdict}    encoded: {}", enc.escape_debug());
}

fn csv_rt<T: Serialize + DeserializeOwned>(v: &T, headers: bool) -> (R<String>, R<T>) {
    let mut w = csv::WriterBuilder::new().has_headers(headers).from_writer(vec![]);
    if let Err(e) = w.serialize(v) {
        return (Err(e.to_string()), Err("not encoded".into()));
    }
    let data = w.into_inner().map_err(|e| e.to_string()).unwrap();
    let text = String::from_utf8_lossy(&data).into_owned();
    let mut r = csv::ReaderBuilder::new().has_headers(headers).from_reader(&data[..]);
    let dec = match r.deserialize::<T>().next() {
        Some(x) => x.map_err(|e| e.to_string()),
        None => Err("no record".into()),
    };
    (Ok(text), dec)
}

fn rmp_rt<T: Serialize + DeserializeOwned>(v: &T, named: bool) -> (R<String>, R<T>) {
    let mut buf = vec![];
    let res = if named {
        v.serialize(&mut rmp_serde::Serializer::new(&mut buf).with_struct_map().with_human_readable())
    } else {
        v.serialize(&mut rmp_serde::Serializer::new(&mut buf).with_human_readable())
    };
    if let Err(e) = res {
        return (Err(e.to_string()), Err("not encoded".into()));
    }
    let mut d = rmp_serde::Deserializer::new(&buf[..]).with_human_readable();
    (Ok(format!("{buf:02x?}")), T::deserialize(&mut d).map_err(|e| e.to_string()))
}

fn all<T: Serialize + DeserializeOwned + PartialEq + Debug>(ty: &str, v: &T) {
    let e = serde_json::to_string(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()));
    show("json", ty, e, d, v);

    let d = serde_json::to_value(v).map_err(|e| e.to_string()).and_then(|x| serde_json::from_value(x).map_err(|e| e.to_string()));
    show("json Value", ty, Ok(String::new()), d, v);

    let (e, d) = csv_rt(v, true);
    show("csv (headers)", ty, e, d, v);
    let (e, d) = csv_rt(v, false);
    show("csv (no headers)", ty, e, d, v);

    let (e, d) = rmp_rt(v, true);
    show("rmp hr struct_map", ty, e, d, v);
    let (e, d) = rmp_rt(v, false);
    show("rmp hr tuple", ty, e, d, v);

    let e = serde_bencode::to_bytes(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|b| serde_bencode::from_bytes(&b).map_err(|e| e.to_string()));
    show("serde_bencode", ty, e.map(|b| String::from_utf8_lossy(&b).into_owned()), d, v);

    let e = quick_xml::se::to_string(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|s| quick_xml::de::from_str(&s).map_err(|e| e.to_string()));
    show("quick-xml", ty, e, d, v);

    let e = ron::to_string(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|s| ron::from_str(&s).map_err(|e| e.to_string()));
    show("ron", ty, e, d, v);

    let e = serde_yaml::to_string(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|s| serde_yaml::from_str(&s).map_err(|e| e.to_string()));
    show("serde_yaml", ty, e, d, v);

    let e = toml::to_string(v).map_err(|e| e.to_string());
    let d = e.clone().and_then(|s| toml::from_str(&s).map_err(|e| e.to_string()));
    show("toml", ty, e, d, v);
}

fn edge_cases() {
    let cases: [(&str, &str); 6] = [
        ("clock missing version", r#"{"party":"20"}"#),
        ("clock unknown field", r#"{"party":"20","version":"e0","extra":"e0"}"#),
        ("clock repeated party", r#"{"party":"20","party":"20","version":"e0"}"#),
        ("clock array of 1", r#"["20"]"#),
        ("clock array of 3", r#"["20","e0","e0"]"#),
        ("span keys hi first", r#"{"hi":"e0","lo":"e0"}"#),
    ];
    for (name, text) in cases {
        let verdict = if name.starts_with("span") {
            serde_json::from_str::<Span<'static>>(text).map(|v| format!("{v:?}")).map_err(|e| e.to_string())
        } else {
            serde_json::from_str::<Clock>(text).map(|v| format!("{v:?}")).map_err(|e| e.to_string())
        };
        println!("EDGE {name:<24} {text:<46} -> {verdict:?}");
    }
}

fn main() {
    edge_cases();
    let mut clock = Clock::seed();
    let _other = clock.fork();
    clock.tick();
    let lo = clock.version().clone();
    clock.tick();
    let hi = clock.version().clone();
    let span: Span<'static> = lo.span(&hi);
    let ranked: Ranked<'static> = Ranked::from(hi.clone());
    all("Clock", &clock);
    all("Span", &span);
    all("Ranked", &ranked);
}
