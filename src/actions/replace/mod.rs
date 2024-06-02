use super::Action;
use crate::scoping::{regex::CaptureGroup, scope::ScopeContext};
use log::{debug, error, info, log_enabled, Level};
use std::{collections::HashSet, error::Error, fmt};
use unescape::unescape;
use variables::{inject_variables, VariableExpressionError, VariablePositions};

mod variables;

/// Replaces input with a fixed string.
///
/// ## Example: replacing invalid characters in identifiers
///
/// ```rust
/// use srgn::RegexPattern;
/// use srgn::scoping::{view::ScopedViewBuilder, regex::Regex};
///
/// let scoper = Regex::new(RegexPattern::new(r"[^a-zA-Z0-9]+").unwrap());
/// let mut builder = ScopedViewBuilder::new("hyphenated-variable-name");
/// builder.explode(&scoper);
/// let mut view = builder.build();
/// view.replace("_".to_string());
///
/// assert_eq!(
///    view.to_string(),
///   "hyphenated_variable_name"
/// );
/// ```
///
/// ## Example: replace emojis
///
/// ```rust
/// use srgn::RegexPattern;
/// use srgn::scoping::{view::ScopedViewBuilder, regex::Regex};
///
/// // A Unicode character class category. See also
/// // https://github.com/rust-lang/regex/blob/061ee815ef2c44101dba7b0b124600fcb03c1912/UNICODE.md#rl12-properties
/// let scoper = Regex::new(RegexPattern::new(r"\p{Emoji}").unwrap());
/// let mut builder = ScopedViewBuilder::new("Party! 😁 💃 🎉 🥳 So much fun! ╰(°▽°)╯");
/// builder.explode(&scoper);
/// let mut view = builder.build();
/// view.replace(":(".to_string());
///
/// assert_eq!(
///    view.to_string(),
///    // Party is over, sorry ¯\_(ツ)_/¯
///   "Party! :( :( :( :( So much fun! ╰(°▽°)╯"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Replacement {
    repl: String,
    variables: VariablePositions,
}

impl TryFrom<String> for Replacement {
    type Error = ReplacementCreationError;

    /// Creates a new replacement from an owned string.
    ///
    /// Escape sequences are accepted and processed, with invalid escape sequences
    /// returning an [`Err`].
    ///
    /// ## Example: Basic usage
    ///
    /// ```
    /// use srgn::actions::Replacement;
    ///
    /// // Successful creation of a regular string
    /// let replacement = Replacement::try_from("Some Replacement".to_owned());
    /// assert!(replacement.is_ok());
    ///
    /// // Successful creation, with escape characters
    /// let replacement = Replacement::try_from(r"Some \t Escape".to_owned());
    /// assert!(replacement.is_ok());
    /// ```
    ///
    /// ## Example: Invalid escape sequence
    ///
    /// Creation fails due to invalid escape sequences.
    ///
    /// ```
    /// use srgn::actions::{Replacement, ReplacementCreationError};
    ///
    /// let replacement = Replacement::try_from(r"Invalid \z Escape".to_owned());
    /// assert_eq!(
    ///    replacement,
    ///    Err(ReplacementCreationError::InvalidEscapeSequences(
    ///      "Invalid \\z Escape".to_owned()
    ///    ))
    /// );
    /// ```
    fn try_from(replacement: String) -> Result<Self, Self::Error> {
        let unescaped = unescape(&replacement).ok_or(
            ReplacementCreationError::InvalidEscapeSequences(replacement),
        )?;

        let variables = inject_variables(&unescaped)?;

        Ok(Self {
            repl: unescaped,
            variables,
        })
    }
}

/// An error that can occur when creating a replacement.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplacementCreationError {
    /// The replacement contains invalid escape sequences.
    InvalidEscapeSequences(String),
    /// The replacement contains an error in its variable expressions.
    VariableError(VariableExpressionError),
}

impl fmt::Display for ReplacementCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEscapeSequences(replacement) => {
                write!(f, "Contains invalid escape sequences: '{replacement}'")
            }
            Self::VariableError(err) => {
                write!(f, "Error in variable expressions: '{err}'")
            }
        }
    }
}

impl Error for ReplacementCreationError {}

impl From<VariableExpressionError> for ReplacementCreationError {
    fn from(value: VariableExpressionError) -> Self {
        Self::VariableError(value)
    }
}

impl Action for Replacement {
    fn act(&self, input: &str) -> String {
        info!("Substituting '{}' with '{}'", input, self.repl);
        self.repl.clone()
    }

    fn act_with_context(&self, input: &str, context: ScopeContext) -> String {
        match context {
            ScopeContext::CaptureGroups(cgs) => {
                if log_enabled!(Level::Debug) {
                    let have: HashSet<&CaptureGroup> = cgs.keys().collect();
                    let need: HashSet<&CaptureGroup> = self.variables.keys().collect();

                    debug!("Excess capture groups: {:?}", have.difference(&need));
                    debug!("Common capture groups: {:?}", have.intersection(&need));
                    error!(
                        "Needed but missing capture groups: {:?}",
                        need.difference(&have)
                    );
                }

                for (cg, positions) in &self.variables {}
            }
        };

        todo!()
    }
}
