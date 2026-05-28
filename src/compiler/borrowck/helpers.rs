use super::*;

pub(super) fn assignment_binding_target(target: &AssignmentTarget) -> String {
    target
        .binding_name()
        .map(ToString::to_string)
        .unwrap_or_else(|| target.to_string())
}

pub(super) fn statements_contain_explicit_return(statements: &[Statement]) -> bool {
    statements.iter().any(statement_contains_explicit_return)
}

fn statement_contains_explicit_return(statement: &Statement) -> bool {
    match statement {
        Statement::Return(_) => true,
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            statements_contain_explicit_return(then_branch)
                || else_branch
                    .as_ref()
                    .is_some_and(|branch| statements_contain_explicit_return(branch))
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Find { body, .. }
        | Statement::Unsafe { body }
        | Statement::Module { body, .. } => statements_contain_explicit_return(body),
        Statement::Match { cases, default, .. } => {
            cases
                .iter()
                .any(|(_, body)| statements_contain_explicit_return(body))
                || statements_contain_explicit_return(default)
        }
        Statement::Function { body, .. }
        | Statement::Type { fields: body, .. }
        | Statement::Impl { methods: body, .. } => statements_contain_explicit_return(body),
        _ => false,
    }
}

pub(super) fn implicit_return_expression(statements: &[Statement]) -> Option<&Expression> {
    match statements.last()? {
        Statement::Expression(expr) => Some(expr),
        _ => None,
    }
}
