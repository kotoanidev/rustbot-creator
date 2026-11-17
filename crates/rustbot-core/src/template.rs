//! Text templates: `Pong, {user.name}!`, with `{{` and `}}` for literal braces (D-047,
//! D-051).

/// A piece of a parsed template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Text copied as is.
    Text(String),
    /// A variable path, e.g. `["user", "name"]` for `{user.name}`.
    Var(Vec<String>),
}

/// A template that does not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    /// A `{` without its `}`.
    #[error("`{{` at byte {0} is never closed")]
    Unclosed(usize),
    /// A `}` without a `{`. Write `}}` for a literal brace.
    #[error("`}}` at byte {0} has no opening `{{` (write `}}}}` for a literal brace)")]
    Stray(usize),
    /// `{}` or a path with an empty part, like `{user.}`.
    #[error("empty variable at byte {0}")]
    Empty(usize),
}

/// Splits a template into text and variables.
///
/// # Errors
/// A brace that is not closed, a stray `}`, or an empty variable.
pub fn parse(template: &str) -> Result<Vec<Segment>, TemplateError> {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut chars = template.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '{' if chars.peek().is_some_and(|&(_, next)| next == '{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek().is_some_and(|&(_, next)| next == '}') => {
                chars.next();
                text.push('}');
            }
            '}' => return Err(TemplateError::Stray(at)),
            '{' => {
                let mut path = String::new();
                loop {
                    match chars.next() {
                        Some((_, '}')) => break,
                        Some((_, c)) => path.push(c),
                        None => return Err(TemplateError::Unclosed(at)),
                    }
                }
                let parts: Vec<String> = path.split('.').map(|p| p.trim().to_owned()).collect();
                if parts.iter().any(String::is_empty) {
                    return Err(TemplateError::Empty(at));
                }
                if !text.is_empty() {
                    segments.push(Segment::Text(std::mem::take(&mut text)));
                }
                segments.push(Segment::Var(parts));
            }
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::{Segment, TemplateError, parse};

    fn var(path: &str) -> Segment {
        Segment::Var(path.split('.').map(str::to_owned).collect())
    }

    fn text(s: &str) -> Segment {
        Segment::Text(s.to_owned())
    }

    #[test]
    fn parses_text_and_variables() {
        assert_eq!(
            parse("Pong, {user.name}!").unwrap(),
            vec![text("Pong, "), var("user.name"), text("!")]
        );
        assert_eq!(parse("{ server.name }").unwrap(), vec![var("server.name")]);
        assert_eq!(parse("").unwrap(), vec![]);
    }

    #[test]
    fn doubled_braces_are_literal() {
        assert_eq!(
            parse("{{not a var}} {user.id}").unwrap(),
            vec![text("{not a var} "), var("user.id")]
        );
    }

    #[test]
    fn errors() {
        assert_eq!(parse("Hi {user.name"), Err(TemplateError::Unclosed(3)));
        assert_eq!(parse("Hi }"), Err(TemplateError::Stray(3)));
        assert_eq!(parse("{}"), Err(TemplateError::Empty(0)));
        assert_eq!(parse("{user.}"), Err(TemplateError::Empty(0)));
    }
}
