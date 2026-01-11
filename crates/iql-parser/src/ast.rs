use std::fmt;

use facet::Facet;
use facet_value::Value as FacetValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Create(CreateStatement),
    Select(SelectStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Assign(AssignStatement),
    Close(CloseStatement),
    Reopen(ReopenStatement),
    Comment(CommentStatement),
}

pub trait IdHelper {
    fn id_from_str(val: &str) -> Self;
    fn str_from_id(&self) -> &str;
}

impl IdHelper for String {
    fn id_from_str(val: &str) -> Self {
        val.to_string()
    }

    fn str_from_id(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct UserId(pub String);

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct IssueId(pub String);

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct CommentId(pub String);

impl IdHelper for ProjectId {
    fn id_from_str(val: &str) -> Self {
        ProjectId(val.to_string())
    }

    fn str_from_id(&self) -> &str {
        &self.0
    }
}

impl IdHelper for IssueId {
    fn id_from_str(val: &str) -> Self {
        IssueId(val.to_string())
    }

    fn str_from_id(&self) -> &str {
        &self.0
    }
}

impl IdHelper for CommentId {
    fn id_from_str(val: &str) -> Self {
        CommentId(val.to_string())
    }

    fn str_from_id(&self) -> &str {
        &self.0
    }
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
        assignee: Option<UserId>,
        labels: Vec<String>,
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
        value: IqlValue,
    },
    And(Box<FilterExpression>, Box<FilterExpression>),
    Or(Box<FilterExpression>, Box<FilterExpression>),
    Not(Box<FilterExpression>),
    In {
        field: String,
        values: Vec<IqlValue>,
    },
    IsNull(String),
    IsNotNull(String),
}

impl FilterExpression {
    pub fn matches(&self, id: &str, value: &FacetValue) -> bool {
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

                if field == "id" {
                    let id_value = facet_value::VString::new(id).into_value();
                    return Self::compare_values(&id_value, op, filter_value);
                }

                let field_value = match obj.get(field) {
                    Some(v) => v,
                    None => return false,
                };

                Self::compare_values(field_value, op, filter_value)
            }
            FilterExpression::And(left, right) => {
                left.matches(id, value) && right.matches(id, value)
            }
            FilterExpression::Or(left, right) => {
                left.matches(id, value) || right.matches(id, value)
            }
            FilterExpression::Not(expr) => !expr.matches(id, value),
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

    fn compare_values(
        field_value: &FacetValue,
        op: &ComparisonOp,
        filter_value: &IqlValue,
    ) -> bool {
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
                if let IqlValue::String(pattern) = filter_value {
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

    fn convert_iql_value_to_facet(iql_value: &IqlValue) -> Option<FacetValue> {
        match iql_value {
            IqlValue::String(s) => Some(facet_value::VString::new(s).into_value()),
            IqlValue::Number(n) => Some(facet_value::VNumber::from_u64(*n as u64).into_value()),
            IqlValue::Float(f) => Some(facet_value::VNumber::from_f64(*f as f64)?.into_value()),
            IqlValue::Boolean(b) => Some(if *b {
                facet_value::Value::TRUE
            } else {
                facet_value::Value::FALSE
            }),
            IqlValue::Null => Some(facet_value::Value::NULL),
            IqlValue::Priority(p) => Some(facet_value::VString::new(&p.to_string()).into_value()),
            IqlValue::Identifier(id) => Some(facet_value::VString::new(id).into_value()),
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
    User(UserId),
    Project(ProjectId),
    Issue(IssueId),
    Comment(CommentId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldUpdate {
    pub field: String,
    pub value: IqlValue,
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

#[derive(Debug, Clone, PartialEq, Facet, Default)]
#[repr(C)]
pub enum CloseReason {
    #[default]
    Done,
    Duplicate,
    WontFix,
}

impl fmt::Display for CloseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloseReason::Done => write!(f, "DONE"),
            CloseReason::Duplicate => write!(f, "DUPLICATE"),
            CloseReason::WontFix => write!(f, "WONTFIX"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloseStatement {
    pub issue_id: IssueId,
    pub reason: Option<CloseReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReopenStatement {
    pub issue_id: IssueId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentStatement {
    pub issue_id: IssueId,
    pub content: String,
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
            Priority::Critical => write!(f, "CRITICAL"),
            Priority::High => write!(f, "HIGH"),
            Priority::Medium => write!(f, "MEDIUM"),
            Priority::Low => write!(f, "LOW"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IqlValue {
    String(String),
    Number(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Priority(Priority),
    Identifier(String),
}

impl fmt::Display for IqlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IqlValue::String(s) => write!(f, "'{}'", s),
            IqlValue::Number(n) => write!(f, "{}", n),
            IqlValue::Float(fl) => write!(f, "{}", fl),
            IqlValue::Boolean(b) => write!(f, "{}", b),
            IqlValue::Null => write!(f, "NULL"),
            IqlValue::Priority(p) => write!(f, "{}", p),
            IqlValue::Identifier(id) => write!(f, "{}", id),
        }
    }
}
