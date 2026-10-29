use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    String,
    Bool,
    Int,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeArg {
    pub name: Option<String>,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttributeArg>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub param_type: Type,
    pub name: String,
    pub default_value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub attributes: Vec<Attribute>,
    pub is_public: bool,
    pub return_type: Type,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub methods: Vec<MethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub usings: Vec<String>,
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VarDecl {
        var_type: Type,
        name: String,
        init: Option<Expression>,
        span: Span,
    },
    Assignment {
        target: String,
        value: Expression,
        span: Span,
    },
    Expression(Expression),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
        span: Span,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Statement>,
        catch_exception: Option<String>,
        catch_var: Option<String>,
        catch_body: Vec<Statement>,
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    StringLit(String, Span),
    InterpolatedString(String, Vec<Expression>, Span),
    IntLit(i64, Span),
    BoolLit(bool, Span),
    NullLit(Span),
    Identifier(String, Span),
    MemberAccess {
        object: Box<Expression>,
        member: String,
        span: Span,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
        span: Span,
    },
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::StringLit(_, s) => s.clone(),
            Expression::InterpolatedString(_, _, s) => s.clone(),
            Expression::IntLit(_, s) => s.clone(),
            Expression::BoolLit(_, s) => s.clone(),
            Expression::NullLit(s) => s.clone(),
            Expression::Identifier(_, s) => s.clone(),
            Expression::MemberAccess { span, .. } => span.clone(),
            Expression::Call { span, .. } => span.clone(),
            Expression::Binary { span, .. } => span.clone(),
            Expression::Unary { span, .. } => span.clone(),
        }
    }
}
