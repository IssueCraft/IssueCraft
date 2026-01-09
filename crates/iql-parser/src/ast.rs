use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Create(CreateStatement),
    Select(SelectStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Assign(AssignStatement),
    Close(CloseStatement),
    Comment(CommentStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreateStatement {
    User {
        username: String,
        email: Option<String>,
        name: Option<String>,
    },
    Project {
        project_id: String,
        name: Option<String>,
        description: Option<String>,
        owner: Option<String>,
    },
    Issue {
        project: String,
        title: String,
        description: Option<String>,
        priority: Option<Priority>,
        assignee: Option<String>,
        labels: Vec<String>,
    },
    Comment {
        issue_id: IssueId,
        content: String,
        author: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub columns: Vec<Column>,
    pub from: EntityType,
    pub filter: Option<FilterExpression>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    All,
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityType {
    Users,
    Projects,
    Issues,
    Comments,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Users => write!(f, "users"),
            EntityType::Projects => write!(f, "projects"),
            EntityType::Issues => write!(f, "issues"),
            EntityType::Comments => write!(f, "comments"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpression {
    Comparison {
        field: String,
        op: ComparisonOp,
        value: Value,
    },
    And(Box<FilterExpression>, Box<FilterExpression>),
    Or(Box<FilterExpression>, Box<FilterExpression>),
    Not(Box<FilterExpression>),
    In {
        field: String,
        values: Vec<Value>,
    },
    IsNull(String),
    IsNotNull(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Like,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub field: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatement {
    pub entity: UpdateTarget,
    pub updates: Vec<FieldUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateTarget {
    User(String),
    Project(String),
    Issue(IssueId),
    Comment(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldUpdate {
    pub field: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub entity: DeleteTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteTarget {
    User(String),
    Project(String),
    Issue(IssueId),
    Comment(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStatement {
    pub issue_id: IssueId,
    pub assignee: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloseStatement {
    pub issue_id: IssueId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentStatement {
    pub issue_id: IssueId,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IssueId {
    pub project: String,
    pub number: u64,
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.project, self.number)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Priority(Priority),
    Identifier(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "'{}'", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "NULL"),
            Value::Priority(p) => write!(f, "{}", p),
            Value::Identifier(id) => write!(f, "{}", id),
        }
    }
}
