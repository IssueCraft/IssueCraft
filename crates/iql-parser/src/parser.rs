use crate::ast::*;
use crate::error::{ParseError, ParseResult};
use crate::lexer::{Token, tokenize};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let tokens = tokenize(input).unwrap_or_else(|_| vec![Token::Eof]);
        Parser {
            tokens,
            position: 0,
        }
    }

    fn get_position_for_error(&self) -> usize {
        self.position + 1
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: Token) -> ParseResult<()> {
        if std::mem::discriminant(self.current()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            })
        }
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if std::mem::discriminant(self.current()) == std::mem::discriminant(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse(&mut self) -> ParseResult<Statement> {
        match self.current() {
            Token::Create => self.parse_create(),
            Token::Select => self.parse_select(),
            Token::Update => self.parse_update(),
            Token::Delete => self.parse_delete(),
            Token::Assign => self.parse_assign(),
            Token::Close => self.parse_close(),
            Token::Reopen => self.parse_reopen(),
            Token::Comment => self.parse_comment(),
            Token::Eof => Err(ParseError::UnexpectedEof),
            _ => Err(ParseError::UnexpectedToken {
                expected: "statement keyword".to_string(),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            }),
        }
    }

    fn parse_create(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Create)?;

        match self.current() {
            Token::User => self.parse_create_user(),
            Token::Project => self.parse_create_project(),
            Token::Issue => self.parse_create_issue(),
            _ => Err(ParseError::UnexpectedToken {
                expected: "USER, PROJECT or ISSUE".to_string(),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            }),
        }
    }

    fn parse_create_user(&mut self) -> ParseResult<Statement> {
        self.expect(Token::User)?;

        let username = self.parse_identifier("USERNAME")?;
        let mut email = None;
        let mut name = None;

        if self.match_token(&Token::With) {
            let mut started = true;
            loop {
                match self.current() {
                    Token::Email => {
                        self.advance();
                        email = Some(self.parse_string_value("EMAIL")?);
                        started = false;
                    }
                    Token::Name => {
                        self.advance();
                        name = Some(self.parse_string_value("NAME")?);
                        started = false;
                    }
                    Token::Identifier(id) if id.eq_ignore_ascii_case("email") => {
                        self.advance();
                        email = Some(self.parse_string_value("EMAIL")?);
                        started = false;
                    }
                    Token::Identifier(id) if id.eq_ignore_ascii_case("name") => {
                        self.advance();
                        name = Some(self.parse_string_value("NAME")?);
                        started = false;
                    }
                    _ => break,
                }
            }
            if started {
                return Err(ParseError::MissingClause {
                    clause: "at least one of EMAIL or NAME".to_string(),
                    position: self.get_position_for_error(),
                });
            }
        }

        Ok(Statement::Create(CreateStatement::User {
            username,
            email,
            name,
        }))
    }

    fn parse_create_project(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Project)?;

        let project_id = self.parse_identifier("PROJECT_ID")?;
        let mut name = None;
        let mut description = None;
        let mut owner = None;

        if self.match_token(&Token::With) {
            let mut started = true;
            loop {
                match self.current() {
                    Token::Name => {
                        self.advance();
                        name = Some(self.parse_string_value("NAME")?);
                        started = false;
                    }
                    Token::Description => {
                        self.advance();
                        description = Some(self.parse_string_value("DESCRIPTION")?);
                        started = false;
                    }
                    Token::Owner => {
                        self.advance();
                        owner = Some(self.parse_identifier("OWNER")?);
                    }
                    Token::Identifier(id) if id.eq_ignore_ascii_case("name") => {
                        self.advance();
                        name = Some(self.parse_string_value("NAME")?);
                        started = false;
                    }
                    Token::Identifier(id) if id.eq_ignore_ascii_case("description") => {
                        self.advance();
                        description = Some(self.parse_string_value("DESCRIPTION")?);
                        started = false;
                    }
                    Token::Identifier(id) if id.eq_ignore_ascii_case("owner") => {
                        self.advance();
                        owner = Some(self.parse_identifier("OWNER")?);
                        started = false;
                    }
                    _ => break,
                }
            }
            if started {
                return Err(ParseError::MissingClause {
                    clause: "at least one of NAME, DESCRIPTION, or OWNER".to_string(),
                    position: self.get_position_for_error(),
                });
            }
        }

        Ok(Statement::Create(CreateStatement::Project {
            project_id,
            name,
            description,
            owner,
        }))
    }

    fn parse_create_issue(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Issue)?;
        self.expect(Token::Of)?;
        self.expect(Token::Kind)?;
        let kind = self.parse_issue_kind()?;

        self.expect(Token::In)?;

        let project = self.parse_identifier("PROJECT_ID")?;

        if !self.match_token(&Token::With) {
            return Err(ParseError::MissingClause {
                clause: "WITH".to_string(),
                position: self.get_position_for_error(),
            });
        }

        let mut title = None;
        let mut description = None;
        let mut priority = None;
        let mut assignee = None;

        loop {
            match self.current() {
                Token::Title => {
                    self.advance();
                    title = Some(self.parse_string_value("TITLE")?);
                }
                Token::Description => {
                    self.advance();
                    description = Some(self.parse_string_value("DESCRIPTION")?);
                }
                Token::Priority => {
                    self.advance();
                    priority = Some(self.parse_priority()?);
                }
                Token::Assignee => {
                    self.advance();
                    assignee = Some(UserId(self.parse_identifier("ASSIGNEE_ID")?));
                }
                Token::Identifier(id) if id.eq_ignore_ascii_case("title") => {
                    self.advance();
                    title = Some(self.parse_string_value("TITLE")?);
                }
                Token::Identifier(id) if id.eq_ignore_ascii_case("description") => {
                    self.advance();
                    description = Some(self.parse_string_value("DESCRIPTION")?);
                }
                Token::Identifier(id) if id.eq_ignore_ascii_case("priority") => {
                    self.advance();
                    priority = Some(self.parse_priority()?);
                }
                Token::Identifier(id) if id.eq_ignore_ascii_case("assignee") => {
                    self.advance();
                    assignee = Some(UserId(self.parse_identifier("ASSIGNEE_ID")?));
                }
                _ => break,
            }
        }

        let title = title.ok_or_else(|| ParseError::MissingClause {
            clause: "TITLE".to_string(),
            position: self.get_position_for_error(),
        })?;

        Ok(Statement::Create(CreateStatement::Issue {
            project,
            title,
            description,
            priority,
            assignee,
            kind,
        }))
    }

    fn parse_select(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Select)?;

        let columns = self.parse_columns()?;

        self.expect(Token::From)?;

        let from = self.parse_entity_type()?;

        let filter = if self.match_token(&Token::Where) {
            Some(self.parse_filter_expression()?)
        } else {
            None
        };

        let order_by = if self.match_token(&Token::Order) {
            self.expect(Token::By)?;
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let limit = if self.match_token(&Token::Limit) {
            Some(self.parse_number()? as u32)
        } else {
            None
        };

        let offset = if self.match_token(&Token::Offset) {
            Some(self.parse_number()? as u32)
        } else {
            None
        };

        Ok(Statement::Select(SelectStatement {
            columns,
            from,
            filter,
            order_by,
            limit,
            offset,
        }))
    }

    fn parse_columns(&mut self) -> ParseResult<Columns> {
        if self.match_token(&Token::Star) {
            return Ok(Columns::All);
        }

        let mut columns = Vec::new();
        loop {
            let col = self.parse_identifier("COLUMN")?;
            columns.push(col);

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        Ok(Columns::Named(columns))
    }

    fn parse_entity_type(&mut self) -> ParseResult<EntityType> {
        let entity = match self.current() {
            Token::Users => EntityType::Users,
            Token::Projects => EntityType::Projects,
            Token::Issues => EntityType::Issues,
            Token::Comments => EntityType::Comments,
            _ => {
                return Err(ParseError::InvalidEntityType {
                    value: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };
        self.advance();
        Ok(entity)
    }

    // Parse filter expression (WHERE clause)
    //
    // This uses operator precedence climbing:
    // - OR has lowest precedence
    // - AND has higher precedence
    // - NOT and parentheses have highest precedence
    fn parse_filter_expression(&mut self) -> ParseResult<FilterExpression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> ParseResult<FilterExpression> {
        let mut left = self.parse_and_expression()?;

        while self.match_token(&Token::Or) {
            let right = self.parse_and_expression()?;
            left = FilterExpression::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> ParseResult<FilterExpression> {
        let mut left = self.parse_primary_filter()?;

        while self.match_token(&Token::And) {
            let right = self.parse_primary_filter()?;
            left = FilterExpression::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    // Parse primary filter expression
    //
    // Handles:
    // - NOT expressions
    // - Parenthesized expressions
    // - Field comparisons
    // - IS NULL / IS NOT NULL
    // - IN clauses
    fn parse_primary_filter(&mut self) -> ParseResult<FilterExpression> {
        if self.match_token(&Token::Not) {
            let expr = self.parse_primary_filter()?;
            return Ok(FilterExpression::Not(Box::new(expr)));
        }

        if self.match_token(&Token::LeftParen) {
            let expr = self.parse_filter_expression()?;
            self.expect(Token::RightParen)?;
            return Ok(expr);
        }

        let field = self.parse_field_name()?;

        if self.match_token(&Token::Is) {
            if self.match_token(&Token::Not) {
                self.expect(Token::Null)?;
                return Ok(FilterExpression::IsNotNull(field));
            } else if self.match_token(&Token::Null) {
                return Ok(FilterExpression::IsNull(field));
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "NULL or NOT NULL".to_string(),
                    found: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        }

        if self.match_token(&Token::In) {
            self.expect(Token::LeftParen)?;
            let values = self.parse_value_list()?;
            self.expect(Token::RightParen)?;
            return Ok(FilterExpression::In { field, values });
        }

        let op = self.parse_comparison_op()?;
        let value = self.parse_value()?;
        Ok(FilterExpression::Comparison { field, op, value })
    }

    fn parse_field_name(&mut self) -> ParseResult<String> {
        if let Some(name) = self.current().to_field_name() {
            self.advance();
            Ok(name)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "field name".to_string(),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            })
        }
    }

    fn parse_comparison_op(&mut self) -> ParseResult<ComparisonOp> {
        let op = match self.current() {
            Token::Equal => ComparisonOp::Equal,
            Token::NotEqual => ComparisonOp::NotEqual,
            Token::GreaterThan => ComparisonOp::GreaterThan,
            Token::LessThan => ComparisonOp::LessThan,
            Token::GreaterOrEqual => ComparisonOp::GreaterThanOrEqual,
            Token::LessOrEqual => ComparisonOp::LessThanOrEqual,
            Token::Like => ComparisonOp::Like,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "comparison operator".to_string(),
                    found: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };
        self.advance();
        Ok(op)
    }

    fn parse_order_by(&mut self) -> ParseResult<OrderBy> {
        let field = self.parse_identifier("FIELD")?;

        let direction = if self.match_token(&Token::Desc) {
            OrderDirection::Desc
        } else {
            self.match_token(&Token::Asc);
            OrderDirection::Asc
        };

        Ok(OrderBy { field, direction })
    }

    fn parse_update(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Update)?;

        let entity = self.parse_update_target()?;

        self.expect(Token::Set)?;

        let updates = self.parse_field_updates()?;

        Ok(Statement::Update(UpdateStatement { entity, updates }))
    }

    fn parse_update_target(&mut self) -> ParseResult<UpdateTarget> {
        let target = match self.current() {
            Token::User => {
                self.advance();
                let username = self.parse_identifier("USERNAME")?;
                UpdateTarget::User(UserId(username))
            }
            Token::Project => {
                self.advance();
                let project = self.parse_identifier("PROJECT")?;
                UpdateTarget::Project(ProjectId(project))
            }
            Token::Issue => {
                self.advance();
                let issue_id = self.parse_issue_id()?;
                UpdateTarget::Issue(issue_id)
            }
            Token::Comment => {
                self.advance();
                let comment_id = self.parse_identifier("COMMENT")?;
                UpdateTarget::Comment(CommentId(comment_id))
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "USER, PROJECT, ISSUE, or COMMENT".to_string(),
                    found: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };

        Ok(target)
    }

    fn parse_field_updates(&mut self) -> ParseResult<Vec<FieldUpdate>> {
        let mut updates = Vec::new();

        loop {
            let field = self.parse_identifier("FIELD")?;
            self.expect(Token::Equal)?;
            let value = self.parse_value()?;

            updates.push(FieldUpdate { field, value });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        Ok(updates)
    }

    fn parse_delete(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Delete)?;

        let entity = self.parse_delete_target()?;

        Ok(Statement::Delete(DeleteStatement { entity }))
    }

    fn parse_delete_target(&mut self) -> ParseResult<DeleteTarget> {
        let target = match self.current() {
            Token::User => {
                self.advance();
                let username = self.parse_identifier("USERNAME")?;
                DeleteTarget::User(username)
            }
            Token::Project => {
                self.advance();
                let project = self.parse_identifier("PROJECT")?;
                DeleteTarget::Project(project)
            }
            Token::Issue => {
                self.advance();
                let issue_id = self.parse_issue_id()?;
                DeleteTarget::Issue(issue_id)
            }
            Token::Comment => {
                self.advance();
                let id = self.parse_number()? as u64;
                DeleteTarget::Comment(id)
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "USER, PROJECT, ISSUE, or COMMENT".to_string(),
                    found: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };

        Ok(target)
    }

    fn parse_assign(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Assign)?;
        self.expect(Token::Issue)?;

        let issue_id = self.parse_issue_id()?;

        self.expect(Token::To)?;

        let assignee = self.parse_identifier("ASSIGNEE")?;

        Ok(Statement::Assign(AssignStatement { issue_id, assignee }))
    }

    fn parse_close(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Close)?;
        self.expect(Token::Issue)?;

        let issue_id = self.parse_issue_id()?;

        let reason = if self.match_token(&Token::With) {
            Some(self.parse_close_reason()?)
        } else {
            None
        };

        Ok(Statement::Close(CloseStatement { issue_id, reason }))
    }

    fn parse_reopen(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Reopen)?;
        self.expect(Token::Issue)?;

        let issue_id = self.parse_issue_id()?;

        Ok(Statement::Reopen(ReopenStatement { issue_id }))
    }

    fn parse_comment(&mut self) -> ParseResult<Statement> {
        self.expect(Token::Comment)?;
        self.expect(Token::On)?;
        self.expect(Token::Issue)?;

        let issue_id = self.parse_issue_id()?;

        self.expect(Token::With)?;

        let content = self.parse_string_value("CONTENT")?;

        Ok(Statement::Comment(CommentStatement { issue_id, content }))
    }

    fn parse_close_reason(&mut self) -> ParseResult<CloseReason> {
        let priority = match self.current() {
            Token::Duplicate => CloseReason::Duplicate,
            Token::WontFix => CloseReason::WontFix,
            Token::Done => CloseReason::Done,
            _ => {
                return Err(ParseError::InvalidCloseReason {
                    value: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };
        self.advance();
        Ok(priority)
    }

    fn parse_issue_id(&mut self) -> ParseResult<IssueId> {
        if let Token::Identifier(project) = self.current() {
            let project = project.clone();
            self.advance();

            if self.match_token(&Token::Hash) {
                let number = self.parse_number()? as u64;
                return Ok(IssueId(format!("{}#{}", project, number)));
            } else {
                return Err(ParseError::InvalidIssueId {
                    value: project,
                    position: self.get_position_for_error(),
                });
            }
        }

        Err(ParseError::UnexpectedToken {
            expected: "issue ID (project#number)".to_string(),
            found: format!("{:?}", self.current()),
            position: self.get_position_for_error(),
        })
    }

    fn parse_issue_kind(&mut self) -> ParseResult<IssueKind> {
        let kind = match self.current() {
            Token::Epic => IssueKind::Epic,
            Token::Improvement => IssueKind::Improvement,
            Token::Bug => IssueKind::Bug,
            Token::Task => IssueKind::Task,
            _ => {
                return Err(ParseError::InvalidIssueKind {
                    value: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };
        self.advance();
        Ok(kind)
    }

    fn parse_priority(&mut self) -> ParseResult<Priority> {
        let priority = match self.current() {
            Token::Critical => Priority::Critical,
            Token::High => Priority::High,
            Token::Medium => Priority::Medium,
            Token::Low => Priority::Low,
            _ => {
                return Err(ParseError::InvalidPriority {
                    value: format!("{:?}", self.current()),
                    position: self.get_position_for_error(),
                });
            }
        };
        self.advance();
        Ok(priority)
    }

    fn parse_value(&mut self) -> ParseResult<IqlValue> {
        match self.current() {
            Token::String(s) => {
                let value = IqlValue::String(s.clone());
                self.advance();
                Ok(value)
            }
            Token::Number(n) => {
                let value = IqlValue::Number(*n);
                self.advance();
                Ok(value)
            }
            Token::Float(f) => {
                let value = IqlValue::Float(*f);
                self.advance();
                Ok(value)
            }
            Token::True => {
                self.advance();
                Ok(IqlValue::Boolean(true))
            }
            Token::False => {
                self.advance();
                Ok(IqlValue::Boolean(false))
            }
            Token::Null => {
                self.advance();
                Ok(IqlValue::Null)
            }
            Token::Critical => {
                self.advance();
                Ok(IqlValue::Priority(Priority::Critical))
            }
            Token::High => {
                self.advance();
                Ok(IqlValue::Priority(Priority::High))
            }
            Token::Medium => {
                self.advance();
                Ok(IqlValue::Priority(Priority::Medium))
            }
            Token::Low => {
                self.advance();
                Ok(IqlValue::Priority(Priority::Low))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "literal".to_string(),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            }),
        }
    }

    fn parse_value_list(&mut self) -> ParseResult<Vec<IqlValue>> {
        let mut values = Vec::new();

        loop {
            values.push(self.parse_value()?);

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        Ok(values)
    }

    fn parse_string_value(&mut self, expected_name: &str) -> ParseResult<String> {
        if let Token::String(s) | Token::Identifier(s) = self.current() {
            let value = s.clone();
            self.advance();
            Ok(value)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("string literal for <{}>", expected_name),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            })
        }
    }

    fn parse_identifier(&mut self, expected_name: &str) -> ParseResult<String> {
        if let Token::Identifier(id) = self.current() {
            let value = id.clone();
            self.advance();
            Ok(value)
        } else if let Some(name) = self.current().to_field_name() {
            self.advance();
            Ok(name)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("identifier for <{}>", expected_name),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            })
        }
    }

    fn parse_number(&mut self) -> ParseResult<i64> {
        if let Token::Number(n) = self.current() {
            let value = *n;
            self.advance();
            Ok(value)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "number".to_string(),
                found: format!("{:?}", self.current()),
                position: self.get_position_for_error(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_user_simple() {
        let mut parser = Parser::new("CREATE USER alice");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_create_user_with_details() {
        let mut parser = Parser::new("CREATE USER bob WITH EMAIL 'bob@test.com' NAME 'Bob Smith'");
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(Statement::Create(CreateStatement::User {
            username,
            email,
            name,
        })) = result
        {
            assert_eq!(username, "bob");
            assert_eq!(email, Some("bob@test.com".to_string()));
            assert_eq!(name, Some("Bob Smith".to_string()));
        }
    }

    #[test]
    fn test_parse_select_all() {
        let mut parser = Parser::new("SELECT * FROM issues");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_columns() {
        let mut parser = Parser::new("SELECT title, status FROM issues");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_with_filter() {
        let mut parser = Parser::new("SELECT * FROM issues WHERE status = 'open'");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_complex_filter() {
        let mut parser =
            Parser::new("SELECT * FROM issues WHERE status = 'open' AND priority = high");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_update_issue() {
        let mut parser = Parser::new("UPDATE issue backend#123 SET status = 'closed'");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_assign() {
        let mut parser = Parser::new("ASSIGN issue backend#456 TO alice");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_close() {
        let mut parser = Parser::new("CLOSE issue backend#789");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_comment() {
        let mut parser = Parser::new("COMMENT ON issue backend#101 WITH 'Great work!'");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_issue_id_project() {
        let mut parser = Parser::new("CLOSE issue backend#42");
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(Statement::Close(stmt)) = result {
            assert_eq!(stmt.issue_id, IssueId("backend#42".to_string()));
        }
    }

    #[test]
    fn test_parse_create_issue() {
        let mut parser =
            Parser::new("CREATE ISSUE IN my-project WITH TITLE 'New feature' PRIORITY high");
        let result = parser.parse();
        assert!(result.is_ok());
    }
}
