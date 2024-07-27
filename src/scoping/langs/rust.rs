use super::{negate, Find, Frobulator, LanguageScoper, TSLanguage, TSQuery, NEG_QUERY, POS_QUERY};
use crate::scoping::{scope::RangesWithContext, Scoper};
use clap::ValueEnum;
use std::{fmt::Debug, str::FromStr};
use tree_sitter::QueryError;

pub struct Rust {
    q: TSQuery,
    nq: Option<TSQuery>,
}

impl Rust {
    pub fn new(q: impl Into<TSQuery> + Clone) -> Self {
        Self {
            q: q.clone().into(),
            nq: negate(q),
        }
    }
}

/// The Rust language.
// pub type Rust = Frobulator<RustQuery>;
/// A query for Rust.
// pub type RustQuery = CodeQuery<CustomRustQuery, PreparedRustQuery>;

/// Prepared tree-sitter queries for Rust.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PreparedRustQuery {
    /// Comments (line and block styles; excluding doc comments; comment chars incl.).
    Comments,
    /// Doc comments (comment chars included).
    DocComments,
    /// Use statements (paths only; excl. `use`/`as`/`*`).
    Uses,
    /// Strings (regular, raw, byte; includes interpolation parts in format strings!).
    ///
    /// There is currently no support for an 'interpolation' type node in
    /// tree-sitter-rust (like there is in TypeScript and Python, for example).
    Strings,
}

impl From<PreparedRustQuery> for TSQuery {
    fn from(value: PreparedRustQuery) -> Self {
        TSQuery::new(
            &Rust::lang(),
            match value {
                PreparedRustQuery::Comments => {
                    r#"
                    [
                        (line_comment)+ @line
                        (block_comment)
                        (#not-match? @line "^///")
                    ]
                    @comment
                    "#
                }
                PreparedRustQuery::DocComments => {
                    r#"
                    (
                        (line_comment)+ @line
                        (#match? @line "^///")
                    )
                    "#
                }
                PreparedRustQuery::Uses => {
                    // Match any (wildcard `_`) `argument`, which includes:
                    //
                    // - `scoped_identifier`
                    // - `scoped_use_list`
                    // - `use_wildcard`
                    // - `use_as_clause`
                    //
                    // all at once.
                    r"
                    [
                        (use_declaration
                            argument: (_) @use
                        )
                    ]
                    "
                }
                PreparedRustQuery::Strings => "(string_content) @string",
            },
        )
        .expect("Prepared queries to be valid")
    }
}

/// A custom tree-sitter query for Rust.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomRustQuery(String);

impl FromStr for CustomRustQuery {
    type Err = QueryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match TSQuery::new(&Rust::lang(), s) {
            Ok(_) => Ok(Self(s.to_string())),
            Err(e) => Err(e),
        }
    }
}

impl From<CustomRustQuery> for TSQuery {
    fn from(value: CustomRustQuery) -> Self {
        TSQuery::new(&Rust::lang(), &value.0)
            .expect("Valid query, as object cannot be constructed otherwise")
    }
}

impl LanguageScoper for Rust {
    fn lang() -> TSLanguage {
        tree_sitter_rust::language()
    }

    fn query(&self) -> &TSQuery {
        &self.q
    }

    fn neg_query(&self) -> Option<&TSQuery>
    where
        Self: Sized,
    {
        self.nq.as_ref()
    }
}

impl Find for Rust {
    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }
}
