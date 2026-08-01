//! The workspace's shared complexity-claims machinery: structured bounds
//! rendered to one normative rustdoc line per claim site, plus the source
//! scanners that let a test bind prose to the roster byte for byte.
//!
//! Rustdoc cost prose cannot be checked; a rendered line can. Each
//! consuming crate keeps a *claims roster* — one committed row per public
//! operation, carrying a [`Bound`] and the [`Site`] of its `# Complexity`
//! section — and a binding test that byte-compares the section's terminal
//! line against [`Bound::render`]'s output. Editing a documented class
//! without the roster (or vice versa) is then a named failure, and the
//! normative sentence at every site is the roster's own rendering, never
//! hand-drifted prose. The pieces here are the crate-agnostic layer:
//!
//! - [`Bound`] and [`Bound::render`]: pure data-to-text, with nothing read
//!   from any crate, so every roster shares one vocabulary of templates
//!   (and one escape hatch, [`Bound::Custom`], whose every use states its
//!   reason as committed data).
//! - [`SourceSpec`] and [`extract_public_fns`]: the public-surface
//!   extractor, so a roster's totality test can hold "every public
//!   operation has exactly one claim" in both directions.
//! - [`doc_index`] and [`DocIndex::section`]: the doc-block scanner that
//!   locates each site's `# Complexity` section for the byte-compare.
//! - [`test_fns`]: the witness scanner, so a roster can require its cited
//!   evidence tests to exist as `#[test]`-attributed items, by name.
//!
//! Everything crate-specific — the roster rows themselves, class
//! vocabularies, evidence bindings — lives in each consuming crate's own
//! claims module, which also defines the variables its rendered lines use
//! (`n`, operand limbs, held digits, ...).
//!
//! # Line discipline
//!
//! The scanners are line scans, not parsers, resting on rustfmt-normalized
//! shape: `impl` headers and `pub mod name {` blocks at column 0, inherent
//! methods and module-block functions at one indent, and a doc block as
//! the contiguous `///` run (attributes transparent) directly above its
//! item. A `pub fn` at an unexpected position panics rather than silently
//! vanishing — the extractor must never under-report the surface it
//! exists to pin.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests;

/// A public-API source file a scanner walks, with the naming context the
/// file cannot carry itself.
pub struct SourceSpec {
    /// Path relative to the consuming crate's manifest directory.
    pub path: &'static str,
    /// Namespace for module-level `pub fn`s (`None`: the file must have
    /// none).
    pub module_prefix: Option<&'static str>,
    /// Public names for the file's inherent-impl types, keyed by local
    /// type name.
    ///
    /// For types whose local name is not their public path or that live
    /// under a public module. A type absent from the list keeps its
    /// parsed name (and a roster's totality test is what catches a
    /// mapping a new public type still needs).
    pub type_overrides: &'static [(&'static str, &'static str)],
}

/// The roster-facing name of one inherent-impl type: its
/// [`type_overrides`](SourceSpec::type_overrides) mapping, or the local
/// name itself.
fn public_type_name<'a>(spec: &SourceSpec, local: &'a str) -> &'a str {
    spec.type_overrides
        .iter()
        .find(|(from, _)| *from == local)
        .map_or(local, |(_, to)| *to)
}

/// Extract the `pub fn` surface from `specs` under `root`, named as a
/// roster names it: `Type::fn` inside an inherent impl block, `mod::fn`
/// inside a column-0 `pub mod` block, `module_prefix::fn` at file top
/// level.
///
/// Trait impl blocks (headers containing ` for `) cannot hold `pub fn`s
/// and are skipped. A `pub fn` at an unexpected position, or an `impl`
/// block nested inside a `pub mod` block, panics rather than silently
/// vanishing from the listing.
pub fn extract_public_fns(root: &Path, specs: &[SourceSpec]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for spec in specs {
        let path = root.join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        // The public name of the current inherent impl block, if inside one.
        let mut current_type: Option<String> = None;
        // The name of the current column-0 `pub mod` block, if inside one.
        let mut current_mod: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    current_type = None; // trait impl: cannot hold `pub fn`
                } else {
                    current_type = parse_impl_self_type(rest)
                        .map(|name| public_type_name(spec, &name).to_owned());
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub mod ") {
                if line.ends_with('{') {
                    current_mod = Some(fn_name(rest).to_owned());
                }
                continue;
            }
            if line == "}" {
                current_type = None;
                current_mod = None;
                continue;
            }
            if current_mod.is_some() && line.starts_with("    impl") {
                panic!(
                    "{}: an impl block nested inside a `pub mod` block is beyond \
                     the extractor's line discipline",
                    spec.path
                );
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                let name = fn_name(rest);
                let context = current_type.as_deref().or(current_mod.as_deref());
                let ty = context.unwrap_or_else(|| {
                    panic!(
                        "{}: `pub fn {name}` outside an inherent impl or pub mod block",
                        spec.path
                    )
                });
                out.insert(format!("{ty}::{name}"));
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub fn ") {
                let name = fn_name(rest);
                let prefix = spec.module_prefix.unwrap_or_else(|| {
                    panic!("{}: unexpected module-level `pub fn {name}`", spec.path)
                });
                out.insert(format!("{prefix}::{name}"));
            }
        }
    }
    out
}

/// The self-type name from an impl header's remainder (after `impl`):
/// skip a balanced generics list, then read the first identifier.
pub fn parse_impl_self_type(rest: &str) -> Option<String> {
    let mut chars = rest.chars().peekable();
    if chars.peek() == Some(&'<') {
        let mut depth = 0usize;
        for c in chars.by_ref() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let name: String = chars
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The function name from the remainder after `pub fn `.
pub fn fn_name(rest: &str) -> &str {
    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
}

/// Where an operation's `# Complexity` section lives.
#[derive(Debug, Clone, Copy)]
pub enum Site {
    /// The doc block of the `pub fn` the surface extractor names; the
    /// operation's own name locates it.
    Fn,
    /// The doc block of a `pub struct` — `(file, local type name)`.
    TypeDoc(&'static str, &'static str),
    /// The module doc (`//!`) of the named file.
    ModuleDoc(&'static str),
    /// The doc block of a trait/operator impl — `(file, a substring of
    /// the impl header line)`.
    ImplDoc(&'static str, &'static str),
}

/// One operation's structured bound: the data behind the rendered
/// `**Complexity**:` terminal line.
///
/// The template vocabulary is uniform across a consuming roster, whose
/// module doc defines its variables; a bare `O(...)` covers time and
/// space, and forms that split the two say so. Rendered lines carry
/// upper bounds only: a proven lower bound is stated in the site's
/// section prose, above the line, never in the rendered headline.
/// [`Bound::Custom`] is the escape hatch for a row whose honest bound
/// fits no template — every use states its reason beside the line, as
/// committed data.
// The variant deliberately carries a class's own name (`MulBound` is the
// claims vocabulary's multiplication-bound class), so the lint's suffix
// rule loses to the one-name-per-concept rule here.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Word-scale work: `O(1)`.
    Constant,
    /// Linear in the packed input: `O(n)`.
    Linear,
    /// Linear in a packed operand pair: `O(a + b)`.
    LinearPair,
    /// A text rendering: packed input `n` plus mandatory text output
    /// `t`.
    TextRender,
    /// A text parse: text input `t` plus mandatory packed output `n`.
    TextParse,
    /// The balanced n-ary reduction: `O(D log k)` time, `O(D)` space.
    Fold,
    /// The indexed fold: [`Bound::Fold`] plus the per-node search
    /// allowance over the accumulator (`B log n`).
    FoldSearch,
    /// The multiplication-bound time claim on one operand — the worst
    /// case and the width-bounded regime — where `M(·)` is an
    /// arithmetic backend's integer-multiplication bound.
    ///
    /// Any proven lower bound lives in the site's section prose, per
    /// the upper-bounds-only rendering rule.
    MulBound,
    /// [`Bound::MulBound`] over an operand pair.
    MulBoundPair,
    /// The escape hatch: the honest bound fits no template. `line` is
    /// rendered verbatim after the `**Complexity**:` lead; `reason`
    /// states, as committed data, why no template fits.
    Custom {
        line: &'static str,
        reason: &'static str,
    },
}

impl Bound {
    /// The rendered terminal line: the one normative sentence a binding
    /// test byte-compares against the section's last line.
    pub fn render(self) -> String {
        let body = match self {
            Bound::Constant => "`O(1)`.",
            Bound::Linear => "`O(n)`.",
            Bound::LinearPair => "`O(a + b)`.",
            Bound::TextRender => "`O(n + t)`.",
            Bound::TextParse => "`O(t + n)`.",
            Bound::Fold => "`O(D log k)` time, `O(D)` space.",
            Bound::FoldSearch => "`O(D log k + B log n)` time, `O(D)` space.",
            Bound::MulBound => {
                "`O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)` with \
                 width-bounded parked drifts."
            }
            Bound::MulBoundPair => {
                "`O(a + b)` space; time `O(M(a + b) · log (a + b))` worst case, \
                 `O((a + b) log (a + b))` with width-bounded parked drifts."
            }
            Bound::Custom { line, .. } => line,
        };
        format!("**Complexity**: {body}")
    }
}

/// One prose check: a site whose `# Complexity` section must end with
/// the bound's rendered line, verbatim.
pub struct Check {
    pub site: Site,
    pub bound: Bound,
}

/// The `# Complexity` sections a set of surface files carry, scanned
/// from source with the extractor's line discipline.
pub struct DocIndex {
    /// `Type::fn` / `module::fn` name → its doc block's Complexity
    /// section, if the block has one.
    pub fns: BTreeMap<String, Option<String>>,
    /// `(file, local type name)` → the `pub struct`'s section.
    pub structs: BTreeMap<(String, String), Option<String>>,
    /// `(file, impl header line)` → the impl's section, for every
    /// documented column-0 impl.
    pub impls: Vec<(String, String, Option<String>)>,
    /// file → the module doc's section.
    pub modules: BTreeMap<String, Option<String>>,
}

impl DocIndex {
    /// The section at `site`, or an error naming what is missing.
    pub fn section(&self, op: &str, site: Site) -> Result<&str, String> {
        let found = match site {
            Site::Fn => self
                .fns
                .get(op)
                .ok_or_else(|| format!("{op}: no `pub fn` doc block found by the scanner"))?,
            Site::TypeDoc(file, ty) => self
                .structs
                .get(&(file.to_owned(), ty.to_owned()))
                .ok_or_else(|| format!("{op}: no `pub struct {ty}` found in {file}"))?,
            Site::ModuleDoc(file) => self
                .modules
                .get(file)
                .ok_or_else(|| format!("{op}: no module doc found in {file}"))?,
            Site::ImplDoc(file, header) => {
                let mut matches = self
                    .impls
                    .iter()
                    .filter(|(f, h, _)| f == file && h.contains(header));
                let (_, _, section) = matches.next().ok_or_else(|| {
                    format!("{op}: no impl header containing `{header}` in {file}")
                })?;
                if matches.next().is_some() {
                    return Err(format!(
                        "{op}: impl header substring `{header}` is ambiguous in {file}"
                    ));
                }
                section
            }
        };
        found.as_deref().ok_or_else(|| {
            format!("{op}: the doc block at its roster site has no `# Complexity` section")
        })
    }
}

/// Scan every surface file in `specs` under `root` for doc blocks and
/// their `# Complexity` sections.
///
/// The same rustfmt-normalized line discipline as [`extract_public_fns`]:
/// column-0 `impl` headers open inherent or trait impls, column-0
/// `pub mod` lines open module blocks, `pub fn` appears at column 0
/// (module level) or one indent (inherent methods and module-block
/// functions), and a doc block is the contiguous `///` run (attributes
/// transparent) directly above its item.
pub fn doc_index(root: &Path, specs: &[SourceSpec]) -> DocIndex {
    let mut index = DocIndex {
        fns: BTreeMap::new(),
        structs: BTreeMap::new(),
        impls: Vec::new(),
        modules: BTreeMap::new(),
    };
    for spec in specs {
        let path = root.join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let mut module_doc = String::new();
        let mut doc = String::new();
        let mut current_type: Option<String> = None;
        let mut current_mod: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("//!") {
                module_doc.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                module_doc.push('\n');
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("///") {
                doc.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                doc.push('\n');
                continue;
            }
            // Attributes and plain comments sit between a doc block and
            // its item without detaching it (rustc ignores both), so the
            // scan treats them as transparent.
            if trimmed.starts_with("#[") || trimmed.starts_with("#!") || trimmed.starts_with("//") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    index
                        .impls
                        .push((spec.path.to_owned(), line.to_owned(), section_of(&doc)));
                    current_type = None;
                } else {
                    current_type = parse_impl_self_type(rest);
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub mod ") {
                if line.ends_with('{') {
                    current_mod = Some(fn_name(rest).to_owned());
                }
                doc.clear();
                continue;
            }
            if line == "}" {
                current_type = None;
                current_mod = None;
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                let context = match current_type.as_deref() {
                    Some(ty) => Some(public_type_name(spec, ty)),
                    None => current_mod.as_deref(),
                };
                if let Some(ty) = context {
                    let name = format!("{ty}::{}", fn_name(rest));
                    index.fns.insert(name, section_of(&doc));
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub fn ") {
                if let Some(prefix) = spec.module_prefix {
                    let name = format!("{prefix}::{}", fn_name(rest));
                    index.fns.insert(name, section_of(&doc));
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub struct ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                index
                    .structs
                    .insert((spec.path.to_owned(), name), section_of(&doc));
                doc.clear();
                continue;
            }
            doc.clear();
        }
        index
            .modules
            .insert(spec.path.to_owned(), section_of(&module_doc));
    }
    index
}

/// The `# Complexity` section of one doc block: the lines from its
/// heading to the next heading or example fence. [`None`] when the block
/// has no such section.
pub fn section_of(doc: &str) -> Option<String> {
    let mut lines = doc.lines();
    lines.by_ref().find(|l| l.trim() == "# Complexity")?;
    let section: Vec<&str> = lines
        .take_while(|l| !l.trim_start().starts_with("# ") && !l.trim_start().starts_with("```"))
        .collect();
    Some(section.join("\n"))
}

/// Every `#[test]`-attributed function name in a source file.
///
/// The witness scanner behind rosters' evidence bindings:
/// attribute-gated, so a prose mention of a deleted test never counts as
/// its existence, and cfg attributes between `#[test]` and the fn keep
/// the arming.
pub fn test_fns(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            continue;
        }
        if armed {
            if let Some(rest) = t.strip_prefix("fn ") {
                if let Some(name) = rest.split('(').next() {
                    names.insert(name.to_string());
                }
            }
            armed = false;
        }
    }
    names
}
