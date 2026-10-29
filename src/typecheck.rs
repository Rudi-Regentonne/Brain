use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    Void,
    String,
    Bool,
    Int,
    Unknown,
}

impl InferredType {
    pub fn from_ast_type(t: &Type) -> Self {
        match t {
            Type::Void => InferredType::Void,
            Type::String => InferredType::String,
            Type::Bool => InferredType::Bool,
            Type::Int => InferredType::Int,
            Type::Custom(_) => InferredType::Unknown,
        }
    }
}

pub struct SymbolTable {
    scopes: Vec<HashMap<String, InferredType>>,
    methods: HashMap<String, (InferredType, Vec<InferredType>)>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut st = Self {
            scopes: vec![HashMap::new()],
            methods: HashMap::new(),
        };

        // Built-in functions available in skills
        st.methods.insert(
            "Say".to_string(),
            (InferredType::Void, vec![InferredType::String]),
        );
        st.methods.insert(
            "Prompt".to_string(),
            (InferredType::Void, vec![InferredType::String]),
        );
        st.methods.insert(
            "Inform".to_string(),
            (InferredType::Void, vec![InferredType::String]),
        );
        st.methods.insert(
            "Execute".to_string(),
            (InferredType::Void, vec![InferredType::String]),
        );
        st.methods.insert(
            "RunCommand".to_string(),
            (InferredType::Void, vec![InferredType::String]),
        );
        st.methods.insert(
            "Ask".to_string(),
            (InferredType::String, vec![InferredType::String]),
        );

        st
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn insert_var(&mut self, name: String, var_type: InferredType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, var_type);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<&InferredType> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t);
            }
        }
        None
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TypeChecker {
    symbols: SymbolTable,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
        }
    }

    pub fn check_file(&mut self, file: &SourceFile) -> Result<(), String> {
        for class in &file.classes {
            self.check_class(class)?;
        }
        Ok(())
    }

    fn check_class(&mut self, class: &ClassDecl) -> Result<(), String> {
        // First pass: register all method signatures
        for method in &class.methods {
            let ret = InferredType::from_ast_type(&method.return_type);
            let params = method
                .params
                .iter()
                .map(|p| InferredType::from_ast_type(&p.param_type))
                .collect();
            self.symbols
                .methods
                .insert(method.name.clone(), (ret, params));
        }

        // Second pass: check method bodies
        for method in &class.methods {
            self.check_method(method)?;
        }

        Ok(())
    }

    fn check_method(&mut self, method: &MethodDecl) -> Result<(), String> {
        self.symbols.enter_scope();

        for param in &method.params {
            let t = InferredType::from_ast_type(&param.param_type);
            self.symbols.insert_var(param.name.clone(), t.clone());
            if let Some(ref def_val) = param.default_value {
                let def_type = self.check_expr(def_val)?;
                if t != InferredType::Unknown && def_type != InferredType::Unknown && t != def_type
                {
                    return Err(format!(
                        "Type mismatch for default value of parameter '{}' at {}: expected {:?}, got {:?}",
                        param.name, param.span, t, def_type
                    ));
                }
            }
        }

        for stmt in &method.body {
            self.check_statement(stmt, &InferredType::from_ast_type(&method.return_type))?;
        }

        self.symbols.exit_scope();
        Ok(())
    }

    fn check_statement(
        &mut self,
        stmt: &Statement,
        expected_ret: &InferredType,
    ) -> Result<(), String> {
        match stmt {
            Statement::VarDecl {
                var_type,
                name,
                init,
                span,
            } => {
                let declared_type = InferredType::from_ast_type(var_type);
                if let Some(init_expr) = init {
                    let init_type = self.check_expr(init_expr)?;
                    if declared_type != InferredType::Unknown
                        && init_type != InferredType::Unknown
                        && declared_type != init_type
                    {
                        return Err(format!(
                            "Type mismatch in variable declaration '{}' at {}: expected {:?}, got {:?}",
                            name, span, declared_type, init_type
                        ));
                    }
                }
                self.symbols.insert_var(name.clone(), declared_type);
            }
            Statement::Assignment {
                target,
                value,
                span,
            } => {
                let val_type = self.check_expr(value)?;
                if let Some(target_type) = self.symbols.lookup_var(target) {
                    if *target_type != InferredType::Unknown
                        && val_type != InferredType::Unknown
                        && *target_type != val_type
                    {
                        return Err(format!(
                            "Cannot assign {:?} to variable '{}' of type {:?} at {}",
                            val_type, target, target_type, span
                        ));
                    }
                } else {
                    return Err(format!("Undefined variable '{}' at {}", target, span));
                }
            }
            Statement::Expression(expr) => {
                self.check_expr(expr)?;
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                span: _span,
            } => {
                let _cond_type = self.check_expr(condition)?;
                // In prompt DSL, conditions can be boolean expressions (flag == true)
                // OR natural language string conditions ("banana is ripe", "user requests brief output")

                self.symbols.enter_scope();
                for s in then_branch {
                    self.check_statement(s, expected_ret)?;
                }
                self.symbols.exit_scope();

                if let Some(else_b) = else_branch {
                    self.symbols.enter_scope();
                    for s in else_b {
                        self.check_statement(s, expected_ret)?;
                    }
                    self.symbols.exit_scope();
                }
            }
            Statement::While {
                condition,
                body,
                span: _span,
            } => {
                let _cond_type = self.check_expr(condition)?;
                // While conditions can also be natural language strings ("there are remaining items")

                self.symbols.enter_scope();
                for s in body {
                    self.check_statement(s, expected_ret)?;
                }
                self.symbols.exit_scope();
            }
            Statement::TryCatch {
                try_body,
                catch_var,
                catch_body,
                ..
            } => {
                self.symbols.enter_scope();
                for s in try_body {
                    self.check_statement(s, expected_ret)?;
                }
                self.symbols.exit_scope();

                self.symbols.enter_scope();
                if let Some(c_var) = catch_var {
                    self.symbols
                        .insert_var(c_var.clone(), InferredType::Unknown);
                }
                for s in catch_body {
                    self.check_statement(s, expected_ret)?;
                }
                self.symbols.exit_scope();
            }
            Statement::Return { value, span } => {
                if let Some(ret_val) = value {
                    let actual_ret = self.check_expr(ret_val)?;
                    if *expected_ret != InferredType::Unknown
                        && actual_ret != InferredType::Unknown
                        && *expected_ret != actual_ret
                    {
                        return Err(format!(
                            "Return type mismatch at {}: expected {:?}, got {:?}",
                            span, expected_ret, actual_ret
                        ));
                    }
                } else if *expected_ret != InferredType::Void
                    && *expected_ret != InferredType::Unknown
                {
                    return Err(format!(
                        "Missing return value at {}: expected {:?}",
                        span, expected_ret
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn check_expr(&mut self, expr: &Expression) -> Result<InferredType, String> {
        match expr {
            Expression::StringLit(_, _) => Ok(InferredType::String),
            Expression::InterpolatedString(_, exprs, _) => {
                for e in exprs {
                    self.check_expr(e)?;
                }
                Ok(InferredType::String)
            }
            Expression::IntLit(_, _) => Ok(InferredType::Int),
            Expression::BoolLit(_, _) => Ok(InferredType::Bool),
            Expression::NullLit(_) => Ok(InferredType::Unknown),
            Expression::Identifier(name, _span) => {
                if let Some(t) = self.symbols.lookup_var(name) {
                    Ok(t.clone())
                } else {
                    // Check if it's a known method or class/module name
                    if self.symbols.methods.contains_key(name) {
                        Ok(InferredType::Unknown)
                    } else {
                        // In AI skill DSL, allow free-form environment identifiers, but give warning/Unknown
                        Ok(InferredType::Unknown)
                    }
                }
            }
            Expression::MemberAccess { object, member, .. } => {
                let _ = self.check_expr(object)?;
                // Member access like Git.IsRepo or string.IsEmpty
                if member == "IsEmpty"
                    || member == "IsNullOrEmpty"
                    || member.starts_with("Is")
                    || member.starts_with("Has")
                {
                    Ok(InferredType::Bool)
                } else {
                    Ok(InferredType::Unknown)
                }
            }
            Expression::Call {
                callee,
                args,
                span: _,
            } => {
                for arg in args {
                    self.check_expr(arg)?;
                }

                if let Expression::Identifier(ref name, _) = **callee
                    && let Some((ret, _)) = self.symbols.methods.get(name)
                {
                    return Ok(ret.clone());
                }

                if let Expression::MemberAccess { ref member, .. } = **callee
                    && (member.starts_with("Is")
                        || member.starts_with("Has")
                        || member == "IsEmpty")
                {
                    return Ok(InferredType::Bool);
                }

                Ok(InferredType::Unknown)
            }
            Expression::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left_t = self.check_expr(left)?;
                let right_t = self.check_expr(right)?;

                match op {
                    BinaryOp::Eq | BinaryOp::NotEq => {
                        if left_t != InferredType::Unknown
                            && right_t != InferredType::Unknown
                            && left_t != right_t
                        {
                            return Err(format!(
                                "Cannot compare different types {:?} and {:?} at {}",
                                left_t, right_t, span
                            ));
                        }
                        Ok(InferredType::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if (left_t != InferredType::Bool && left_t != InferredType::Unknown)
                            || (right_t != InferredType::Bool && right_t != InferredType::Unknown)
                        {
                            return Err(format!(
                                "Logical operator expects booleans at {}, got {:?} and {:?}",
                                span, left_t, right_t
                            ));
                        }
                        Ok(InferredType::Bool)
                    }
                    BinaryOp::Add => {
                        if left_t == InferredType::String || right_t == InferredType::String {
                            Ok(InferredType::String)
                        } else if left_t == InferredType::Int && right_t == InferredType::Int {
                            Ok(InferredType::Int)
                        } else {
                            Ok(InferredType::Unknown)
                        }
                    }
                    BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                        if (left_t != InferredType::Int && left_t != InferredType::Unknown)
                            || (right_t != InferredType::Int && right_t != InferredType::Unknown)
                        {
                            return Err(format!(
                                "Arithmetic operator expects integers at {}, got {:?} and {:?}",
                                span, left_t, right_t
                            ));
                        }
                        Ok(InferredType::Int)
                    }
                }
            }
            Expression::Unary { op, expr, span } => {
                let t = self.check_expr(expr)?;
                match op {
                    UnaryOp::Not => {
                        if t != InferredType::Bool && t != InferredType::Unknown {
                            return Err(format!(
                                "'!' operator expects bool at {}, got {:?}",
                                span, t
                            ));
                        }
                        Ok(InferredType::Bool)
                    }
                    UnaryOp::Neg => {
                        if t != InferredType::Int && t != InferredType::Unknown {
                            return Err(format!(
                                "'-' operator expects integer at {}, got {:?}",
                                span, t
                            ));
                        }
                        Ok(InferredType::Int)
                    }
                }
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
