//! Flattens the kv example into one program, chunk by chunk, and renders each
//! chunk as a code fragment of the tutorial.
//!
//! The example is an ordinary crate whose modules are its chunks: each file
//! declares its modules in teaching order, and a file's own items are a chunk
//! too, placed among the modules it declares wherever the items stand among
//! those declarations. The tutorial presents
//! the same program as one file, built up a chunk at a time, so flattening
//! deletes what only the module structure needs: `mod` declarations, imports
//! through `crate::`, `super::`, or `self::`, visibility, module docs, and
//! imports of names an earlier chunk already imported. Each deletion removes
//! exactly the tokens involved and keeps the rest of the text byte for byte.
//! Flattening also deletes the blank lines between consecutive imports, so a
//! chunk's imports read as one block, in the order the source lists them,
//! however its groups were separated for rustfmt, and it deletes every
//! `#[rustfmt::skip]`, which only tells rustfmt how to treat the example's
//! source.
//!
//! Flattening rejects what deletion alone cannot express: a crate-relative
//! path outside an import, a name bound to two different things across the
//! program, a glob import, or a file whose own items a module declaration
//! splits in two.
//!
//! Each fragment is a `no_run` doctest whose hidden lines hold every earlier
//! chunk, so the doctests compile every prefix of the program: a chunk that
//! refers to anything in a later chunk fails its own fragment.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::Path;

use proc_macro2::{LineColumn, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{AttrStyle, Attribute, Expr, ImplItem, Item, Lit, Meta, UseTree, Visibility};

/// The name of the chunk that holds the crate root's own items.
pub const ROOT: &str = "main";

/// Render the example's manifest as its fragment: the template, byte for
/// byte, in a TOML block that rustdoc does not compile.
pub fn manifest_fragment(template: &str) -> String {
    format!("\n```toml\n{}\n```\n", template.trim_end_matches('\n'))
}

/// One chunk of the example, named by its file's path below the example's
/// directory without the `.rs` extension (`node/seed`, or `node` for
/// `node.rs`), or [`ROOT`] for `main.rs`.
pub struct Chunk {
    /// The chunk's name, which is also its fragment's path.
    pub name: String,
    /// The chunk's complete source text.
    pub source: String,
}

/// Read the example at `dir` and return its chunks in teaching order.
///
/// Each file's modules follow the order the file declares them in. A file's
/// own items form a chunk named after the file, placed among its modules where
/// the items stand among their declarations: before them, as a type's
/// definition precedes the modules that say what it can do; between them, as
/// a type's definition follows the modules defining the types it holds; or
/// after them, as `main.rs`'s items do. A file with no items of its own
/// contributes no chunk.
pub fn chunks(dir: &Path) -> Result<Vec<Chunk>, String> {
    let mut chunks = Vec::new();
    walk(dir, None, &mut chunks)?;
    Ok(chunks)
}

/// Add the chunks of the module at `module` (a path below the crate root, or
/// `None` for the root itself) to `chunks`, in teaching order.
fn walk(dir: &Path, module: Option<&str>, chunks: &mut Vec<Chunk>) -> Result<(), String> {
    let (name, path) = match module {
        None => (ROOT.to_owned(), dir.join("main.rs")),
        Some(module) => (module.to_owned(), dir.join(format!("{module}.rs"))),
    };
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    let file = parse(&source, &name)?;
    let children = declared_modules(&file, &name)?;
    let position = position(&file, &children, &name)?;
    let mut own = position.is_some().then_some(Chunk { name, source });
    for (index, child) in children.iter().enumerate() {
        if position == Some(index) {
            chunks.extend(own.take());
        }
        let child = match module {
            None => child.clone(),
            Some(module) => format!("{module}/{child}"),
        };
        walk(dir, Some(&child), chunks)?;
    }
    chunks.extend(own);
    Ok(())
}

/// Where `file`'s own items stand among the modules it declares, as the
/// number of declarations that precede them, or `None` if it has no items of
/// its own.
///
/// A file's own items are every item except its module declarations and the
/// imports that flattening deletes, which name `children` or start at
/// `crate`, `super`, or `self`. They form one chunk, so no module declaration
/// may stand between two of them.
fn position(file: &syn::File, children: &[String], name: &str) -> Result<Option<usize>, String> {
    let mut declared = 0;
    let mut position = None;
    for item in &file.items {
        match item {
            Item::Mod(_) => declared += 1,
            Item::Use(import) if is_relative(&import.tree, children) => {}
            _ => match position {
                None => position = Some(declared),
                Some(at) if at == declared => {}
                Some(_) => {
                    return Err(format!(
                        "{name}: keep the file's own items together, with no module \
                         declaration among them"
                    ));
                }
            },
        }
    }
    Ok(position)
}

/// Whether every name `tree` imports is reached through `crate`, `super`,
/// `self`, or one of `modules`.
fn is_relative(tree: &UseTree, modules: &[String]) -> bool {
    match tree {
        UseTree::Path(path) => is_relative_start(&path.ident.to_string(), modules),
        UseTree::Name(name) => is_relative_start(&name.ident.to_string(), modules),
        UseTree::Rename(rename) => is_relative_start(&rename.ident.to_string(), modules),
        UseTree::Glob(_) => false,
        UseTree::Group(group) => group.items.iter().all(|tree| is_relative(tree, modules)),
    }
}

/// Whether a path starting at `segment` stays inside the crate.
fn is_relative_start<S: AsRef<str>>(segment: &str, modules: &[S]) -> bool {
    matches!(segment, "crate" | "super" | "self")
        || modules.iter().any(|module| module.as_ref() == segment)
}

/// The modules `file` declares with `mod name;`, in declaration order.
fn declared_modules(file: &syn::File, name: &str) -> Result<Vec<String>, String> {
    let mut modules = Vec::new();
    for item in &file.items {
        if let Item::Mod(module) = item {
            if module.content.is_some() || !module.attrs.is_empty() {
                return Err(format!(
                    "{name}: `mod {}` must be a plain declaration of a file module",
                    module.ident
                ));
            }
            modules.push(module.ident.to_string());
        }
    }
    Ok(modules)
}

/// Parse `source` as a Rust file, naming `name` in any error.
fn parse(source: &str, name: &str) -> Result<syn::File, String> {
    syn::parse_file(source).map_err(|error| format!("{name}: {error}"))
}

/// Flatten every chunk, returning each chunk's text as it appears in the
/// one-file program, in the same order as `chunks`.
pub fn flatten(chunks: &[Chunk]) -> Result<Vec<String>, String> {
    let mut program = Program::default();
    for chunk in chunks {
        let file = parse(&chunk.source, &chunk.name)?;
        program
            .modules
            .extend(declared_modules(&file, &chunk.name)?);
    }
    chunks.iter().map(|chunk| program.flatten(chunk)).collect()
}

/// What the one-file program has bound so far, for deduplicating imports and
/// detecting collisions.
#[derive(Default)]
struct Program {
    /// Every module the example declares, whose paths flattening deletes.
    modules: BTreeSet<String>,
    /// Each imported name, with the path it imports and the chunk that
    /// imported it first.
    imports: BTreeMap<String, (String, String)>,
    /// Each name an item defines, with the chunk that defines it.
    items: BTreeMap<String, String>,
}

impl Program {
    /// Flatten one chunk, binding its new names.
    fn flatten(&mut self, chunk: &Chunk) -> Result<String, String> {
        let file = parse(&chunk.source, &chunk.name)?;
        let lines = Lines::new(&chunk.source);
        let mut edits = Vec::new();

        for attribute in &file.attrs {
            if let AttrStyle::Inner(_) = attribute.style {
                edits.push(lines.whole_lines(attribute.to_token_stream(), String::new()));
            }
        }
        for attribute in rustfmt_skips(&file) {
            edits.push(lines.attribute(attribute));
        }
        for pair in file.items.windows(2) {
            if let [Item::Use(above), Item::Use(below)] = pair {
                edits.extend(lines.blank_lines_between(above, below));
            }
        }
        for item in &file.items {
            match item {
                Item::Mod(module) => {
                    edits.push(lines.whole_lines(module.to_token_stream(), String::new()));
                }
                Item::Use(import) => match self.import(chunk, &import.tree)? {
                    Some(replacement) => {
                        edits.push(lines.whole_lines(import.to_token_stream(), replacement));
                    }
                    None if !matches!(import.vis, Visibility::Inherited) => {
                        edits.push(lines.visibility(&import.vis));
                    }
                    None => {}
                },
                item => {
                    self.define(chunk, item)?;
                    self.reject_crate_paths(chunk, item)?;
                    edits.extend(visibilities(item).map(|vis| lines.visibility(vis)));
                }
            }
        }
        Ok(tidy(&apply(&chunk.source, edits)))
    }

    /// Bind the names an import introduces, returning what replaces the
    /// import in the one-file program.
    ///
    /// It returns nothing if every name is new and so the import stays as
    /// written, the new names alone if only some are, and an empty text if none
    /// are or the import stays inside the crate.
    fn import(&mut self, chunk: &Chunk, tree: &UseTree) -> Result<Option<String>, String> {
        let mut leaves = Vec::new();
        collect_leaves(tree, &mut Vec::new(), &mut leaves, &chunk.name)?;
        let mut kept = Vec::new();
        for leaf in &leaves {
            if is_relative_start(&leaf.path[0], &self.modules.iter().collect::<Vec<_>>()) {
                continue;
            }
            let path = leaf.path.join("::");
            if let Some(chunk_defining) = self.items.get(&leaf.binding) {
                return Err(format!(
                    "{}: importing `{path}` as `{}` collides with the item `{}` in {chunk_defining}",
                    chunk.name, leaf.binding, leaf.binding
                ));
            }
            match self.imports.get(&leaf.binding) {
                Some((earlier, _)) if *earlier == path => {}
                Some((earlier, importer)) => {
                    return Err(format!(
                        "{}: `{}` imports `{path}`, but {importer} already imports `{earlier}` \
                         under that name; qualify one of them in the source",
                        chunk.name, leaf.binding
                    ));
                }
                None => {
                    self.imports
                        .insert(leaf.binding.clone(), (path, chunk.name.clone()));
                    kept.push(leaf);
                }
            }
        }
        if kept.len() == leaves.len() {
            return Ok(None);
        }
        Ok(Some(print_imports(&kept)))
    }

    /// Bind the name `item` defines, rejecting a name already bound.
    fn define(&mut self, chunk: &Chunk, item: &Item) -> Result<(), String> {
        let Some(name) = defined_name(item) else {
            return Ok(());
        };
        if let Some((path, importer)) = self.imports.get(&name) {
            return Err(format!(
                "{}: the item `{name}` collides with `{path}`, which {importer} imports",
                chunk.name
            ));
        }
        if let Some(other) = self.items.insert(name.clone(), chunk.name.clone()) {
            return Err(format!(
                "{}: the item `{name}` collides with the item of that name in {other}",
                chunk.name
            ));
        }
        Ok(())
    }
}

/// Rewrite token-stream spacing (`a :: b :: { C , D }`) as source spacing
/// (`a::b::{C, D}`).
fn normalize_tree(text: &str) -> String {
    text.replace(" :: ", "::")
        .replace(":: ", "::")
        .replace(" ::", "::")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace(" ,", ",")
}

/// One name an import binds.
struct Leaf {
    /// The path of the imported item, `self` resolved to its module.
    path: Vec<String>,
    /// The name the import binds the item to.
    binding: String,
    /// Whether the import named the module as `self` inside a group.
    is_self: bool,
}

/// Collect every name `tree` binds below `prefix`.
fn collect_leaves(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    leaves: &mut Vec<Leaf>,
    chunk: &str,
) -> Result<(), String> {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_leaves(&path.tree, prefix, leaves, chunk)?;
            prefix.pop();
        }
        UseTree::Name(name) if name.ident == "self" => {
            let Some(binding) = prefix.last().cloned() else {
                return Err(format!("{chunk}: `use self;` names no module"));
            };
            leaves.push(Leaf {
                path: prefix.clone(),
                binding,
                is_self: true,
            });
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            leaves.push(Leaf {
                path,
                binding: name.ident.to_string(),
                is_self: false,
            });
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            leaves.push(Leaf {
                path,
                binding: rename.rename.to_string(),
                is_self: false,
            });
        }
        UseTree::Glob(_) => {
            return Err(format!(
                "{chunk}: a glob import (`{}::*`) binds names the tutorial cannot \
                 deduplicate; name them",
                prefix.join("::")
            ));
        }
        UseTree::Group(group) => {
            for tree in &group.items {
                collect_leaves(tree, prefix, leaves, chunk)?;
            }
        }
    }
    Ok(())
}

/// Print `leaves` as `use` items, one per parent module, in first-seen order.
fn print_imports(leaves: &[&Leaf]) -> String {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for leaf in leaves {
        let (parent, name) = if leaf.is_self {
            (leaf.path.join("::"), "self".to_owned())
        } else {
            let (name, parent) = leaf.path.split_last().expect("an import names an item");
            let last = leaf.path.last().expect("an import names an item");
            let name = if *last == leaf.binding {
                name.clone()
            } else {
                format!("{name} as {}", leaf.binding)
            };
            (parent.join("::"), name)
        };
        match groups.iter_mut().find(|(known, _)| *known == parent) {
            Some((_, names)) => names.push(name),
            None => groups.push((parent, vec![name])),
        }
    }
    groups
        .iter()
        .map(|(parent, names)| match names.as_slice() {
            [only] if only == "self" => format!("use {parent};"),
            [only] => format!("use {parent}::{only};"),
            names => format!("use {parent}::{{{}}};", names.join(", ")),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The name a top-level item defines, if it defines one.
fn defined_name(item: &Item) -> Option<String> {
    let ident = match item {
        Item::Const(item) => &item.ident,
        Item::Enum(item) => &item.ident,
        Item::Fn(item) => &item.sig.ident,
        Item::Static(item) => &item.ident,
        Item::Struct(item) => &item.ident,
        Item::Trait(item) => &item.ident,
        Item::Type(item) => &item.ident,
        Item::Union(item) => &item.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

/// Every `#[rustfmt::skip]` among `file`'s items, at any depth: on an item, a
/// field, or a method, or inside a body.
///
/// The file's own inner attributes are left out, since flattening deletes
/// them all already.
fn rustfmt_skips(file: &syn::File) -> Vec<&Attribute> {
    struct Finder<'ast> {
        found: Vec<&'ast Attribute>,
    }
    impl<'ast> Visit<'ast> for Finder<'ast> {
        fn visit_attribute(&mut self, attribute: &'ast Attribute) {
            if let Meta::Path(path) = &attribute.meta
                && path.leading_colon.is_none()
                && path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .eq(["rustfmt", "skip"])
            {
                self.found.push(attribute);
            }
        }
    }
    let mut finder = Finder { found: Vec::new() };
    for item in &file.items {
        finder.visit_item(item);
    }
    finder.found
}

/// Every visibility marker on `item`, its fields, and its associated items.
fn visibilities(item: &Item) -> impl Iterator<Item = &Visibility> {
    let mut found: Vec<&Visibility> = Vec::new();
    match item {
        Item::Const(item) => found.push(&item.vis),
        Item::Enum(item) => found.push(&item.vis),
        Item::Fn(item) => found.push(&item.vis),
        Item::Static(item) => found.push(&item.vis),
        Item::Struct(item) => {
            found.push(&item.vis);
            found.extend(item.fields.iter().map(|field| &field.vis));
        }
        Item::Trait(item) => found.push(&item.vis),
        Item::Type(item) => found.push(&item.vis),
        Item::Union(item) => {
            found.push(&item.vis);
            found.extend(item.fields.named.iter().map(|field| &field.vis));
        }
        Item::Impl(item) => {
            for associated in &item.items {
                match associated {
                    ImplItem::Const(associated) => found.push(&associated.vis),
                    ImplItem::Fn(associated) => found.push(&associated.vis),
                    ImplItem::Type(associated) => found.push(&associated.vis),
                    _ => {}
                }
            }
        }
        _ => {}
    }
    found
        .into_iter()
        .filter(|vis| !matches!(vis, Visibility::Inherited))
}

impl Program {
    /// Reject a path through `crate`, `super`, `self`, or a module of the
    /// example anywhere in `item`, macro arguments included: flattening
    /// deletes the imports such paths rely on, so it cannot keep them
    /// meaningful.
    fn reject_crate_paths(&self, chunk: &Chunk, item: &Item) -> Result<(), String> {
        struct Finder<'a> {
            modules: Vec<&'a String>,
            found: Option<String>,
        }
        impl<'ast> Visit<'ast> for Finder<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                if self.found.is_none()
                    && path.segments.len() > 1
                    && is_relative_start(&path.segments[0].ident.to_string(), &self.modules)
                {
                    self.found = Some(path.to_token_stream().to_string());
                }
                visit::visit_path(self, path);
            }
            fn visit_macro(&mut self, mac: &'ast syn::Macro) {
                if self.found.is_none() {
                    self.found = crate_path_in_tokens(mac.tokens.clone(), &self.modules);
                }
                visit::visit_macro(self, mac);
            }
        }
        let mut finder = Finder {
            modules: self.modules.iter().collect(),
            found: None,
        };
        finder.visit_item(item);
        match finder.found {
            Some(path) => Err(format!(
                "{}: `{}` is a path inside the crate outside an import; import the \
                 item instead",
                chunk.name,
                normalize_tree(&path)
            )),
            None => Ok(()),
        }
    }
}

/// The first path inside the crate in `tokens` (an identifier that starts one,
/// followed by `::`), searching groups.
fn crate_path_in_tokens(tokens: TokenStream, modules: &[&String]) -> Option<String> {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Ident(ident) if is_relative_start(&ident.to_string(), modules) => {
                if let Some(TokenTree::Punct(colon)) = tokens.get(index + 1)
                    && colon.as_char() == ':'
                {
                    return Some(format!("{ident}::…"));
                }
            }
            TokenTree::Group(group) => {
                if let Some(found) = crate_path_in_tokens(group.stream(), modules) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Byte offsets for the positions spans report.
struct Lines<'a> {
    /// The text the positions index.
    source: &'a str,
    /// The byte offset at which each line starts.
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    /// Index the lines of `source`.
    fn new(source: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(index, _)| index + 1))
            .collect();
        Self { source, starts }
    }

    /// The byte offset of a span position (a 1-based line and a 0-based
    /// column counted in characters).
    fn offset(&self, at: LineColumn) -> usize {
        let start = self.starts[at.line - 1];
        self.source[start..]
            .char_indices()
            .nth(at.column)
            .map_or(self.source.len(), |(index, _)| start + index)
    }

    /// The byte range `tokens` spans.
    fn range(&self, tokens: TokenStream) -> Range<usize> {
        let mut tokens = tokens.into_iter();
        let first = tokens.next().expect("a syntax node has tokens").span();
        let last = tokens.last().map_or(first, |token| token.span());
        self.offset(first.start())..self.offset(last.end())
    }

    /// An edit replacing `tokens` with `replacement`, widened over whole lines
    /// when the tokens fill them, so a deletion leaves no blank line behind.
    fn whole_lines(&self, tokens: TokenStream, replacement: String) -> Edit {
        let Range { mut start, mut end } = self.range(tokens);
        let line_start = self.source[..start]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line_end = self.source[end..]
            .find('\n')
            .map_or(self.source.len(), |index| end + index + 1);
        if self.source[line_start..start].trim().is_empty()
            && self.source[end..line_end].trim().is_empty()
        {
            start = line_start;
            end = line_end;
            if !replacement.is_empty() {
                return Edit {
                    range: start..end,
                    replacement: format!("{replacement}\n"),
                };
            }
        }
        Edit {
            range: start..end,
            replacement,
        }
    }

    /// An edit deleting the blank lines between `above` and `below`, if
    /// nothing but blank lines separates them.
    ///
    /// The edit starts after the line `above` ends on and stops at the line
    /// `below` starts on, so it touches but never overlaps the whole-line edit
    /// of either item.
    fn blank_lines_between(&self, above: &impl ToTokens, below: &impl ToTokens) -> Option<Edit> {
        let end = self.range(above.to_token_stream()).end;
        let start = self.range(below.to_token_stream()).start;
        let gap_start = end + self.source[end..start].find('\n')? + 1;
        let gap_end = self.source[..start].rfind('\n')? + 1;
        (gap_start < gap_end && self.source[gap_start..gap_end].trim().is_empty()).then(|| Edit {
            range: gap_start..gap_end,
            replacement: String::new(),
        })
    }

    /// An edit deleting an attribute: its whole lines when it stands on lines
    /// of its own, and otherwise the attribute and the spaces after it.
    fn attribute(&self, attribute: &Attribute) -> Edit {
        let Range { start, end } = self
            .whole_lines(attribute.to_token_stream(), String::new())
            .range;
        let trailing = if self.source[..end].ends_with('\n') {
            0
        } else {
            self.source[end..].len() - self.source[end..].trim_start_matches(' ').len()
        };
        Edit {
            range: start..end + trailing,
            replacement: String::new(),
        }
    }

    /// An edit deleting a visibility marker and the spaces after it.
    fn visibility(&self, vis: &Visibility) -> Edit {
        let Range { start, end } = match vis {
            Visibility::Public(token) => self.span(token.span),
            Visibility::Restricted(restricted) => {
                self.span(restricted.pub_token.span).start
                    ..self.span(restricted.paren_token.span.close()).end
            }
            Visibility::Inherited => unreachable!("only written visibilities are deleted"),
        };
        let trailing = self.source[end..].len() - self.source[end..].trim_start_matches(' ').len();
        Edit {
            range: start..end + trailing,
            replacement: String::new(),
        }
    }

    /// The byte range of one token's span.
    fn span(&self, span: Span) -> Range<usize> {
        self.offset(span.start())..self.offset(span.end())
    }
}

/// One replacement of a byte range of a chunk's source.
struct Edit {
    /// The bytes replaced.
    range: Range<usize>,
    /// The text that takes their place.
    replacement: String,
}

/// Apply `edits`, which must not overlap, to `source`.
fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| edit.range.start);
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for edit in edits {
        out.push_str(&source[at..edit.range.start]);
        out.push_str(&edit.replacement);
        at = edit.range.end;
    }
    out.push_str(&source[at..]);
    out
}

/// Collapse runs of blank lines to one, and trim blank lines at either end.
fn tidy(text: &str) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() && out.last().is_none_or(|last: &&str| last.trim().is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|last| last.trim().is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// Render the fragment for chunk `index`: a `no_run` doctest showing that
/// chunk, with every earlier chunk as hidden lines.
///
/// Every fragment but the last allows dead code, since a prefix defines items
/// only later chunks use, and ends with a hidden `main` so that its items stay
/// top-level.
pub fn fragment(flattened: &[String], index: usize) -> Result<String, String> {
    let last = index + 1 == flattened.len();
    let mut out = String::from("\n```no_run\n");
    if !last {
        out.push_str("# #![allow(dead_code)]\n");
    }
    for earlier in &flattened[..index] {
        for line in earlier.lines().chain(std::iter::once("")) {
            out.push_str(&hidden(line));
        }
    }
    for line in flattened[index].lines() {
        let trimmed = line.trim_start();
        if trimmed == "#" || trimmed.starts_with("# ") || trimmed.starts_with("##") {
            return Err(format!(
                "a line of the chunk at position {index} would be hidden or unescaped \
                 by rustdoc: {line}"
            ));
        }
        out.push_str(line);
        out.push('\n');
    }
    if !last {
        out.push_str("# fn main() {}\n");
    }
    out.push_str("```\n");
    Ok(out)
}

/// One hidden doctest line.
fn hidden(line: &str) -> String {
    if line.is_empty() {
        "#\n".to_owned()
    } else {
        format!("# {line}\n")
    }
}

/// The chunk names the tutorial's `#![doc = tutorial!("…")]` attributes
/// reference, in order.
pub fn references(tutorial: &str) -> Result<Vec<String>, String> {
    let file = parse(tutorial, "src/tutorial.rs")?;
    let mut names = Vec::new();
    for attribute in &file.attrs {
        if let Meta::NameValue(pair) = &attribute.meta
            && pair.path.is_ident("doc")
            && let Expr::Macro(call) = &pair.value
            && call.mac.path.is_ident("tutorial")
        {
            let name: Lit = call
                .mac
                .parse_body()
                .map_err(|error| format!("src/tutorial.rs: `tutorial!` takes a string: {error}"))?;
            match name {
                Lit::Str(name) => names.push(name.value()),
                _ => return Err("src/tutorial.rs: `tutorial!` takes a string".to_owned()),
            }
        }
    }
    Ok(names)
}

/// Check that the tutorial references every chunk exactly once, in teaching
/// order, explaining the first difference otherwise.
pub fn check_order(chunks: &[String], references: &[String]) -> Result<(), String> {
    if chunks == references {
        return Ok(());
    }
    let position = chunks
        .iter()
        .zip(references)
        .position(|(chunk, reference)| chunk != reference)
        .unwrap_or(chunks.len().min(references.len()));
    let found = references
        .get(position)
        .map_or("nothing more".to_owned(), |name| {
            format!("tutorial!(\"{name}\")")
        });
    let expected = chunks
        .get(position)
        .map_or("nothing more".to_owned(), |name| {
            format!("tutorial!(\"{name}\")")
        });
    Err(format!(
        "src/tutorial.rs must reference each kv chunk once, in teaching order. At \
         position {position} it has {found} where the example needs {expected}.\n\
         The example's chunks, in order: {}",
        chunks.join(", ")
    ))
}
