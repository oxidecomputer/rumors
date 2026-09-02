<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch during the rumors triage of 2026-09-02; findings about the oxidecomputer/cbor-diag-rs fork surfaced by the capture renderer lane (rulings T138, T140) and its fresh-eyes reviews; not authored, audited, or endorsed by Finch. Read with the ground rules in ../README.md. -->

# cbor-diag fork items

Findings against `oxidecomputer/cbor-diag-rs` at revision `a6a71367`,
the pin the capture renderer uses. None blocks the renderer: each has an
in-tree accommodation named beside it. They are Finch's to land in the
fork; the accommodations can be dissolved once they are.

1. **The parser rejects a trailing comma before `>>`** that the pretty
   printer emits for tag-63 sequences laid out over several lines
   (`parse/diag.rs`, the `<<` arm closes with a bare `tag(">>")`, while
   `definite_array` closes with `opt_comma_tag("]")`). In tree: nothing
   calls `parse_diag` (T140).
2. **Text strings escape only `"` and `\`**; control characters print
   raw, so a text payload can forge a harness header line
   (`encode/diag.rs`, `definite_textstring_to_diag`). Extended
   diagnostic notation uses JSON string syntax, which requires escaping
   them. In tree: the renderer sends text containing a control character
   to its hex fallback.
3. **Width indicators are spelled only for integers, negatives, tags,
   and floats**; the recorded widths of byte-string, text-string, array,
   and map heads are never printed, and the binary parser accepts the
   ill-formed two-byte simple form (`f8 XX` for `XX < 24`), so
   non-canonical items collide in the rendering. In tree: the renderer
   holds every item to canonical form before delegating and sends the
   rest to the hex fallback.
4. **NaN payload bits and sign have no spelling** in the notation. In
   tree: documented as unrepresentable under the crate's `Eq` payload
   contract.
5. **An embedded item that exceeds the remaining depth budget renders as
   a plain byte string** (`24_0(h'…')`) with no marker, while the
   top-level depth stop is commented. Cosmetic; a snapshot reader cannot
   tell a depth stop from an unparseable embed.
