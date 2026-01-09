pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Unexpected token '{found}' at position {position}. Expected {expected}")]
    UnexpectedToken {
        expected: String,
        found: String,
        position: usize,
    },

    #[error("Invalid syntax: {message} at position {position}")]
    InvalidSyntax { message: String, position: usize },

    #[error("Invalid number format: {value} at position {position}")]
    InvalidNumber { value: String, position: usize },

    #[error("Invalid identifier '{value}' at position {position}")]
    InvalidIdentifier { value: String, position: usize },

    #[error("Unterminated string literal at position {position}")]
    UnterminatedString { position: usize },

    #[error("Invalid entity type '{value}' at position {position}")]
    InvalidEntityType { value: String, position: usize },

    #[error("Invalid priority '{value}' at position {position}")]
    InvalidPriority { value: String, position: usize },

    #[error("Missing clause '{clause}' at position {position}")]
    MissingClause { clause: String, position: usize },

    #[error("Invalid issue ID '{value}' at position {position}")]
    InvalidIssueId { value: String, position: usize },

    #[error("General Error: {0}")]
    General(String),
}
