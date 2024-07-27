use super::{negate, Frobulator, LanguageScoper, TSLanguage, TSQuery, NEG_QUERY, POS_QUERY};
use crate::{
    find::Find,
    scoping::{langs::IGNORE, scope::RangesWithContext, Scoper},
};
use clap::ValueEnum;
use const_format::formatcp;
use std::{fmt::Debug, str::FromStr};
use tree_sitter::QueryError;

pub struct CSharp {
    q: TSQuery,
    nq: Option<TSQuery>,
}

impl CSharp {
    pub fn new(q: impl Into<TSQuery> + Clone) -> Self {
        Self {
            q: q.clone().into(),
            nq: negate(q),
        }
    }
}

/// The C# language.
// pub type CSharp = Frobulator<CSharpQuery>;
/// A query for C#.
// pub type CSharpQuery = CodeQuery<CustomCSharpQuery, PreparedCSharpQuery>;

/// Prepared tree-sitter queries for C#.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PreparedCSharpQuery {
    /// Comments (including XML, inline, doc comments).
    Comments,
    /// Strings (incl. verbatim, interpolated; incl. quotes, except for interpolated).
    ///
    /// Raw strings are [not yet
    /// supported](https://github.com/tree-sitter/tree-sitter-c-sharp/pull/240).
    Strings,
    /// `using` directives (including periods).
    Usings,
}

impl From<PreparedCSharpQuery> for TSQuery {
    fn from(value: PreparedCSharpQuery) -> Self {
        TSQuery::new(
            &CSharp::lang(),
            match value {
                PreparedCSharpQuery::Comments => "(comment) @comment",
                PreparedCSharpQuery::Usings => {
                    r"(using_directive [(identifier) (qualified_name)] @import)"
                }
                PreparedCSharpQuery::Strings => {
                    formatcp!(
                        r"
                            [
                                (interpolated_string_expression (interpolation) @{0})
                                (string_literal)
                                (raw_string_literal)
                                (verbatim_string_literal)
                            ]
                            @string
                    ",
                        IGNORE
                    )
                }
            },
        )
        .expect("Prepared queries to be valid")
    }
}

/// A custom tree-sitter query for C#.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomCSharpQuery(String);

impl FromStr for CustomCSharpQuery {
    type Err = QueryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match TSQuery::new(&CSharp::lang(), s) {
            Ok(_) => Ok(Self(s.to_string())),
            Err(e) => Err(e),
        }
    }
}

impl From<CustomCSharpQuery> for TSQuery {
    fn from(value: CustomCSharpQuery) -> Self {
        TSQuery::new(&CSharp::lang(), &value.0)
            .expect("Valid query, as object cannot be constructed otherwise")
    }
}

impl LanguageScoper for CSharp {
    fn lang() -> TSLanguage {
        tree_sitter_c_sharp::language()
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

impl Find for CSharp {
    fn extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }
}
