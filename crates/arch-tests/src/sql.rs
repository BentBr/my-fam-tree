//! All SQL lives in the persistence crate: no `sqlx` paths, query macros or
//! SQL string literals anywhere else, and persistence does not re-export sqlx.

/// Crates whose `src/` is not scanned: persistence owns SQL, migrator runs
/// `sqlx::migrate!`, arch-tests holds the scanner's SQL-positive fixtures.
pub const EXCLUDED_CRATES: &[&str] = &["persistence", "migrator", "arch-tests"];

/// One place where SQL (or sqlx) shows up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlHit {
    pub file: String,
    pub line: usize,
    pub what: String,
}

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// Macros that run SQL (bare or `sqlx::`-qualified).
const QUERY_MACROS: &[&str] = &["query", "query_as", "query_scalar", "query_file", "query_file_as"];

/// Whether `s` reads as an SQL statement. Needs a statement shape, not just a
/// leading keyword, so UI text like "Select a family" is not SQL.
#[must_use]
pub fn looks_like_sql(s: &str) -> bool {
    let norm = s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let words: Vec<&str> = norm.split(' ').collect();
    let has = |w: &str| words.contains(&w);
    match words.first().copied().unwrap_or_default() {
        "select" => has("from"),
        "insert" => words.get(1) == Some(&"into"),
        "update" => has("set") && norm.contains('='),
        "delete" => {
            words.get(1) == Some(&"from")
                && words.get(3).is_none_or(|w| matches!(*w, "where" | "returning" | "using"))
        }
        "with" => norm.contains(" as ("),
        "truncate" => words.len() >= 2,
        _ => false,
    }
}

struct Scanner<'a> {
    file: &'a str,
    hits: Vec<SqlHit>,
}

impl Scanner<'_> {
    fn hit(&mut self, span: proc_macro2::Span, what: impl Into<String>) {
        self.hits.push(SqlHit {
            file: self.file.to_owned(),
            line: span.start().line,
            what: what.into(),
        });
    }

    fn lit(&mut self, span: proc_macro2::Span, value: &str) {
        if looks_like_sql(value) {
            let preview: String = value.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
            self.hit(span, format!("SQL string literal `{preview}…`"));
        }
    }

    fn tokens(&mut self, tokens: TokenStream) {
        for tt in tokens {
            match tt {
                TokenTree::Group(g) => self.tokens(g.stream()),
                TokenTree::Literal(l) => {
                    if let Ok(s) = syn::parse_str::<syn::LitStr>(&l.to_string()) {
                        self.lit(l.span(), &s.value());
                    }
                }
                TokenTree::Ident(i) if i == "sqlx" => {
                    self.hit(i.span(), "`sqlx` inside a macro or attribute");
                }
                TokenTree::Ident(_) | TokenTree::Punct(_) => {}
            }
        }
    }
}

fn is_sqlx(path: &syn::Path) -> bool {
    path.segments.first().is_some_and(|s| s.ident == "sqlx")
}

/// Span of the first `sqlx` root in a use tree: `use sqlx::X`, `use sqlx`,
/// `use sqlx as db` and any of those inside a `{…}` group.
fn use_tree_sqlx(tree: &syn::UseTree) -> Option<proc_macro2::Span> {
    match tree {
        syn::UseTree::Path(p) => (p.ident == "sqlx").then(|| p.ident.span()),
        syn::UseTree::Name(n) => (n.ident == "sqlx").then(|| n.ident.span()),
        syn::UseTree::Rename(r) => (r.ident == "sqlx").then(|| r.ident.span()),
        syn::UseTree::Group(g) => g.items.iter().find_map(use_tree_sqlx),
        syn::UseTree::Glob(_) => None,
    }
}

/// Public sqlx re-exports among `items`, descending into inline modules.
fn collect_reexports(items: &[syn::Item], out: &mut Vec<proc_macro2::Span>) {
    let public = |v: &syn::Visibility| matches!(v, syn::Visibility::Public(_));
    for item in items {
        match item {
            syn::Item::Use(u) if public(&u.vis) && use_tree_sqlx(&u.tree).is_some() => {
                out.push(u.span());
            }
            syn::Item::Type(t) if public(&t.vis) => {
                if matches!(&*t.ty, syn::Type::Path(tp) if is_sqlx(&tp.path)) {
                    out.push(t.span());
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    collect_reexports(inner, out);
                }
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if is_sqlx(path) {
            self.hit(path.span(), "`sqlx::` path");
        }
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if let Some(span) = use_tree_sqlx(&item.tree) {
            self.hit(span, "`use sqlx`");
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = mac.path.segments.last().map_or_else(String::new, |s| s.ident.to_string());
        if QUERY_MACROS.contains(&name.as_str()) && !is_sqlx(&mac.path) {
            self.hit(mac.path.span(), format!("query macro `{name}!`"));
        }
        self.tokens(mac.tokens.clone());
        visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        // Doc comments are prose; other attributes (derive(sqlx::FromRow),
        // #[sqlx(...)]) are checked for the sqlx ident.
        if attr.path().is_ident("doc") {
            return;
        }
        if attr.path().is_ident("sqlx") {
            self.hit(attr.span(), "`#[sqlx]` attribute");
            return;
        }
        if let syn::Meta::List(list) = &attr.meta {
            self.tokens(list.tokens.clone());
        }
    }

    fn visit_lit_str(&mut self, lit: &'ast syn::LitStr) {
        self.lit(lit.span(), &lit.value());
    }
}

/// SQL / sqlx usages in one source file.
///
/// # Errors
/// When `src` is not valid Rust.
pub fn scan_source(path: &str, src: &str) -> Result<Vec<SqlHit>, syn::Error> {
    let file = syn::parse_file(src)?;
    let mut scanner = Scanner { file: path, hits: Vec::new() };
    scanner.visit_file(&file);
    Ok(scanner.hits)
}

/// Public re-exports of sqlx (`pub use sqlx…`, `pub use {sqlx::…}`,
/// `pub type X = sqlx::…`), including those inside inline modules.
///
/// # Errors
/// When `src` is not valid Rust.
pub fn check_reexports(path: &str, src: &str) -> Result<Vec<SqlHit>, syn::Error> {
    let file = syn::parse_file(src)?;
    let mut spans = Vec::new();
    collect_reexports(&file.items, &mut spans);
    let hits = spans
        .into_iter()
        .map(|span| SqlHit {
            file: path.to_owned(),
            line: span.start().line,
            what: "public sqlx re-export".into(),
        })
        .collect();
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(src: &str) -> Vec<SqlHit> {
        scan_source("t.rs", src)
            .unwrap_or_else(|e| vec![SqlHit { file: "parse".into(), line: 0, what: e.to_string() }])
    }

    #[test]
    fn flags_sqlx_path() {
        assert_eq!(hits("fn f() { sqlx::query(\"x\"); }").len(), 1, "sqlx:: call");
    }

    #[test]
    fn flags_use_sqlx() {
        assert_eq!(hits("use sqlx::PgPool;").len(), 1, "use sqlx");
    }

    #[test]
    fn flags_sqlx_type() {
        assert_eq!(hits("fn f(p: &sqlx::PgPool) {}").len(), 1, "sqlx type in signature");
    }

    #[test]
    fn flags_query_macro() {
        assert!(
            !hits("fn f() { query_as!(T, \"SELECT id FROM persons\"); }").is_empty(),
            "query_as!"
        );
    }

    #[test]
    fn flags_sql_literal() {
        assert_eq!(
            hits("const Q: &str = \"DELETE FROM partnerships WHERE id = $1\";").len(),
            1,
            "SQL literal"
        );
    }

    #[test]
    fn flags_sql_in_format_macro() {
        assert_eq!(
            hits("fn f() { let _ = format!(\"SELECT {} FROM persons\", c); }").len(),
            1,
            "SQL in format!"
        );
    }

    #[test]
    fn flags_raw_string_sql() {
        assert_eq!(
            hits("const Q: &str = r#\"INSERT INTO users (id) VALUES ($1)\"#;").len(),
            1,
            "raw string"
        );
    }

    #[test]
    fn ignores_ui_text() {
        let src = r#"const A: &str = "Select a family"; const B: &str = "Update your profile"; const C: &str = "with love"; const D: &str = "Delete from the tree";"#;
        assert!(hits(src).is_empty(), "UI text is not SQL: {:?}", hits(src));
    }

    #[test]
    fn ignores_comments_and_docs() {
        assert!(
            hits("/// SELECT id FROM persons\nfn f() {}\n// DELETE FROM x\n").is_empty(),
            "docs and comments"
        );
    }

    #[test]
    fn reports_line_numbers() {
        let h = hits("\n\nuse sqlx::PgPool;");
        assert_eq!(h.iter().map(|x| x.line).collect::<Vec<_>>(), vec![3], "line of the hit");
    }

    #[test]
    fn flags_reexport() {
        let reexport = check_reexports("p.rs", "pub use sqlx::PgPool;").unwrap_or_default();
        assert_eq!(reexport.len(), 1, "pub use sqlx");
        let alias = check_reexports("p.rs", "pub type Pool = sqlx::PgPool;").unwrap_or_default();
        assert_eq!(alias.len(), 1, "pub type alias");
        let private = check_reexports("p.rs", "use sqlx::PgPool;").unwrap_or_default();
        assert!(private.is_empty(), "private use is fine");
    }

    #[test]
    fn flags_grouped_and_bare_use_sqlx() {
        assert_eq!(hits("use {sqlx::PgPool};").len(), 1, "grouped");
        assert_eq!(hits("use {std::fmt, sqlx::PgPool};").len(), 1, "grouped with others");
        assert_eq!(hits("use sqlx;").len(), 1, "bare crate");
        assert_eq!(hits("use sqlx as db;").len(), 1, "renamed crate");
        assert!(hits("use std::{fmt, io};").is_empty(), "unrelated group");
    }

    #[test]
    fn flags_grouped_and_nested_reexports() {
        let count = |src: &str| check_reexports("p.rs", src).map_or(usize::MAX, |h| h.len());
        assert_eq!(count("pub use {sqlx::PgPool};"), 1, "grouped pub use");
        assert_eq!(count("pub mod db { pub use sqlx::PgPool; }"), 1, "pub use in inline module");
        assert_eq!(
            count("mod db { pub type Pool = sqlx::PgPool; }"),
            1,
            "pub type in inline module"
        );
        assert_eq!(count("pub mod db { use sqlx::PgPool; }"), 0, "private use in module");
    }

    #[test]
    fn parse_error_is_reported() {
        assert!(scan_source("t.rs", "fn (").is_err(), "invalid Rust");
    }

    #[test]
    fn exclusion_list_is_exact() {
        assert_eq!(
            EXCLUDED_CRATES,
            &["persistence", "migrator", "arch-tests"],
            "only these three skip the scan"
        );
    }

    #[test]
    fn looks_like_sql_needs_two_keywords() {
        assert!(looks_like_sql("  select id from x"), "select … from");
        assert!(looks_like_sql("UPDATE persons SET name = $1"), "update … set");
        assert!(looks_like_sql("WITH t AS (SELECT 1) SELECT * FROM t"), "with … as (");
        assert!(looks_like_sql("TRUNCATE users"), "truncate");
        assert!(!looks_like_sql("Select a family"), "UI text");
        assert!(!looks_like_sql("update available"), "UI text");
    }
}
