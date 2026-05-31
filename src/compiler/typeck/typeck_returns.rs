use crate::parser::ast::{Expression, Statement};

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

pub(super) fn implicit_return_expression_mut(
    statements: &mut [Statement],
) -> Option<&mut Expression> {
    match statements.last_mut()? {
        Statement::Expression(expr) => Some(expr),
        _ => None,
    }
}
