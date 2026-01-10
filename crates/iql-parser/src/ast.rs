use std::fmt;

use facet_value::Value as FacetValue;

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
    pub columns: Columns,
    pub from: EntityType,
    pub filter: Option<FilterExpression>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Columns {
    All,
    Named(Vec<String>),
}

impl Columns {
    pub fn len(&self) -> usize {
        match self {
            Columns::All => usize::MAX,
            Columns::Named(cols) => cols.len(),
        }
    }
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

impl FilterExpression {
    pub fn matches(&self, value: &FacetValue) -> bool {
        match self {
            FilterExpression::Comparison {
                field,
                op,
                value: filter_value,
            } => {
                let obj = match value.as_object() {
                    Some(obj) => obj,
                    None => return false,
                };

                let field_value = match obj.get(field) {
                    Some(v) => v,
                    None => return false,
                };

                Self::compare_values(field_value, op, filter_value)
            }
            FilterExpression::And(left, right) => left.matches(value) && right.matches(value),
            FilterExpression::Or(left, right) => left.matches(value) || right.matches(value),
            FilterExpression::Not(expr) => !expr.matches(value),
            FilterExpression::In { field, values } => {
                let obj = match value.as_object() {
                    Some(obj) => obj,
                    None => return false,
                };

                let field_value = match obj.get(field) {
                    Some(v) => v,
                    None => return false,
                };

                values.iter().any(|filter_val| {
                    Self::compare_values(field_value, &ComparisonOp::Equal, filter_val)
                })
            }
            FilterExpression::IsNull(field) => {
                let obj = match value.as_object() {
                    Some(obj) => obj,
                    None => return false,
                };

                match obj.get(field) {
                    None => true,
                    Some(v) => v.is_null(),
                }
            }
            FilterExpression::IsNotNull(field) => {
                let obj = match value.as_object() {
                    Some(obj) => obj,
                    None => return false,
                };

                match obj.get(field) {
                    None => false,
                    Some(v) => !v.is_null(),
                }
            }
        }
    }

    fn compare_values(field_value: &FacetValue, op: &ComparisonOp, filter_value: &Value) -> bool {
        match op {
            ComparisonOp::Equal => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    field_value == &converted
                } else {
                    false
                }
            }
            ComparisonOp::NotEqual => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    field_value != &converted
                } else {
                    true
                }
            }
            ComparisonOp::GreaterThan => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    field_value.partial_cmp(&converted) == Some(std::cmp::Ordering::Greater)
                } else {
                    false
                }
            }
            ComparisonOp::LessThan => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    field_value.partial_cmp(&converted) == Some(std::cmp::Ordering::Less)
                } else {
                    false
                }
            }
            ComparisonOp::GreaterThanOrEqual => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    matches!(
                        field_value.partial_cmp(&converted),
                        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                    )
                } else {
                    false
                }
            }
            ComparisonOp::LessThanOrEqual => {
                if let Some(converted) = Self::convert_iql_value_to_facet(filter_value) {
                    matches!(
                        field_value.partial_cmp(&converted),
                        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                    )
                } else {
                    false
                }
            }
            ComparisonOp::Like => {
                let field_str = field_value.as_string().map(|s| s.as_str()).unwrap_or("");
                if let Value::String(pattern) = filter_value {
                    let pattern = pattern.replace("%", ".*");
                    if let Ok(regex) = regex::Regex::new(&format!("^{}$", pattern)) {
                        regex.is_match(field_str)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    fn convert_iql_value_to_facet(iql_value: &Value) -> Option<FacetValue> {
        match iql_value {
            Value::String(s) => Some(facet_value::VString::new(s).into_value()),
            Value::Number(n) => Some(facet_value::VNumber::from_u64(*n as u64).into_value()),
            Value::Float(f) => Some(facet_value::VNumber::from_f64(*f as f64)?.into_value()),
            Value::Boolean(b) => Some(if *b {
                facet_value::Value::TRUE
            } else {
                facet_value::Value::FALSE
            }),
            Value::Null => Some(facet_value::Value::NULL),
            Value::Priority(p) => Some(facet_value::VString::new(&p.to_string()).into_value()),
            Value::Identifier(id) => Some(facet_value::VString::new(id).into_value()),
        }
    }
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
