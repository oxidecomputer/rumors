//! Formats the committed fuelscape widget datasets into rustdoc islands.
//!
//! Inputs, all committed: `fuelscape/` (the `fuelscape-widget-data`
//! documents the `before-fuelscape` compactor derives from a measuring
//! dump; that crate owns the format, the binning, and the two-ways
//! verification against the dump) and `docs/` (the widget's stylesheet
//! and script, plus their derived `--html-in-header` concatenation).
//!
//! Outputs: `$OUT_DIR/fuelscapes/<op>.html` — one single-line
//! `<details>` island per operation, pulled into a `# Complexity`
//! section via
//! `#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/<op>.html"))]`
//! — and `$OUT_DIR/fuelscapes/index`, the emitted operation names one
//! per line, which the doc-attachment totality test compares against
//! the sources' include sites.
//!
//! This script is a pure formatter: it re-bins nothing, computes no
//! statistics, and holds no constants the widget or compactor also
//! hold. Every failure here is a defect in the committed repository
//! (malformed data, a stale derived header), so it panics naming the
//! file and check rather than reporting errors to a caller.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=fuelscape");
    println!("cargo:rerun-if-changed=docs/fuelscape.css");
    println!("cargo:rerun-if-changed=docs/fuelscape.js");
    println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
    check_header_fresh();

    let out_dir = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let dst = Path::new(&out_dir).join("fuelscapes");
    std::fs::create_dir_all(&dst).expect("island output directory is creatable");

    let index = read_json(Path::new("fuelscape/index.json"));
    check_banner("fuelscape/index.json", &index, "fuelscape-widget-index");
    let ops = index["ops"]
        .as_array()
        .expect("fuelscape/index.json: ops is a list");
    assert!(
        !ops.is_empty(),
        "fuelscape/index.json: the dataset names no operations"
    );

    let mut emitted = String::new();
    for name in ops {
        let name = name
            .as_str()
            .expect("fuelscape/index.json: op names are strings");
        let file = format!("fuelscape/{name}.json");
        let doc = read_json(Path::new(&file));
        check_banner(&file, &doc, "fuelscape-widget-data");
        assert_eq!(
            doc["meta"], index["meta"],
            "{file}: meta differs from the index's"
        );
        let op = &doc["op"];
        assert_eq!(
            op["op_name"].as_str(),
            Some(name),
            "{file}: holds a different operation than the index claims"
        );
        validate(&file, op);
        let island = island(&doc["meta"], op);
        std::fs::write(dst.join(format!("{name}.html")), island).expect("island file is writable");
        writeln!(emitted, "{name}").expect("string writes are infallible");
    }
    std::fs::write(dst.join("index"), emitted).expect("island index is writable");
}

/// One single-line island: the clickable contract-and-claim summary, the
/// widget's data payload, and a no-JavaScript fallback.
fn island(meta: &serde_json::Value, op: &serde_json::Value) -> String {
    let contract = code_spans(op["contract"].as_str().expect("validated"));
    let claim = op["claim"].as_str().expect("validated");
    // The widget's dataset payload. `default` is the claim: the
    // pre-selected compensation hypothesis.
    let data = serde_json::json!({
        "name": op["op_name"],
        "size_measure": op["size_measure"],
        "commit": meta["commit"],
        "seed": format!("{:#x}", meta["base_seed"].as_u64().expect("validated")),
        "spc": meta["samples_per_column"],
        "sizes": op["sizes"],
        "cols": op["cols"],
        "res": op["res"],
        "default": claim,
    });
    // `</` inside inline JSON would close the island's own script
    // element; JSON strings tolerate the escaped solidus verbatim.
    let data = data.to_string().replace("</", "<\\/");
    let claim_html = escape(claim);
    format!(
        "<details class=\"toggle fs-details\"><summary>{contract} · \
         <span class=\"fs-claim\">measured growth Θ(<code>{claim_html}</code>) \
         in total input bytes · fuelscape</span></summary>\
         <div class=\"fuelscape\"><script type=\"application/json\">{data}</script></div>\
         <noscript><p>The interactive fuelscape requires JavaScript; the claimed \
         bulk growth is Θ({claim_html}) in total input bytes.</p></noscript>\
         </details>\n"
    )
}

/// Renders a contract string's backticked spans as `<code>`, escaping
/// everything: the roster writes contracts in the doc comments' own
/// idiom, and this is the one place that idiom meets HTML.
fn code_spans(contract: &str) -> String {
    let mut out = String::new();
    for (i, part) in contract.split('`').enumerate() {
        if i % 2 == 1 {
            out.push_str("<code>");
            out.push_str(&escape(part));
            out.push_str("</code>");
        } else {
            out.push_str(&escape(part));
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn read_json(path: &Path) -> serde_json::Value {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("{}: {e} (run `just fuelscape-compact`?)", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn check_banner(file: &str, doc: &serde_json::Value, expected: &str) {
    assert_eq!(
        (doc["format"].as_str(), doc["version"].as_u64()),
        (Some(expected), Some(1)),
        "{file}: not a {expected} v1 document"
    );
}

/// The structural checks the compactor's strict reader also enforces,
/// re-checked here because the committed files, not that reader, are
/// this script's input.
fn validate(file: &str, op: &serde_json::Value) {
    for key in ["contract", "claim"] {
        let s = op[key].as_str().unwrap_or("");
        assert!(
            !s.trim().is_empty(),
            "{file}: {key} must be a non-empty string"
        );
    }
    assert!(
        op["res"].as_f64().is_some_and(|r| r.is_finite() && r > 0.0),
        "{file}: res must be finite and positive"
    );
    let sizes = op["sizes"].as_array().expect("sizes is a list");
    let cols = op["cols"].as_array().expect("cols is a list");
    assert!(!sizes.is_empty(), "{file}: the size axis is empty");
    assert_eq!(
        sizes.len(),
        cols.len(),
        "{file}: one histogram per size column"
    );
    assert!(
        sizes.windows(2).all(|w| w[0].as_u64() < w[1].as_u64()),
        "{file}: the size axis must be strictly ascending"
    );
    for col in cols {
        let c = col["c"].as_array().expect("histogram counts are a list");
        let ends_nonzero = c.first().is_some_and(|v| v.as_u64() != Some(0))
            && c.last().is_some_and(|v| v.as_u64() != Some(0));
        assert!(
            ends_nonzero,
            "{file}: histograms must be tight and non-empty"
        );
    }
}

/// The committed header must be exactly the concatenation of the
/// committed stylesheet and script.
///
/// rustdoc flags cannot point into `$OUT_DIR`, so the header is a
/// derived committed file, and this check is what keeps it from rotting.
fn check_header_fresh() {
    let css = std::fs::read_to_string("docs/fuelscape.css").expect("docs/fuelscape.css");
    let js = std::fs::read_to_string("docs/fuelscape.js").expect("docs/fuelscape.js");
    let header =
        std::fs::read_to_string("docs/fuelscape-header.html").expect("docs/fuelscape-header.html");
    assert_eq!(
        header,
        format!("<style>{css}</style>\n<script>{js}</script>\n"),
        "docs/fuelscape-header.html is stale relative to fuelscape.css/fuelscape.js: \
         run `just fuelscape-header`"
    );
}
