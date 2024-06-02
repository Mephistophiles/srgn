use crate::scoping::regex::CaptureGroup;
use std::{collections::HashMap, error::Error, fmt, ops::Range};

/// Mapping variables (which are named or numbered) to their (potentially multiple)
/// position(s) in an input variable expression.
pub(super) type VariablePositions = HashMap<CaptureGroup, Vec<Range<usize>>>;

/// In an input like `Hello $var World`, find the positions of variables.
///
/// Variables are treated as they occur in regular expressions: they can be [named or
/// numbered](https://docs.rs/regex/latest/regex/struct.Captures.html).
#[allow(clippy::too_many_lines)] // :(
pub(super) fn inject_variables(
    input: &str,
    variables: HashMap<CaptureGroup, &str>,
) -> Result<String, VariableExpressionError> {
    // let mut variables = VariablePositions::default();
    let mut state = State::Noop;
    let mut out = String::with_capacity(input.len());

    for (i, c) in input.chars().enumerate() {
        state = match (state, c) {
            // Initial state
            (State::Noop | State::WindUpBraces(_), '$') => State::MaybeVariableStart,
            (State::Noop, _) => State::Noop,

            // Init
            (State::MaybeVariableStart, '{') => State::WindUpBraces(1),
            (State::WindUpBraces(n), '{') => State::WindUpBraces(n + 1),
            (State::MaybeVariableStart, 'a'..='z' | 'A'..='Z' | '_') => {
                State::BuildingNamedVariable {
                    name: String::from(c),
                    start: i - '$'.len_utf8(),
                    n_braces: 0,
                }
            }
            (State::WindUpBraces(n), 'a'..='z' | 'A'..='Z' | '_') => State::BuildingNamedVariable {
                name: String::from(c),
                start: i - ('$'.len_utf8() + '{'.len_utf8() * n),
                n_braces: n,
            },
            (State::MaybeVariableStart, '0'..='9') => State::BuildingNumberedVariable {
                magnitude: c.to_digit(10).expect("hard-coded digit is valid number") as usize,
                start: i - '$'.len_utf8(),
                n_braces: 0,
            },
            (State::WindUpBraces(n), '0'..='9') => State::BuildingNumberedVariable {
                magnitude: c.to_digit(10).expect("hard-coded digit is valid number") as usize,
                start: i - ('$'.len_utf8() + '{'.len_utf8() * n),
                n_braces: n,
            },

            // Nothing useful matched, go back. Matches `$` as well, allowing escaping.
            // This is order-dependent, see also
            // https://github.com/rust-lang/rust-clippy/issues/860
            #[allow(clippy::match_same_arms)]
            (State::MaybeVariableStart | State::WindUpBraces(_), _) => State::Noop,

            // Building up
            (
                State::BuildingNamedVariable {
                    mut name,
                    start,
                    n_braces,
                },
                'a'..='z' | 'A'..='Z' | '_' | '0'..='9',
            ) => State::BuildingNamedVariable {
                name: {
                    name.push(c);
                    name
                },
                start,
                n_braces,
            },
            (
                State::BuildingNumberedVariable {
                    magnitude,
                    start,
                    n_braces,
                },
                '0'..='9',
            ) => State::BuildingNumberedVariable {
                magnitude: {
                    magnitude * 10
                        + c.to_digit(10).expect("hard-coded digit is valid number") as usize
                },
                start,
                n_braces,
            },

            // Building stops
            (
                State::BuildingNamedVariable {
                    name,
                    start,
                    n_braces: 0,
                }
                | State::WindDownNamedVariable {
                    name,
                    start,
                    n_braces: 0,
                },
                _,
            ) => {
                let range = start..i;
                variables
                    .entry(CaptureGroup::Named(name))
                    .and_modify(|ranges| ranges.push(range.clone()))
                    .or_insert(vec![range]);

                match c {
                    '$' => State::MaybeVariableStart,
                    _ => State::Noop,
                }
            }

            (
                State::BuildingNamedVariable {
                    name,
                    start,
                    n_braces,
                }
                | State::WindDownNamedVariable {
                    name,
                    start,
                    n_braces,
                },
                _,
            ) => match c {
                '}' => State::WindDownNamedVariable {
                    name,
                    start,
                    n_braces: n_braces - 1,
                },
                _ => return Err(VariableExpressionError::MismatchedBraces(input.to_owned())),
            },

            (
                State::BuildingNumberedVariable {
                    magnitude,
                    start,
                    n_braces: 0,
                }
                | State::WindDownNumberedVariable {
                    magnitude,
                    start,
                    n_braces: 0,
                },
                _,
            ) => {
                let range = start..i;
                variables
                    .entry(CaptureGroup::Numbered(magnitude))
                    .and_modify(|ranges| ranges.push(range.clone()))
                    .or_insert(vec![range]);

                match c {
                    '$' => State::MaybeVariableStart,
                    _ => State::Noop,
                }
            }

            (
                State::BuildingNumberedVariable {
                    magnitude,
                    start,
                    n_braces,
                }
                | State::WindDownNumberedVariable {
                    magnitude,
                    start,
                    n_braces,
                },
                _,
            ) => match c {
                '}' => State::WindDownNumberedVariable {
                    magnitude,
                    start,
                    n_braces: n_braces - 1,
                },
                _ => return Err(VariableExpressionError::MismatchedBraces(input.to_owned())),
            },
        }
    }

    // Flush out any pending state
    match state {
        State::BuildingNamedVariable {
            name,
            start,
            n_braces: 0,
        }
        | State::WindDownNamedVariable {
            name,
            start,
            n_braces: 0,
        } => {
            let range = start..input.len();
            variables
                .entry(CaptureGroup::Named(name))
                .and_modify(|ranges| ranges.push(range.clone()))
                .or_insert(vec![range]);
        }
        State::BuildingNumberedVariable {
            magnitude,
            start,
            n_braces: 0,
        }
        | State::WindDownNumberedVariable {
            magnitude,
            start,
            n_braces: 0,
        } => {
            let range = start..input.len();
            variables
                .entry(CaptureGroup::Numbered(magnitude))
                .and_modify(|ranges| ranges.push(range.clone()))
                .or_insert(vec![range]);
        }
        State::BuildingNamedVariable { .. }
        | State::WindDownNamedVariable { .. }
        | State::BuildingNumberedVariable { .. }
        | State::WindDownNumberedVariable { .. } => {
            return Err(VariableExpressionError::MismatchedBraces(input.to_owned()))
        }
        _ => (),
    };

    Ok(variables)
}

/// State during extraction of variables in an expression like `Hello $var World`.
enum State {
    /// The character initiating a variable declaration has been seen.
    MaybeVariableStart,
    /// A variable declaration is being built up, where the variable is surrounded by
    /// braces.
    WindUpBraces(usize),
    /// A named variable is detected and is being built up.
    BuildingNamedVariable {
        name: String,
        start: usize,
        n_braces: usize,
    },
    /// A numbered variable is detected and is being built up.
    BuildingNumberedVariable {
        magnitude: usize,
        start: usize,
        n_braces: usize,
    },
    /// A named variable was processed, and processing is now safely being wrapped up.
    WindDownNamedVariable {
        name: String,
        start: usize,
        n_braces: usize,
    },
    /// A numbered variable was processed, and processing is now safely being wrapped
    /// up.
    WindDownNumberedVariable {
        magnitude: usize,
        start: usize,
        n_braces: usize,
    },
    /// Start and neutral state.
    Noop,
}

/// An error in variable expressions.
#[derive(Debug, PartialEq, Eq)]
pub enum VariableExpressionError {
    /// A variable expression with mismatched number of braces.
    MismatchedBraces(String),
}

impl fmt::Display for VariableExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedBraces(expression) => {
                write!(f, "Contains an imbalanced set of braces: '{expression}'")
            }
        }
    }
}

impl Error for VariableExpressionError {}

#[cfg(never)]
#[allow(clippy::single_range_in_vec_init)]
mod test {
    use super::*;
    use rstest::*;

    #[rstest]
    // No variables
    #[case("", Ok(VariablePositions::default()))]
    #[case("Regular content", Ok(VariablePositions::default()))]
    #[case("$", Ok(VariablePositions::default()))]
    #[case("$$", Ok(VariablePositions::default()))]
    #[case("$$$", Ok(VariablePositions::default()))]
    //
    // Basic, named
    #[case("$var", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..4])])))]
    #[case(" $var", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![1..5])])))]
    #[case("$var ", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..4])])))]
    // Named are allowed to contain various extra characters
    #[case("$var1", Ok(VariablePositions::from([(CaptureGroup::Named("var1".into()), vec![0..5])])))]
    #[case("$var_1337_wow", Ok(VariablePositions::from([(CaptureGroup::Named("var_1337_wow".into()), vec![0..13])])))]
    #[case("$1var", Ok(VariablePositions::from([(CaptureGroup::Numbered(1), vec![0..2])])))]
    //
    // Uppercase
    #[case("$VAR", Ok(VariablePositions::from([(CaptureGroup::Named("VAR".into()), vec![0..4])])))]
    //
    // Braces
    #[case("${0}", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..4])])))]
    #[case("${var}", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..6])])))]
    #[case("${{var}}", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..8])])))]
    // Excess closing braces are fine
    #[case("${var}}", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..6])])))]
    // Insufficient closing braces aren't fine
    #[case("${{var}", Err(VariableExpressionError::MismatchedBraces("${{var}".into())))]
    #[case("${var", Err(VariableExpressionError::MismatchedBraces("${var".into())))]
    #[case("$var}", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..4])])))]
    //
    // Mixed content
    #[case("Mixed $var content", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![6..10])])))]
    #[case("Mixed$varcontent", Ok(VariablePositions::from([(CaptureGroup::Named("varcontent".into()), vec![5..16])])))]
    #[case("Mixed${var}content", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![5..11])])))]
    #[case("Mixed${{var}}content", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![5..13])])))]
    //
    // Basic, numbered
    #[case("$0", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..2])])))]
    #[case("$1", Ok(VariablePositions::from([(CaptureGroup::Numbered(1), vec![0..2])])))]
    #[case("$9999", Ok(VariablePositions::from([(CaptureGroup::Numbered(9999), vec![0..5])])))]
    #[case("$-1", Ok(VariablePositions::default()))]
    // Numbered cannot contain letters
    #[case("$0hello", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..2])])))]
    #[case("${0}hello", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..4])])))]
    //
    // Multiple variables, spaced out
    #[case("$var $var", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..4, 5..9])])))]
    #[case("$var $0 $var", Ok(VariablePositions::from([(CaptureGroup::Named("var".into()), vec![0..4, 8..12]), (CaptureGroup::Numbered(0), vec![5..7])])))]
    //
    // Multiple variables, back to back
    #[case("$var1$var2", Ok(VariablePositions::from([(CaptureGroup::Named("var1".into()), vec![0..5]), (CaptureGroup::Named("var2".into()), vec![5..10])])))]
    #[case("${var1}${var2}", Ok(VariablePositions::from([(CaptureGroup::Named("var1".into()), vec![0..7]), (CaptureGroup::Named("var2".into()), vec![7..14])])))]
    #[case("$0$1", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..2]), (CaptureGroup::Numbered(1), vec![2..4])])))]
    #[case("${0}${1}", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![0..4]), (CaptureGroup::Numbered(1), vec![4..8])])))]
    //
    // End-to-end
    #[case("A long $0 example $TEST!", Ok(VariablePositions::from([(CaptureGroup::Numbered(0), vec![7..9]), (CaptureGroup::Named("TEST".into()), vec![18..23])])))]
    //
    // Non-ASCII
    #[case("$🦀", Ok(VariablePositions::default()))]
    //
    // Escaping is possible
    #[case("$$notavar", Ok(VariablePositions::default()))]
    #[case("$$$avar", Ok(VariablePositions::from([(CaptureGroup::Named("avar".into()), vec![2..7])])))]
    #[case("$$${avar}$$", Ok(VariablePositions::from([(CaptureGroup::Named("avar".into()), vec![2..9])])))]
    fn test_extract_variable_position(
        #[case] expression: &str,
        #[case] expected: Result<VariablePositions, VariableExpressionError>,
    ) {
        let variables = inject_variables(expression);

        assert_eq!(variables, expected);
    }
}
