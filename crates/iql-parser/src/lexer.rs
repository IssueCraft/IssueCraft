use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f\r]+")] // Skip whitespace
#[logos(error = String)]
pub enum Token {
    // ========== Keywords (case-insensitive) ==========
    #[regex("(?i)create")]
    Create,

    #[regex("(?i)select")]
    Select,

    #[regex("(?i)update")]
    Update,

    #[regex("(?i)delete")]
    Delete,

    #[regex("(?i)assign")]
    Assign,

    #[regex("(?i)close")]
    Close,

    #[regex("(?i)reopen")]
    Reopen,

    #[regex("(?i)comment")]
    Comment,

    #[regex("(?i)from")]
    From,

    #[regex("(?i)where")]
    Where,

    #[regex("(?i)and")]
    And,

    #[regex("(?i)or")]
    Or,

    #[regex("(?i)not")]
    Not,

    #[regex("(?i)in")]
    In,

    #[regex("(?i)of")]
    Of,

    #[regex("(?i)is")]
    Is,

    #[regex("(?i)null")]
    Null,

    #[regex("(?i)set")]
    Set,

    #[regex("(?i)to")]
    To,

    #[regex("(?i)on")]
    On,

    #[regex("(?i)with")]
    With,

    #[regex("(?i)order")]
    Order,

    #[regex("(?i)by")]
    By,

    #[regex("(?i)limit")]
    Limit,

    #[regex("(?i)offset")]
    Offset,

    #[regex("(?i)asc")]
    Asc,

    #[regex("(?i)desc")]
    Desc,

    #[regex("(?i)like")]
    Like,

    // ========== Entity Types ==========
    #[regex("(?i)user")]
    User,

    #[regex("(?i)project")]
    Project,

    #[regex("(?i)issue")]
    Issue,

    #[regex("(?i)issues")]
    Issues,

    #[regex("(?i)users")]
    Users,

    #[regex("(?i)projects")]
    Projects,

    #[regex("(?i)comments")]
    Comments,

    // ========== Field Names (used in WITH clauses) ==========
    #[regex("(?i)email")]
    Email,

    #[regex("(?i)name")]
    Name,

    #[regex("(?i)title")]
    Title,

    #[regex("(?i)kind")]
    Kind,

    #[regex("(?i)description")]
    Description,

    #[regex("(?i)priority")]
    Priority,

    #[regex("(?i)assignee")]
    Assignee,

    #[regex("(?i)owner")]
    Owner,

    // ========== Close Reasons ==========
    #[regex("(?i)duplicate")]
    Duplicate,

    #[regex("(?i)wontfix")]
    WontFix,

    #[regex("(?i)done")]
    Done,

    // ========== Issue Kinds ==========
    #[regex("(?i)epic")]
    Epic,

    #[regex("(?i)improvement")]
    Improvement,

    #[regex("(?i)bug")]
    Bug,

    #[regex("(?i)task")]
    Task,

    // ========== Priority Levels ==========
    #[regex("(?i)critical")]
    Critical,

    #[regex("(?i)high")]
    High,

    #[regex("(?i)medium")]
    Medium,

    #[regex("(?i)low")]
    Low,

    // ========== Literals ==========
    #[regex(r#"'([^'\\]|\\.)*'"#, parse_single_quoted_string)]
    #[regex(r#""([^"\\]|\\.)*""#, parse_double_quoted_string)]
    String(String),

    #[regex(r"-[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<u64>().ok())]
    UnsignedInteger(u64),

    #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex("(?i)true")]
    True,

    #[regex("(?i)false")]
    False,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_-]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // ========== Operators ==========
    #[token("*")]
    Star,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token("#")]
    Hash,

    #[token("=")]
    Equal,

    #[token("!=")]
    NotEqual,

    #[token(">")]
    GreaterThan,

    #[token("<")]
    LessThan,

    #[token(">=")]
    GreaterOrEqual,

    #[token("<=")]
    LessOrEqual,

    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    // ========== Special ==========
    Eof,
}

fn parse_single_quoted_string(lex: &mut logos::Lexer<Token>) -> String {
    let slice = lex.slice();
    let content = &slice[1..slice.len() - 1];
    unescape_string(content)
}

fn parse_double_quoted_string(lex: &mut logos::Lexer<Token>) -> String {
    let slice = lex.slice();
    let content = &slice[1..slice.len() - 1];
    unescape_string(content)
}

fn unescape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') | None => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\'') => result.push('\''),
                Some('"') => result.push('"'),
                Some('0') => result.push('\0'),
                Some(c) => {
                    // Unknown escape sequence - keep as is
                    result.push('\\');
                    result.push(c);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lexer = Token::lexer(input);

    for result in lexer {
        match result {
            Ok(token) => tokens.push(token),
            Err(err) => return Err(err),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

impl Token {
    #[cfg(test)]
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Create
                | Token::Select
                | Token::Update
                | Token::Delete
                | Token::Assign
                | Token::Close
                | Token::Reopen
                | Token::Comment
                | Token::From
                | Token::Where
                | Token::And
                | Token::Or
                | Token::Not
                | Token::In
                | Token::Is
                | Token::Null
                | Token::Set
                | Token::To
                | Token::On
                | Token::With
                | Token::Order
                | Token::By
                | Token::Limit
                | Token::Offset
                | Token::Asc
                | Token::Desc
                | Token::Like
                | Token::User
                | Token::Project
                | Token::Issue
                | Token::Issues
                | Token::Users
                | Token::Projects
                | Token::Comments
                | Token::Email
                | Token::Name
                | Token::Title
                | Token::Description
                | Token::Priority
                | Token::Assignee
                | Token::Owner
                | Token::Critical
                | Token::High
                | Token::Medium
                | Token::Low
                | Token::True
                | Token::False
        )
    }

    #[cfg(test)]
    pub fn can_be_field_name(&self) -> bool {
        self.is_keyword() || matches!(self, Token::Identifier(_))
    }

    pub fn to_field_name(&self) -> Option<String> {
        match self {
            Token::Identifier(s) => Some(s.clone()),
            Token::Email => Some("email".to_string()),
            Token::Name => Some("name".to_string()),
            Token::Title => Some("title".to_string()),
            Token::Description => Some("description".to_string()),
            Token::Priority => Some("priority".to_string()),
            Token::Assignee => Some("assignee".to_string()),
            Token::Owner => Some("owner".to_string()),
            Token::User => Some("user".to_string()),
            Token::Project => Some("project".to_string()),
            Token::Issue => Some("issue".to_string()),
            Token::Comment => Some("comment".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_keywords() {
        let tokens = tokenize("CREATE SELECT UPDATE DELETE");
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_case_insensitive() {
        let tokens = tokenize("CrEaTe SeLeCt UpDaTe");
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_string_single_quotes() {
        let tokens = tokenize("'hello world'");
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_string_double_quotes() {
        let tokens = tokenize(r#""hello world""#);
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_escaped_string() {
        let tokens = tokenize(r"'hello\nworld\t!'");
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_escaped_quotes() {
        let tokens = tokenize(r#"'She said \'hello\''"#).unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_number() {
        let tokens = tokenize("123 -456").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_float() {
        let tokens = tokenize("3.14 -0.5").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_operators() {
        let tokens = tokenize("= != > < >= <=").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokens = tokenize("* , . # ( ) [ ]").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_identifier() {
        let tokens = tokenize("my_var my-project user123").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_priority_levels() {
        let tokens = tokenize("critical high medium low").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_boolean() {
        let tokens = tokenize("true false TRUE FALSE").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_entity_types() {
        let tokens = tokenize("users projects issues comments").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_complex_query() {
        let tokens = tokenize("SELECT * FROM issues WHERE status = 'open'").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_with_newlines() {
        let tokens = tokenize("SELECT *\nFROM issues\nWHERE status = 'open'").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_tokenize_field_names() {
        let tokens = tokenize("email name title description priority").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_token_to_field_name() {
        assert_eq!(Token::Email.to_field_name(), Some("email".to_string()));
        assert_eq!(Token::Name.to_field_name(), Some("name".to_string()));
        assert_eq!(Token::Project.to_field_name(), Some("project".to_string()));
        assert_eq!(
            Token::Identifier("custom".to_string()).to_field_name(),
            Some("custom".to_string())
        );
        assert_eq!(Token::Star.to_field_name(), None);
    }

    #[test]
    fn test_is_keyword() {
        assert!(Token::Create.is_keyword());
        assert!(Token::Select.is_keyword());
        assert!(Token::Priority.is_keyword());
        assert!(!Token::Star.is_keyword());
        assert!(!Token::Identifier("test".to_string()).is_keyword());
    }

    #[test]
    fn test_can_be_field_name() {
        assert!(Token::Email.can_be_field_name());
        assert!(Token::Priority.can_be_field_name());
        assert!(Token::Identifier("custom".to_string()).can_be_field_name());
        assert!(!Token::Star.can_be_field_name());
    }

    #[test]
    fn test_unescape_all_sequences() {
        assert_eq!(unescape_string(r"hello\nworld"), "hello\nworld");
        assert_eq!(unescape_string(r"tab\there"), "tab\there");
        assert_eq!(unescape_string(r"back\\slash"), "back\\slash");
        assert_eq!(unescape_string(r"quote\'here"), "quote'here");
        assert_eq!(unescape_string(r#"quote\"here"#), "quote\"here");
        assert_eq!(unescape_string(r"null\0char"), "null\0char");
    }

    #[test]
    fn test_empty_string() {
        let tokens = tokenize("''").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_string_with_spaces() {
        let tokens = tokenize("'hello   world'").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_hyphenated_identifier() {
        let tokens = tokenize("my-project-name").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_all_logical_operators() {
        let tokens = tokenize("AND OR NOT IN IS LIKE").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_complete_create_statement() {
        let input = "CREATE USER alice WITH EMAIL 'alice@example.com'";
        let tokens = tokenize(input).unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }

    #[test]
    fn test_issue_id_format() {
        let tokens = tokenize("backend#123").unwrap();
        insta::assert_debug_snapshot!(&tokens);
    }
}
