mod ast;
mod error;
mod lexer;
mod parser;

use std::fmt::Display;

pub use ast::*;
use async_trait::async_trait;
pub use error::{ParseError, ParseResult};
use parser::Parser;

pub fn parse_query(query: &str) -> ParseResult<Statement> {
    let mut parser = Parser::new(query);
    parser.parse()
}

#[derive(thiserror::Error, Debug)]
pub enum IqlError {
    #[error("IQL query could not be parsed: {0}")]
    MalformedIql(#[from] ParseError),
    #[error("Not implemented")]
    NotImplemented,
    #[error("This action is not supported by the chosen backend")]
    NotSupported,
    #[error("A project with the name '{0}' already exists")]
    ProjectAlreadyExists(String),
    #[error("No project with the name '{0}' exists")]
    ProjectNotFound(String),
    #[error("No issue with the name '{0}' exists")]
    IssueNotFound(String),
    #[error("The issue withe the name '{0}' was already closed. Reason '{1}'")]
    IssueAlreadyClosed(String, CloseReason),
    #[error("{0}")]
    ImplementationSpecific(String),
}

#[async_trait]
pub trait ExecutionEngine {
    async fn execute(&mut self, query: &str) -> Result<ExecutionResult, IqlError>;
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub affected_rows: u128,
    pub info: Option<String>,
}

impl Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Affected Rows: {}", self.affected_rows)?;
        if let Some(info) = &self.info {
            write!(f, "\nInfo: {}", info)?;
        }
        Ok(())
    }
}

impl From<String> for ExecutionResult {
    fn from(s: String) -> Self {
        Self {
            affected_rows: 0,
            info: Some(s),
        }
    }
}

impl From<&str> for ExecutionResult {
    fn from(s: &str) -> Self {
        Self {
            affected_rows: 0,
            info: Some(s.to_string()),
        }
    }
}

impl ExecutionResult {
    pub fn new(rows: u128) -> Self {
        Self {
            affected_rows: rows,
            info: None,
        }
    }

    pub fn one() -> Self {
        Self {
            affected_rows: 1,
            info: None,
        }
    }

    pub fn zero() -> Self {
        Self {
            affected_rows: 0,
            info: None,
        }
    }

    pub fn with_info(mut self, info: &str) -> Self {
        self.info = Some(info.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_user() {
        let query = "CREATE USER john_doe WITH EMAIL 'john@example.com' NAME 'John Doe'";
        let result = parse_query(query);
        assert!(result.is_ok());

        if let Ok(Statement::Create(CreateStatement::User {
            username,
            email,
            name,
        })) = result
        {
            assert_eq!(username, "john_doe");
            assert_eq!(email, Some("john@example.com".to_string()));
            assert_eq!(name, Some("John Doe".to_string()));
        } else {
            panic!("Expected CreateStatement::User");
        }
    }

    #[test]
    fn test_parse_create_project() {
        let query = "CREATE PROJECT my-project WITH NAME 'My Project' DESCRIPTION 'A test project'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_create_issue() {
        let query = "CREATE ISSUE IN my-project WITH TITLE 'Bug found' DESCRIPTION 'Something broke' PRIORITY high ASSIGNEE john_doe";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_all() {
        let query = "SELECT * FROM issues";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_with_where() {
        let query = "SELECT * FROM issues WHERE status = 'open' AND priority = high";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_update() {
        let query = "UPDATE issue my-project#123 SET status = 'closed', priority = low";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_delete() {
        let query = "DELETE issue my-project#456";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_assign() {
        let query = "ASSIGN issue my-project#789 TO alice";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_close() {
        let query = "CLOSE issue my-project#101";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_comment() {
        let query = "COMMENT ON issue my-project#202 WITH 'This is a comment'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complex_query() {
        let query = "SELECT title, status, assignee FROM issues WHERE project = 'backend' AND (priority = high OR status = 'critical') ORDER BY created_at DESC LIMIT 10";
        let result = parse_query(query);
        if let Err(ref e) = result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_project_qualified_issue() {
        let query = "CLOSE issue my-project#42 WITH 'Completed'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_labels() {
        let query = "CREATE ISSUE IN frontend WITH TITLE 'Test' LABELS [bug, urgent, frontend]";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_multiple_field_updates() {
        let query = "UPDATE issue my-project#100 SET status = 'closed', priority = medium, assignee = 'bob'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_in_operator() {
        let query = "SELECT * FROM issues WHERE priority IN (critical, high)";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_is_null() {
        let query = "SELECT * FROM issues WHERE assignee IS NULL";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_is_not_null() {
        let query = "SELECT * FROM issues WHERE assignee IS NOT NULL";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_not_operator() {
        let query = "SELECT * FROM issues WHERE NOT status = 'closed'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_like_operator() {
        let query = "SELECT * FROM issues WHERE title LIKE '%bug%'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_order_asc() {
        let query = "SELECT * FROM issues ORDER BY created_at ASC";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_offset() {
        let query = "SELECT * FROM issues LIMIT 10 OFFSET 20";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_all_priorities() {
        let queries = vec![
            "CREATE ISSUE IN test WITH TITLE 'Test' PRIORITY critical",
            "CREATE ISSUE IN test WITH TITLE 'Test' PRIORITY high",
            "CREATE ISSUE IN test WITH TITLE 'Test' PRIORITY medium",
            "CREATE ISSUE IN test WITH TITLE 'Test' PRIORITY low",
        ];

        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed to parse: {}", query);
        }
    }

    #[test]
    fn test_parse_all_entity_types() {
        let queries = vec![
            "SELECT * FROM users",
            "SELECT * FROM projects",
            "SELECT * FROM issues",
            "SELECT * FROM comments",
        ];

        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed to parse: {}", query);
        }
    }

    #[test]
    fn test_integration_workflow() {
        let queries = vec![
            "CREATE USER alice WITH EMAIL 'alice@test.com' NAME 'Alice'",
            "CREATE PROJECT backend WITH NAME 'Backend' OWNER alice",
            "CREATE ISSUE IN backend WITH TITLE 'Bug fix' PRIORITY high ASSIGNEE alice",
            "SELECT * FROM issues WHERE assignee = 'alice'",
            "ASSIGN issue backend#1 TO alice",
            "COMMENT ON ISSUE backend#1 WITH 'Working on it'",
            "UPDATE issue backend#1 SET status = 'in-progress'",
            "CLOSE issue backend#1 WITH 'Fixed'",
        ];

        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed to parse: {}", query);
        }
    }

    #[test]
    fn test_empty_labels() {
        let query = "CREATE ISSUE IN test WITH TITLE 'Test' LABELS []";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Create(CreateStatement::Issue { labels, .. })) = result {
            assert_eq!(labels.len(), 0);
        }
    }

    #[test]
    fn test_string_with_multiple_escapes() {
        let query = r"CREATE ISSUE IN test WITH TITLE 'Line1\nLine2\tTab\rReturn\\Backslash'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_negative_numbers() {
        let query = "UPDATE issue test#100 SET count = -50";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_float_values() {
        let query = "UPDATE issue test#100 SET score = 3.14159";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deeply_nested_filters() {
        let query = "SELECT * FROM issues WHERE ((a = 1 AND b = 2) OR (c = 3 AND d = 4)) AND e = 5";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_not_with_parentheses() {
        let query = "SELECT * FROM issues WHERE NOT (status = 'closed' OR status = 'archived')";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_with_priorities() {
        let query = "SELECT * FROM issues WHERE priority IN (critical, high, medium)";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_with_strings() {
        let query = "SELECT * FROM issues WHERE status IN ('open', 'in-progress', 'review')";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_comparison_operators() {
        let queries = vec![
            "SELECT * FROM issues WHERE count > 10",
            "SELECT * FROM issues WHERE count < 5",
            "SELECT * FROM issues WHERE count >= 10",
            "SELECT * FROM issues WHERE count <= 5",
            "SELECT * FROM issues WHERE status != 'closed'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_case_insensitive_keywords() {
        let queries = vec![
            "select * from issues",
            "SELECT * FROM ISSUES",
            "SeLeCt * FrOm IsSuEs",
            "create user alice",
            "CREATE USER ALICE",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_hyphenated_identifiers() {
        let queries = vec![
            "CREATE USER my-user-name",
            "CREATE PROJECT my-cool-project",
            "SELECT * FROM issues WHERE project = 'my-backend-api'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_keywords_as_field_names() {
        let queries = vec![
            "SELECT project, user, issue FROM issues",
            "SELECT * FROM issues WHERE project = 'test'",
            "SELECT * FROM issues WHERE user = 'alice'",
            "UPDATE issue test#1 SET comment = 'test'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_all_field_keywords_in_create() {
        let query = "CREATE ISSUE IN test WITH TITLE 'T' DESCRIPTION 'D' PRIORITY high ASSIGNEE alice LABELS [bug]";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_delete_targets() {
        let queries = vec![
            "DELETE user alice",
            "DELETE project backend",
            "DELETE issue backend#456",
            "DELETE comment 789",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_all_update_targets() {
        let queries = vec![
            "UPDATE user alice SET email = 'new@test.com'",
            "UPDATE project backend SET name = 'New Name'",
            "UPDATE issue backend#123 SET status = 'closed'",
            "UPDATE issue backend#456 SET priority = high",
            "UPDATE comment 789 SET content = 'updated'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_multiple_columns_select() {
        let query =
            "SELECT id, title, status, priority, assignee, created_at, updated_at FROM issues";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Select(select)) = result {
            assert_eq!(select.columns.len(), 7);
        }
    }

    #[test]
    fn test_limit_and_offset_together() {
        let query = "SELECT * FROM issues LIMIT 50 OFFSET 100";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Select(select)) = result {
            assert_eq!(select.limit, Some(50));
            assert_eq!(select.offset, Some(100));
        }
    }

    #[test]
    fn test_order_by_asc_explicit() {
        let query = "SELECT * FROM issues ORDER BY created_at ASC";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Select(select)) = result {
            assert!(select.order_by.is_some());
            let order = select.order_by.unwrap();
            assert_eq!(order.direction, OrderDirection::Asc);
        }
    }

    #[test]
    fn test_boolean_values() {
        let queries = vec![
            "UPDATE issue backend#1 SET active = true",
            "UPDATE issue backend#1 SET archived = false",
            "SELECT * FROM issues WHERE active = TRUE",
            "SELECT * FROM issues WHERE archived = FALSE",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_null_values() {
        let queries = vec![
            "UPDATE issue backend#1 SET assignee = null",
            "SELECT * FROM issues WHERE assignee = NULL",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_create_comment_variations() {
        let queries = vec![
            "CREATE COMMENT ON ISSUE backend#123 WITH 'Simple comment'",
            "CREATE COMMENT ON ISSUE backend#123 WITH 'Comment' AUTHOR alice",
            "CREATE COMMENT ON ISSUE backend#456 WITH 'Project issue comment'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_comment_statement() {
        let query = "COMMENT ON ISSUE backend#123 WITH 'Quick comment'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_close_with_and_without_reason() {
        let queries = vec![
            "CLOSE issue backend#123",
            "CLOSE issue backend#123 WITH 'Completed'",
            "CLOSE issue backend#456 WITH 'Duplicate of #455'",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_empty_string_value() {
        let query = "UPDATE issue backend#1 SET description = ''";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_special_characters_in_strings() {
        let query =
            r"CREATE ISSUE IN test WITH TITLE 'Special chars: !@#$%^&*()_+-={}[]|:;<>?,./~`'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_double_quotes_in_strings() {
        let query = r#"CREATE ISSUE IN test WITH TITLE "Double quoted string""#;
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_labels_with_hyphens() {
        let query =
            "CREATE ISSUE IN test WITH TITLE 'Test' LABELS [high-priority, bug-fix, ui-component]";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_real_world_query() {
        let query = r#"
            SELECT title, status, priority, assignee, created_at
            FROM issues
            WHERE (priority = critical OR priority = high)
              AND status IN ('open', 'in-progress')
              AND assignee IS NOT NULL
              AND project = 'backend'
            ORDER BY priority DESC
            LIMIT 25
            OFFSET 0
        "#;
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_minimal_create_user() {
        let query = "CREATE USER alice";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Create(CreateStatement::User { email, name, .. })) = result {
            assert!(email.is_none());
            assert!(name.is_none());
        }
    }

    #[test]
    fn test_minimal_create_project() {
        let query = "CREATE PROJECT test";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_select_from_all_entities() {
        for entity in &["users", "projects", "issues", "comments"] {
            let query = format!("SELECT * FROM {}", entity);
            let result = parse_query(&query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_issue_id_variations() {
        let queries = vec![
            "CLOSE issue a#1",
            "CLOSE issue my-project#123",
            "CLOSE issue backend_api#456",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_priority_in_different_cases() {
        let queries = vec![
            "CREATE ISSUE IN test WITH TITLE 'T' PRIORITY critical",
            "CREATE ISSUE IN test WITH TITLE 'T' PRIORITY CRITICAL",
            "CREATE ISSUE IN test WITH TITLE 'T' PRIORITY Critical",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_all_comparison_ops_with_strings() {
        let query = "SELECT * FROM issues WHERE title LIKE '%bug%'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_column_select() {
        let query = "SELECT title FROM issues";
        let result = parse_query(query);
        assert!(result.is_ok());
        if let Ok(Statement::Select(select)) = result {
            assert_eq!(select.columns.len(), 1);
        }
    }

    #[test]
    fn test_whitespace_variations() {
        let queries = vec![
            "SELECT * FROM issues",
            "SELECT  *  FROM  issues",
            "SELECT\t*\tFROM\tissues",
            "SELECT\n*\nFROM\nissues",
        ];
        for query in queries {
            let result = parse_query(query);
            assert!(result.is_ok(), "Failed: {}", query);
        }
    }

    #[test]
    fn test_field_update_with_priority() {
        let query = "UPDATE issue backend#1 SET priority = critical, status = 'open'";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_field_update_with_identifier() {
        let query = "UPDATE issue backend#1 SET assignee = alice, project = backend";
        let result = parse_query(query);
        assert!(result.is_ok());
    }
}
