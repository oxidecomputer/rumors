//! Rendering one wire item into RFC 8949 diagnostic-notation-style text.
//!
//! The adapter's events carry the observed item as one text field, so
//! the rendering must stay legible and bounded no matter what crosses
//! the wire: structure is unfolded (arrays, maps, embedded-CBOR tags),
//! rumors' registered atom tags are named, and every dimension of the
//! output — byte-string hex, embedded-item unfolding, nesting depth,
//! and total length — is capped by a constant, never by the input.

use std::fmt::Write;

use ciborium::value::Value;
use rumors::tags::{CLOCK_TAG, PARTY_TAG, VERSION_TAG};

/// The embedded-CBOR tags whose byte strings are unfolded and shown as
/// their contents: tag 24 holds one encoded item, tag 63 an encoded
/// sequence. Diagnostic notation writes both as `<<…>>`.
const TAG_EMBEDDED_ITEM: u64 = 24;
const TAG_EMBEDDED_SEQUENCE: u64 = 63;

/// How many bytes of a byte string are shown as hex before elision.
/// Version and party atoms fit well inside this; supply payloads and
/// digests elide to their prefix plus a length.
const SHOWN_BYTES: usize = 32;

/// How many levels of embedded-CBOR byte strings are re-parsed and
/// unfolded.
///
/// The wire nests two levels today (a supply run holds records); the
/// budget leaves headroom without letting adversarial nesting demand
/// unbounded re-parsing.
const UNFOLD_BUDGET: u8 = 4;

/// How deep into one parsed item's structure the renderer descends.
/// Rendering recurses on the parsed value's shape, so this constant —
/// not the input — bounds the stack.
const DEPTH_BUDGET: u8 = 64;

/// The rendered form's length cap in bytes. Once an item's rendering
/// crosses it, the remainder elides: events stay cheap even when a
/// megabyte supply run is observed.
const LENGTH_BUDGET: usize = 2048;

/// Renders exactly one wire item as diagnostic-notation-style text.
///
/// The hook's contract is one CBOR item per invocation; bytes that are
/// not that (undecodable, or carrying trailing garbage) render as an
/// explicit defect note plus a hex prefix rather than panicking — the
/// adapter observes, it never judges.
pub(crate) fn item(bytes: &[u8]) -> String {
    let mut cursor = std::io::Cursor::new(bytes);
    let mut out = String::new();
    match ciborium::de::from_reader::<Value, _>(&mut cursor) {
        Ok(value) => {
            render(&value, &mut out, UNFOLD_BUDGET, DEPTH_BUDGET);
            let consumed = cursor.position() as usize;
            if consumed != bytes.len() {
                let _ = write!(out, " !trailing({} B)", bytes.len() - consumed);
            }
        }
        Err(_) => {
            out.push_str("!undecodable ");
            hex(bytes, &mut out);
        }
    }
    if out.len() > LENGTH_BUDGET {
        // Truncation must land on a character boundary: the budget is
        // in bytes, and the rendering carries multibyte characters
        // (elision marks, escaped text) that may straddle it.
        let mut cut = LENGTH_BUDGET;
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
        out.push('…');
    }
    out
}

/// Renders one parsed value, appending to `out`.
///
/// `unfold` prices embedded-CBOR re-parses and `depth` prices descent
/// into the value's own structure; both only ever shrink, so the
/// recursion is bounded by the two constants above regardless of
/// input. Output growth is checked against the length budget at every
/// level so a wide value cannot buy unbounded work with small nesting.
fn render(value: &Value, out: &mut String, unfold: u8, depth: u8) {
    if out.len() > LENGTH_BUDGET {
        return;
    }
    let Some(deeper) = depth.checked_sub(1) else {
        out.push('…');
        return;
    };
    match value {
        Value::Integer(n) => {
            let _ = write!(out, "{}", i128::from(*n));
        }
        Value::Float(f) => {
            let _ = write!(out, "{f}");
        }
        Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        Value::Null => out.push_str("null"),
        Value::Text(t) => {
            let _ = write!(out, "\"{}\"", t.escape_debug());
        }
        Value::Bytes(b) => hex(b, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                render(item, out, unfold, deeper);
                if out.len() > LENGTH_BUDGET {
                    return;
                }
            }
            out.push(']');
        }
        Value::Map(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                render(k, out, unfold, deeper);
                out.push_str(": ");
                render(v, out, unfold, deeper);
                if out.len() > LENGTH_BUDGET {
                    return;
                }
            }
            out.push('}');
        }
        Value::Tag(tag, inner) => render_tag(*tag, inner, out, unfold, deeper),
        // `Value` is non-exhaustive; an item kind this renderer does not
        // know is still an observed item, so show its debug form rather
        // than dropping it.
        other => {
            let _ = write!(out, "{other:?}");
        }
    }
}

/// Renders one tagged value: rumors' atom tags by name, embedded-CBOR
/// tags unfolded as `<<…>>`, everything else as `tag(content)`.
fn render_tag(tag: u64, inner: &Value, out: &mut String, unfold: u8, depth: u8) {
    let atom = match tag {
        PARTY_TAG => Some("party"),
        VERSION_TAG => Some("version"),
        CLOCK_TAG => Some("clock"),
        _ => None,
    };
    if let Some(name) = atom {
        let _ = write!(out, "{name}(");
        render(inner, out, unfold, depth);
        out.push(')');
        return;
    }
    if matches!(tag, TAG_EMBEDDED_ITEM | TAG_EMBEDDED_SEQUENCE)
        && let Value::Bytes(encoded) = inner
        && let Some(remaining) = unfold.checked_sub(1)
        && let Some(rendered) = embedded(tag, encoded, remaining, depth)
    {
        let _ = write!(out, "{tag}(<<{rendered}>>)");
        return;
    }
    let _ = write!(out, "{tag}(");
    render(inner, out, unfold, depth);
    out.push(')');
}

/// Re-parses an embedded-CBOR byte string and renders its contents:
/// one item for tag 24, a whole sequence for tag 63.
///
/// `None` when the bytes do not parse to exactly the promised shape —
/// the caller then falls back to the raw byte-string form, which is
/// always honest.
fn embedded(tag: u64, encoded: &[u8], unfold: u8, depth: u8) -> Option<String> {
    let mut cursor = std::io::Cursor::new(encoded);
    let mut out = String::new();
    let mut first = true;
    while (cursor.position() as usize) < encoded.len() {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let value = ciborium::de::from_reader::<Value, _>(&mut cursor).ok()?;
        render(&value, &mut out, unfold, depth);
        if tag == TAG_EMBEDDED_ITEM && (cursor.position() as usize) < encoded.len() {
            return None;
        }
        if out.len() > LENGTH_BUDGET {
            break;
        }
    }
    Some(out)
}

/// Appends a byte string's diagnostic form: full hex up to the shown
/// cap, then an elision with the true length.
fn hex(bytes: &[u8], out: &mut String) {
    let shown = bytes.len().min(SHOWN_BYTES);
    out.push_str("h'");
    for byte in &bytes[..shown] {
        let _ = write!(out, "{byte:02x}");
    }
    out.push('\'');
    if shown < bytes.len() {
        let _ = write!(out, "…({} B)", bytes.len());
    }
}

#[cfg(test)]
mod tests;
