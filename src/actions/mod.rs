mod deletion;
#[cfg(feature = "german")]
mod german;
mod lower;
mod normalization;
mod replace;
#[cfg(feature = "symbols")]
mod symbols;
mod titlecase;
mod upper;

use crate::scoping::scope::ScopeContext;
pub use deletion::Deletion;
#[cfg(feature = "german")]
pub use german::German;
pub use lower::Lower;
pub use normalization::Normalization;
pub use replace::{Replacement, ReplacementCreationError};
#[cfg(feature = "symbols")]
pub use symbols::{inversion::SymbolsInversion, Symbols};
pub use titlecase::Titlecase;
pub use upper::Upper;

/// An action in the processing pipeline.
///
/// Actions are the core of the text processing pipeline and can be applied in any
/// order, [any number of times each](https://en.wikipedia.org/wiki/Idempotence) (more
/// than once being wasted work, though).
pub trait Action: Send + Sync {
    /// Apply this action to the given input.
    ///
    /// This is infallible: it cannot fail in the sense of [`Result`]. It can only
    /// return incorrect results, which would be bugs (please report).
    fn act(&self, input: &str) -> String;

    /// Acts taking into account additional context.
    ///
    /// By default, the context is ignored and [`Action::act`] is called. Implementors
    /// which need and know how to handle additional context can overwrite this method.
    fn act_with_context(&self, input: &str, context: ScopeContext) -> String {
        let _ = context; // Mark variable as used
        self.act(input)
    }
}

/// Any function that can be used as an [`Action`].
impl<T> Action for T
where
    T: Fn(&str) -> String + Send + Sync,
{
    fn act(&self, input: &str) -> String {
        self(input)
    }
}

/// Any [`Action`] that can be boxed.
// https://www.reddit.com/r/rust/comments/droxdg/why_arent_traits_impld_for_boxdyn_trait/
impl Action for Box<dyn Action> {
    fn act(&self, input: &str) -> String {
        self.as_ref().act(input)
    }
}
