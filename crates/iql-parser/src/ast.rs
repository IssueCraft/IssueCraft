use std::{
    fmt::{self, Display},
    ops::Deref,
};

use facet::{Facet, Type};
use facet_value::Value as FacetValue;

use crate::IqlError;

#[derive(Debug, Clone, PartialEq)]
pub enum IqlQuery {
    Create(CreateStatement),
    Select(SelectStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Assign(AssignStatement),
    Close(CloseStatement),
    Reopen(ReopenStatement),
    Comment(CommentStatement),
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct UserId(String);

impl UserId {
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Into<FacetValue> for UserId {
    fn into(self) -> FacetValue {
        facet_value::VString::new(&self.0).into_value()
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for UserId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for ProjectId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct IssueId(String);

impl IssueId {
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Deref for IssueId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub struct CommentId(String);

impl CommentId {
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Deref for CommentId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
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
        project_id: ProjectId,
        name: Option<String>,
        description: Option<String>,
        owner: Option<UserId>,
    },
    Issue {
        project: ProjectId,
        title: String,
        kind: IssueKind,
        description: Option<String>,
        priority: Option<Priority>,
        assignee: Option<UserId>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub columns: Columns,
    pub from: EntityType,
    pub filter: Option<FilterExpression>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Columns {
    All,
    Named(Vec<String>),
}

impl Columns {
    #[must_use]
    pub fn count(&self) -> usize {
        match self {
            Columns::All => usize::MAX,
            Columns::Named(cols) => cols.len(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EntityType {
    Users,
    Projects,
    Issues,
    Comments,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Users => write!(f, "USERS"),
            EntityType::Projects => write!(f, "PROJECTS"),
            EntityType::Issues => write!(f, "ISSUES"),
            EntityType::Comments => write!(f, "COMMENTS"),
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
    #[must_use]
    pub fn matches(&self, id: &str, value: &FacetValue) -> bool {
        match self {
            FilterExpression::Comparison {
                field,
                op,
                value: filter_value,
            } => {
                let Some(obj) = value.as_object() else {
                    return false;
                };

                if field == "id" {
                    let id_value = facet_value::VString::new(id).into_value();
                    return Self::compare_values(&id_value, op, filter_value);
                }

                let Some(field_value) = obj.get(field) else {
                    return false;
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
                let Some(obj) = value.as_object() else {
                    return false;
                };

                let Some(field_value) = obj.get(field) else {
                    return false;
                };

                values.iter().any(|filter_val| {
                    Self::compare_values(field_value, &ComparisonOp::Equal, filter_val)
                })
            }
            FilterExpression::IsNull(field) => {
                let Some(obj) = value.as_object() else {
                    return false;
                };

                match obj.get(field) {
                    None => true,
                    Some(v) => v.is_null(),
                }
            }
            FilterExpression::IsNotNull(field) => {
                let Some(obj) = value.as_object() else {
                    return false;
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
            ComparisonOp::Equal => field_value == &filter_value.to_facet(),
            ComparisonOp::NotEqual => field_value != &filter_value.to_facet(),
            ComparisonOp::GreaterThan => {
                field_value.partial_cmp(&filter_value.to_facet())
                    == Some(std::cmp::Ordering::Greater)
            }
            ComparisonOp::LessThan => {
                field_value.partial_cmp(&filter_value.to_facet()) == Some(std::cmp::Ordering::Less)
            }
            ComparisonOp::GreaterThanOrEqual => {
                matches!(
                    field_value.partial_cmp(&filter_value.to_facet()),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                )
            }
            ComparisonOp::LessThanOrEqual => {
                matches!(
                    field_value.partial_cmp(&filter_value.to_facet()),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            }
            ComparisonOp::Like => {
                let field_str = field_value
                    .as_string()
                    .map(facet_value::VString::as_str)
                    .unwrap_or_default();
                if let IqlValue::String(pattern) = filter_value {
                    let pattern = pattern.replace('%', ".*");
                    if let Ok(regex) = regex::Regex::new(&format!("^{pattern}$")) {
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

impl FieldUpdate {
    pub fn apply_to<'a, S: Facet<'a>>(&self, value: &mut FacetValue) -> Result<(), IqlError> {
        let o = value.as_object_mut().unwrap();
        if let Type::User(facet::UserType::Struct(s)) = S::SHAPE.ty {
            if !s.fields.iter().any(|f| f.name == self.field) {
                return Err(IqlError::FieldNotFound(self.field.clone()));
            }
        } else {
            panic!("Not a struct type");
        }
        o.insert(&self.field, self.value.to_facet());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub entity: DeleteTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteTarget {
    User(UserId),
    Project(ProjectId),
    Issue(IssueId),
    Comment(CommentId),
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
pub enum IssueKind {
    Epic,
    Improvement,
    Bug,
    Task,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStatement {
    pub issue_id: IssueId,
    pub assignee: UserId,
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
    Integer(i64),
    UnsignedInteger(u64),
    Float(f64),
    Boolean(bool),
    Null,
    Priority(Priority),
    Identifier(String),
}

impl IqlValue {
    fn to_facet(&self) -> FacetValue {
        match self {
            IqlValue::String(s) => facet_value::VString::new(s).into_value(),
            IqlValue::Integer(n) => facet_value::VNumber::from_i64(*n).into_value(),
            IqlValue::UnsignedInteger(n) => facet_value::VNumber::from_u64(*n).into_value(),
            IqlValue::Float(f) => facet_value::VNumber::from_f64(*f)
                .expect("Invalid float value")
                .into_value(),
            IqlValue::Boolean(b) => {
                if *b {
                    facet_value::Value::TRUE
                } else {
                    facet_value::Value::FALSE
                }
            }
            IqlValue::Null => facet_value::Value::NULL,
            IqlValue::Priority(p) => facet_value::VString::new(&p.to_string()).into_value(),
            IqlValue::Identifier(id) => facet_value::VString::new(id).into_value(),
        }
    }
}

impl fmt::Display for IqlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IqlValue::String(s) => write!(f, "'{s}'"),
            IqlValue::Integer(n) => write!(f, "{n}"),
            IqlValue::UnsignedInteger(n) => write!(f, "{n}"),
            IqlValue::Float(fl) => write!(f, "{fl}"),
            IqlValue::Boolean(b) => write!(f, "{b}"),
            IqlValue::Null => write!(f, "NULL"),
            IqlValue::Priority(p) => write!(f, "{p}"),
            IqlValue::Identifier(id) => write!(f, "{id}"),
        }
    }
}
