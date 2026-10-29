use crate::ast::*;
use crate::config::TargetPlatform;

pub struct Emitter {
    target: TargetPlatform,
}

impl Emitter {
    pub fn new(target: TargetPlatform) -> Self {
        Self { target }
    }

    pub fn emit_file(&self, file: &SourceFile) -> Result<Vec<(String, String)>, String> {
        let mut results = Vec::new();
        // Separate primary skill classes (annotated with [Skill]) from imported/helper classes
        let skill_classes: Vec<&ClassDecl> = file
            .classes
            .iter()
            .filter(|c| c.attributes.iter().any(|a| a.name == "Skill"))
            .collect();

        // Extra helper classes to merge into the skill
        let helper_classes: Vec<&ClassDecl> = file
            .classes
            .iter()
            .filter(|c| !c.attributes.iter().any(|a| a.name == "Skill"))
            .collect();

        if skill_classes.is_empty() {
            // Helper-only file (no [Skill] attribute) -> this is an imported library/module, not a standalone skill!
            return Ok(Vec::new());
        } else {
            for skill_class in skill_classes {
                let (skill_name, markdown_content) =
                    self.emit_class(skill_class, &helper_classes)?;
                results.push((skill_name, markdown_content));
            }
        }

        Ok(results)
    }

    fn emit_class(
        &self,
        class: &ClassDecl,
        helpers: &[&ClassDecl],
    ) -> Result<(String, String), String> {
        let mut name = class.name.to_lowercase();
        let mut description = format!("Skill {} implementation", class.name);
        let mut triggers = Vec::new();
        let mut tools = Vec::new();
        let mut rules = Vec::new();
        let mut guardrails = Vec::new();
        let mut examples = Vec::new();

        // Process class attributes
        for attr in &class.attributes {
            match attr.name.as_str() {
                "Skill" => {
                    let mut positional_idx = 0;
                    for arg in &attr.args {
                        if let Some(ref arg_name) = arg.name {
                            if arg_name == "Name" {
                                if let Expression::StringLit(s, _) = &arg.value {
                                    name = s.clone();
                                }
                            } else if arg_name == "Description"
                                && let Expression::StringLit(s, _) = &arg.value
                            {
                                description = s.clone();
                            }
                        } else {
                            // Positional arguments: first is name, second is description
                            if let Expression::StringLit(s, _) = &arg.value {
                                if positional_idx == 0 {
                                    name = s.clone();
                                } else if positional_idx == 1 {
                                    description = s.clone();
                                }
                                positional_idx += 1;
                            }
                        }
                    }
                }
                "Trigger" => {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            triggers.push(s.clone());
                        }
                    }
                }
                "Requires" | "RequiresTool" => {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            tools.push(s.clone());
                        }
                    }
                }
                "Rule" => {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            rules.push(s.clone());
                        }
                    }
                }
                "Guardrail" | "Never" | "Constraint" => {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            guardrails.push(s.clone());
                        }
                    }
                }
                "Example" => {
                    let mut in_val = String::new();
                    let mut out_val = String::new();
                    let mut good_val = String::new();
                    let mut bad_val = String::new();
                    for arg in &attr.args {
                        if let Some(ref arg_name) = arg.name {
                            match arg_name.as_str() {
                                "Input" => {
                                    if let Expression::StringLit(s, _) = &arg.value {
                                        in_val = s.clone();
                                    }
                                }
                                "Output" => {
                                    if let Expression::StringLit(s, _) = &arg.value {
                                        out_val = s.clone();
                                    }
                                }
                                "Good" => {
                                    if let Expression::StringLit(s, _) = &arg.value {
                                        good_val = s.clone();
                                    }
                                }
                                "Bad" => {
                                    if let Expression::StringLit(s, _) = &arg.value {
                                        bad_val = s.clone();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    if !in_val.is_empty() {
                        examples.push((in_val, out_val, good_val, bad_val));
                    }
                }
                _ => {}
            }
        }

        // Include rules from helper/imported classes
        for helper in helpers {
            for attr in &helper.attributes {
                if attr.name == "Rule" {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            rules.push(s.clone());
                        }
                    }
                } else if attr.name == "Guardrail"
                    || attr.name == "Never"
                    || attr.name == "Constraint"
                {
                    for arg in &attr.args {
                        if let Expression::StringLit(s, _) = &arg.value {
                            guardrails.push(s.clone());
                        }
                    }
                }
            }
        }

        let mut body_md = String::new();

        // Main skill title
        body_md.push_str(&format!("# {}\n\n", title_case(&name)));
        body_md.push_str(&format!("{}\n\n", description));

        // Hard Guardrails (CRITICAL boundaries)
        if !guardrails.is_empty() {
            body_md.push_str("<guardrails>\n");
            body_md.push_str("CRITICAL:\n");
            for g in &guardrails {
                body_md.push_str(&format!(
                    "- DO NOT {}\n",
                    g.trim_start_matches("DO NOT ")
                        .trim_start_matches("Never ")
                        .trim_start_matches("never ")
                ));
            }
            body_md.push_str("</guardrails>\n\n");
        }

        // Rules & Guidelines section
        if !rules.is_empty() {
            body_md.push_str("<rules>\n");
            for r in &rules {
                body_md.push_str(&format!("- {}\n", r));
            }
            body_md.push_str("</rules>\n\n");
        }

        // Prerequisites / Preconditions
        let mut preconditions = Vec::new();

        // Class-level [Precondition("...")] attributes
        for attr in &class.attributes {
            if attr.name == "Precondition"
                && let Some(arg) = attr.args.first()
                && let Expression::StringLit(s, _) = &arg.value
            {
                preconditions.push(s.clone());
            }
        }

        // Method-level preconditions: inspect method body if no string literal was given!
        for method in &class.methods {
            for attr in &method.attributes {
                if attr.name == "Precondition" {
                    if let Some(arg) = attr.args.first()
                        && let Expression::StringLit(s, _) = &arg.value
                    {
                        preconditions.push(s.clone());
                        continue;
                    }

                    // Decompile the method body into plain English
                    let mut precond_summary = Vec::new();
                    self.extract_preconditions_from_stmts(&method.body, &mut precond_summary);

                    if precond_summary.is_empty() {
                        preconditions.push(format!("Check prerequisites via {}", method.name));
                    } else {
                        preconditions.extend(precond_summary);
                    }
                }
            }
        }
        if !preconditions.is_empty() {
            body_md.push_str("<prerequisites>\n");
            for p in &preconditions {
                body_md.push_str(&format!("- {}\n", p));
            }
            body_md.push_str("</prerequisites>\n\n");
        }

        // Methods / Execution workflows
        // Find main execution method: Run or Execute, or first public method
        let mut main_method = None;
        for method in &class.methods {
            if method.name.to_lowercase() == "examples" {
                // Collect examples from attribute
                for attr in &method.attributes {
                    if attr.name == "Example" {
                        let mut in_val = String::new();
                        let mut out_val = String::new();
                        let mut good_val = String::new();
                        let mut bad_val = String::new();
                        for arg in &attr.args {
                            if let Some(ref arg_name) = arg.name {
                                match arg_name.as_str() {
                                    "Input" => {
                                        if let Expression::StringLit(s, _) = &arg.value {
                                            in_val = s.clone();
                                        }
                                    }
                                    "Output" => {
                                        if let Expression::StringLit(s, _) = &arg.value {
                                            out_val = s.clone();
                                        }
                                    }
                                    "Good" => {
                                        if let Expression::StringLit(s, _) = &arg.value {
                                            good_val = s.clone();
                                        }
                                    }
                                    "Bad" => {
                                        if let Expression::StringLit(s, _) = &arg.value {
                                            bad_val = s.clone();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if !in_val.is_empty() {
                            examples.push((in_val, out_val, good_val, bad_val));
                        }
                    }
                }
                continue;
            }

            if method.name == "Run" || method.name == "Execute" {
                main_method = Some(method);
                break;
            } else if method.is_public && main_method.is_none() {
                // Avoid using precondition helper as main method
                let is_precond = method.attributes.iter().any(|a| a.name == "Precondition");
                if !is_precond {
                    main_method = Some(method);
                }
            }
        }

        if let Some(main) = main_method {
            // Parameters: clear bullet points inside <parameters>
            if !main.params.is_empty() {
                body_md.push_str("<parameters>\n");
                for param in &main.params {
                    let def_str = if let Some(ref d) = param.default_value {
                        format!(" (default: `{}`)", self.expr_to_str(d))
                    } else {
                        " (required)".to_string()
                    };
                    let type_str = match &param.param_type {
                        Type::Void => "void",
                        Type::String => "string",
                        Type::Bool => "bool",
                        Type::Int => "int",
                        Type::Custom(c) => c.as_str(),
                    };
                    body_md.push_str(&format!(
                        "- `{}` (`{}`){}: Parameter `{}`\n",
                        param.name, type_str, def_str, param.name
                    ));
                }
                body_md.push_str("</parameters>\n\n");
            }

            // Main workflow steps
            body_md.push_str(&format!("<instructions workflow=\"{}\">\n", main.name));
            let mut step_num = 1;
            for stmt in &main.body {
                self.emit_statement(stmt, &mut body_md, &mut step_num, 0);
            }
            body_md.push_str("</instructions>\n\n");
        }

        // Subroutines / Helper methods (including imported helper classes)
        for method in &class.methods {
            if Some(method) == main_method
                || method.name.to_lowercase() == "examples"
                || method.attributes.iter().any(|a| a.name == "Precondition")
            {
                continue;
            }

            body_md.push_str(&format!("<procedure name=\"{}\">\n", method.name));
            let mut step_num = 1;
            for stmt in &method.body {
                self.emit_statement(stmt, &mut body_md, &mut step_num, 0);
            }
            body_md.push_str("</procedure>\n\n");
        }

        for helper in helpers {
            for method in &helper.methods {
                if method.name.to_lowercase() == "examples"
                    || method.attributes.iter().any(|a| a.name == "Precondition")
                {
                    continue;
                }

                body_md.push_str(&format!("<procedure name=\"{}\">\n", method.name));
                let mut step_num = 1;
                for stmt in &method.body {
                    self.emit_statement(stmt, &mut body_md, &mut step_num, 0);
                }
                body_md.push_str("</procedure>\n\n");
            }
        }

        // Examples section with contrastive pairs support
        if !examples.is_empty() {
            body_md.push_str("<examples>\n");
            for (inp, out, good, bad) in &examples {
                body_md.push_str(&format!("- Input: {}\n", inp));
                if !bad.is_empty() {
                    body_md.push_str(&format!("  ❌ BAD: {}\n", bad));
                }
                if !good.is_empty() {
                    body_md.push_str(&format!("  ✅ GOOD: {}\n", good));
                }
                if !out.is_empty() {
                    body_md.push_str(&format!("  Output: `{}`\n", out));
                }
            }
            body_md.push_str("</examples>\n\n");
        }

        // Generate Frontmatter
        let frontmatter = self.generate_frontmatter(&name, &description, &triggers, &tools);

        // Combine into full markdown
        let final_markdown = format!("{}\n{}", frontmatter, body_md);

        Ok((name, final_markdown))
    }

    fn extract_preconditions_from_stmts(&self, stmts: &[Statement], out: &mut Vec<String>) {
        self.extract_preconditions_recursive(stmts, out, &[]);
    }

    fn extract_preconditions_recursive(
        &self,
        stmts: &[Statement],
        out: &mut Vec<String>,
        parent_conditions: &[String],
    ) {
        for stmt in stmts {
            match stmt {
                Statement::If {
                    condition,
                    then_branch,
                    ..
                } => {
                    let mut current_conditions = parent_conditions.to_vec();
                    current_conditions.push(self.expr_to_condition_str(condition));

                    // Check if then_branch directly returns true
                    let returns_true = then_branch.iter().any(|s| {
                        matches!(
                            s,
                            Statement::Return {
                                value: Some(Expression::BoolLit(true, _)),
                                ..
                            }
                        )
                    });

                    if returns_true {
                        let combined = current_conditions.join(" and ");
                        out.push(format!("Requires that {}", combined));
                    } else {
                        // Recurse into nested ifs preserving parent conditions
                        self.extract_preconditions_recursive(then_branch, out, &current_conditions);
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => match expr {
                    Expression::BoolLit(true, _) => {
                        if !parent_conditions.is_empty() {
                            out.push(format!("Requires that {}", parent_conditions.join(" and ")));
                        }
                    }
                    Expression::BoolLit(false, _) => {}
                    _ => {
                        let cond_str = self.expr_to_condition_str(expr);
                        if !parent_conditions.is_empty() {
                            out.push(format!(
                                "Requires that {} and {}",
                                parent_conditions.join(" and "),
                                cond_str
                            ));
                        } else {
                            out.push(format!("Requires that {}", cond_str));
                        }
                    }
                },
                Statement::While {
                    condition, body, ..
                } => {
                    out.push(format!(
                        "Repeat while {}:",
                        self.expr_to_condition_str(condition)
                    ));
                    for b in body {
                        if let Statement::Expression(Expression::Call { callee, args, .. }) = b {
                            let fn_name = self.expr_to_str(callee);
                            if (fn_name == "Say"
                                || fn_name == "Prompt"
                                || fn_name == "Promt"
                                || fn_name == "Inform")
                                && !args.is_empty()
                            {
                                out.push(format!(
                                    "  - {}",
                                    self.expr_to_str(&args[0]).trim_matches('"')
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn generate_frontmatter(
        &self,
        name: &str,
        description: &str,
        triggers: &[String],
        tools: &[String],
    ) -> String {
        let mut out = String::from("---\n");
        out.push_str(&format!("name: {}\n", name));
        out.push_str(&format!(
            "description: \"{}\"\n",
            description.replace('"', "\\\"")
        ));

        if !triggers.is_empty() {
            out.push_str("triggers:\n");
            for t in triggers {
                out.push_str(&format!("  - \"{}\"\n", t));
            }
        }

        if !tools.is_empty() {
            out.push_str("tools:\n");
            for tool in tools {
                out.push_str(&format!("  - \"{}\"\n", tool));
            }
        }

        // Claude specific metadata if Claude target
        if self.target == TargetPlatform::Claude {
            out.push_str("platform: claude\n");
        } else {
            out.push_str("platform: opencode\n");
        }

        out.push_str("---\n");
        out
    }

    fn emit_statement(
        &self,
        stmt: &Statement,
        out: &mut String,
        step_num: &mut usize,
        indent: usize,
    ) {
        let ind_str = "  ".repeat(indent);

        match stmt {
            Statement::VarDecl { name, init, .. } => {
                let init_str = if let Some(init_expr) = init {
                    format!("initialized to {}", self.expr_to_str(init_expr))
                } else {
                    "uninitialized".to_string()
                };
                if indent == 0 {
                    out.push_str(&format!("{}. Let `{}` be {}.\n", step_num, name, init_str));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- Let `{}` be {}.\n", ind_str, name, init_str));
                }
            }
            Statement::Assignment { target, value, .. } => {
                let val_str = self.expr_to_str(value);
                if indent == 0 {
                    out.push_str(&format!("{}. Set `{}` to {}.\n", step_num, target, val_str));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- Set `{}` to {}.\n", ind_str, target, val_str));
                }
            }
            Statement::Expression(expr) => {
                let text = match expr {
                    Expression::Call { callee, args, .. } => {
                        let fn_name = self.expr_to_str(callee);
                        if (fn_name == "Say" || fn_name == "Prompt" || fn_name == "Inform")
                            && !args.is_empty()
                        {
                            format!("Output: {}", self.expr_to_str(&args[0]))
                        } else if (fn_name == "Execute" || fn_name == "RunCommand")
                            && !args.is_empty()
                        {
                            let cmd = self.expr_to_str(&args[0]);
                            format!("Run command in shell: `{}`", cmd.trim_matches('"'))
                        } else {
                            format!("Execute `{}`", self.expr_to_str(expr))
                        }
                    }
                    _ => format!("Evaluate `{}`", self.expr_to_str(expr)),
                };

                if indent == 0 {
                    out.push_str(&format!("{}. {}\n", step_num, text));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- {}\n", ind_str, text));
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_str = self.expr_to_condition_str(condition);
                if indent == 0 {
                    out.push_str(&format!("{}. **WHEN** {}:\n", step_num, cond_str));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- **WHEN** {}:\n", ind_str, cond_str));
                }

                for s in then_branch {
                    let mut dummy = 1;
                    self.emit_statement(s, out, &mut dummy, indent + 1);
                }

                if let Some(else_b) = else_branch {
                    out.push_str(&format!("{}  **OTHERWISE**:\n", ind_str));
                    for s in else_b {
                        let mut dummy = 1;
                        self.emit_statement(s, out, &mut dummy, indent + 1);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                let cond_str = self.expr_to_condition_str(condition);
                if indent == 0 {
                    out.push_str(&format!("{}. **Repeat while** {}:\n", step_num, cond_str));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- **Repeat while** {}:\n", ind_str, cond_str));
                }

                for s in body {
                    let mut dummy = 1;
                    self.emit_statement(s, out, &mut dummy, indent + 1);
                }
            }
            Statement::TryCatch {
                try_body,
                catch_exception,
                catch_body,
                ..
            } => {
                if indent == 0 {
                    out.push_str(&format!("{}. Perform primary action:\n", step_num));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- Perform primary action:\n", ind_str));
                }

                for s in try_body {
                    let mut dummy = 1;
                    self.emit_statement(s, out, &mut dummy, indent + 1);
                }

                let exc_name = catch_exception.as_deref().unwrap_or("failure");
                out.push_str(&format!("{}  **On {} fallback**:\n", ind_str, exc_name));
                for s in catch_body {
                    let mut dummy = 1;
                    self.emit_statement(s, out, &mut dummy, indent + 1);
                }
            }
            Statement::Return { value, .. } => {
                let ret_str = if let Some(v) = value {
                    format!("with {}", self.expr_to_str(v))
                } else {
                    "successfully".to_string()
                };
                if indent == 0 {
                    out.push_str(&format!("{}. Return {}.\n", step_num, ret_str));
                    *step_num += 1;
                } else {
                    out.push_str(&format!("{}- Return {}.\n", ind_str, ret_str));
                }
            }
        }
    }

    fn expr_to_condition_str(&self, expr: &Expression) -> String {
        match expr {
            Expression::StringLit(s, _) => s.clone(),
            _ => self.expr_to_str(expr),
        }
    }

    fn expr_to_str(&self, expr: &Expression) -> String {
        match expr {
            Expression::StringLit(s, _) => format!("\"{}\"", s),
            Expression::InterpolatedString(tmpl, exprs, _) => {
                let mut res = tmpl.clone();
                for e in exprs {
                    let placeholder = "{}";
                    let repl = self.expr_to_str(e);
                    if let Some(pos) = res.find(placeholder) {
                        res.replace_range(
                            pos..pos + placeholder.len(),
                            &format!("{{{}}}", repl.trim_matches('"')),
                        );
                    }
                }
                format!("\"{}\"", res)
            }
            Expression::IntLit(i, _) => i.to_string(),
            Expression::BoolLit(b, _) => b.to_string(),
            Expression::NullLit(_) => "null".to_string(),
            Expression::Identifier(name, _) => name.clone(),
            Expression::MemberAccess { object, member, .. } => {
                format!("{}.{}", self.expr_to_str(object), member)
            }
            Expression::Call { callee, args, .. } => {
                let args_str = args
                    .iter()
                    .map(|a| self.expr_to_str(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", self.expr_to_str(callee), args_str)
            }
            Expression::Binary {
                op, left, right, ..
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Eq => "==",
                    BinaryOp::NotEq => "!=",
                    BinaryOp::And => "and",
                    BinaryOp::Or => "or",
                };
                format!(
                    "{} {} {}",
                    self.expr_to_str(left),
                    op_str,
                    self.expr_to_str(right)
                )
            }
            Expression::Unary { op, expr, .. } => {
                let op_str = match op {
                    UnaryOp::Not => "not ",
                    UnaryOp::Neg => "-",
                };
                format!("{}{}", op_str, self.expr_to_str(expr))
            }
        }
    }
}

fn title_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
