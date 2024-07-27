use super::Scoper;
#[cfg(doc)]
use crate::scoping::scope::Scope::{In, Out};
use crate::{find::Find, ranges::Ranges};
use log::{debug, trace};
use std::{str::FromStr, sync::OnceLock};
pub use tree_sitter::{
    Language as TSLanguage, Parser as TSParser, Query as TSQuery, QueryCursor as TSQueryCursor,
};

/// C#.
pub mod csharp;
/// Go.
pub mod go;
/// Hashicorp Configuration Language
pub mod hcl;
/// Python.
pub mod python;
/// Rust.
pub mod rust;
/// TypeScript.
pub mod typescript;

/// Represents a (programming) language.
#[derive(Debug)]
pub struct Frobulator
// where
//     &TSQuery: From<&PQ>,
{
    pos_query: TSQuery,
    neg_query: Option<TSQuery>,
}

// impl Frobulator
// where
//     PQ: Into<TSQuery> + Clone,
// trait Language {
//     fn new(q: impl Into<TSQuery>) -> Self
//     where
//         Self: Sized;

// {
/// Create a new language with the given associated query over it.
fn negate<Q: Into<TSQuery> + Clone>(query: Q) -> Option<TSQuery> {
    let mut query = query.into();
    let is_ignored = |name: &str| name.starts_with(IGNORE);
    let has_ignored_captures = query.capture_names().iter().any(|name| is_ignored(name));

    let neg_query = {
        if has_ignored_captures {
            // let mut query: TSQuery = query.clone().into();
            let acknowledged_captures = query
                .capture_names()
                .iter()
                .filter(|name| !is_ignored(name))
                .map(|s| String::from(*s))
                .collect::<Vec<_>>();

            for name in acknowledged_captures {
                trace!("Disabling capture for: {:?}", name);
                query.disable_capture(&name);
            }

            Some(query)
        } else {
            None
        }
    };

    return neg_query;

    // Self {
    //     pos_query: query.to_owned().into(),
    //     neg_query,
    // }
}

/// A query over a language, for scoping.
///
/// Parts hit by the query are [`In`] scope, parts not hit are [`Out`] of scope.
// #[derive(Debug, Clone)]
// pub enum CodeQuery<C, P>
// where
//     C: FromStr + Into<TSQuery>,
//     // &C: FromStr + Into<TSQuery>,
//     for<'a> &'a P: Into<TSQuery>,
// {
//     /// A custom, user-defined query.
//     Custom(C),
//     /// A prepared query.
//     ///
//     /// Availability depends on the language, respective languages features, and
//     /// implementation in this crate.
//     Prepared(P),
// }

// impl<C, P> From<&CodeQuery<C, P>> for TSQuery
// where
//     C: FromStr + Into<TSQuery>,
//     for<'a> &'a C: Into<TSQuery>,
//     for<'a> &'a P: Into<TSQuery>,
// {
//     fn from(value: &CodeQuery<C, P>) -> Self {
//         match value {
//             CodeQuery::Custom(query) => query.into(),
//             CodeQuery::Prepared(query) => query.into(),
//         }
//     }
// }

pub(crate) static POS_QUERY: OnceLock<TSQuery> = OnceLock::new();
pub(crate) static NEG_QUERY: OnceLock<Option<TSQuery>> = OnceLock::new();

// impl<'a, C, P> From<&CodeQuery<C, P>> for &TSQuery
// where
//     C: FromStr + Into<&'a TSQuery>,
//     P: Into<&'a TSQuery>,
// {
//     fn from(value: &CodeQuery<C, P>) -> Self {
//         match value {
//             CodeQuery::Custom(query) => query.into(),
//             CodeQuery::Prepared(query) => query.into(),
//         }
//     }
// }

/// In a query, use this name to mark a capture to be ignored.
///
/// Useful for queries where tree-sitter doesn't natively support a fitting node type,
/// and a result is instead obtained by ignoring unwanted parts of bigger captures.
pub(super) const IGNORE: &str = "_SRGN_IGNORE";

/// A scoper for a language.
///
/// Functions much the same, but provides specific language-related functionality.
pub trait LanguageScoper: Find + Send + Sync {
    /// The language's tree-sitter language.
    fn lang() -> TSLanguage
    where
        Self: Sized; // Exclude from trait object

    /// The language's tree-sitter query.
    fn query(&self) -> &TSQuery
    where
        Self: Sized; // Exclude from trait object

    /// The language's tree-sitter query.
    fn neg_query(&self) -> Option<&TSQuery>
    where
        Self: Sized; // Exclude from trait object

    /// The language's tree-sitter parser.
    #[must_use]
    fn parser() -> TSParser
    where
        Self: Sized, // Exclude from trait object
    {
        let mut parser = TSParser::new();
        parser
            .set_language(&Self::lang())
            .expect("Should be able to load language grammar and parser");

        parser
    }

    /// Scope the given input using the language's query.
    ///
    /// In principle, this is the same as [`Scoper::scope`].
    fn scope_via_query(&self, input: &str) -> Ranges<usize>
    where
        Self: Sized, // Exclude from trait object
    {
        // tree-sitter is about incremental parsing, which we don't use here
        let old_tree = None;

        trace!("Parsing into AST: {:?}", input);

        let tree = Self::parser()
            .parse(input, old_tree)
            .expect("No language set in parser, or other unrecoverable error");

        let root = tree.root_node();
        debug!(
            "S expression of parsed source code is: {:?}",
            root.to_sexp()
        );

        let run = |query: &TSQuery| {
            trace!("Running query: {:?}", query);

            let mut qc = TSQueryCursor::new();
            let matches = qc.matches(query, root, input.as_bytes());

            let mut ranges: Ranges<usize> = matches
                .flat_map(|query_match| query_match.captures)
                .map(|capture| capture.node.byte_range())
                .collect();

            // ⚠️ tree-sitter queries with multiple captures will return them in some
            // mixed order (not ordered, and not merged), but we later rely on cleanly
            // ordered, non-overlapping ranges (a bit unfortunate we have to know about
            // that remote part over here).
            ranges.merge();
            trace!("Querying yielded ranges: {:?}", ranges);

            ranges
        };

        match &self.neg_query() {
            Some(q) => run(self.query()) - run(q),
            None => run(self.query()),
        }
    }
}

impl<T> Scoper for T
where
    T: LanguageScoper,
{
    fn scope_raw<'viewee>(&self, input: &'viewee str) -> super::scope::RangesWithContext<'viewee> {
        self.scope_via_query(input).into()
    }
}
