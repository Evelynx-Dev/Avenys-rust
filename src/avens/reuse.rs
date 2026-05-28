use super::*;
use crate::incremental::{
    AnalysisInvalidationReport, CachedAnalysisSnapshot, analysis_child_unit_key, analysis_unit_key,
    analysis_units_for_program, compute_invalidation_report,
};

pub(super) fn prepare_program_with_partial_analysis_reuse(
    current_program: &mut Program,
    cached: CachedAnalysisSnapshot,
) -> (AnalysisSelection, AnalysisInvalidationReport) {
    let report = compute_partial_reuse_report(current_program, &cached.units);
    if report.invalidated_units.is_empty() {
        current_program.statements = cached.program.statements;
        return (
            AnalysisSelection {
                statement_mask: vec![false; current_program.statements.len()],
                ..AnalysisSelection::default()
            },
            report,
        );
    }

    let previous_by_key: HashMap<_, _> = cached
        .program
        .statements
        .into_iter()
        .map(|statement| (analysis_unit_key(&statement), statement))
        .collect();
    let invalidated_units: std::collections::HashSet<_> =
        report.invalidated_units.iter().cloned().collect();

    let mut selection = AnalysisSelection {
        statement_mask: Vec::with_capacity(current_program.statements.len()),
        ..AnalysisSelection::default()
    };
    for statement in current_program.statements.iter_mut() {
        let unit_key = analysis_unit_key(statement);
        let should_recheck = invalidated_units.contains(&unit_key);
        if !should_recheck && let Some(previous) = previous_by_key.get(&unit_key) {
            *statement = previous.clone();
            selection.statement_mask.push(false);
            continue;
        }

        if let Some(previous) = previous_by_key.get(&unit_key)
            && let Some(child_mask) =
                prepare_nested_reuse(&unit_key, statement, previous, &invalidated_units)
        {
            selection
                .nested_statement_masks
                .insert(unit_key.clone(), child_mask);
        }
        selection.statement_mask.push(true);
    }

    (selection, report)
}

fn prepare_nested_reuse(
    parent_key: &str,
    current: &mut Statement,
    previous: &Statement,
    invalidated_units: &std::collections::HashSet<String>,
) -> Option<Vec<bool>> {
    let (current_children, previous_children) = match (
        container_children_mut(current),
        container_children(previous),
    ) {
        (Some(current_children), Some(previous_children)) => (current_children, previous_children),
        _ => return None,
    };

    let previous_by_key: HashMap<_, _> = previous_children
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            (
                analysis_child_unit_key(parent_key, statement, index),
                statement.clone(),
            )
        })
        .collect();

    let mut child_mask = Vec::with_capacity(current_children.len());
    for (child_index, child) in current_children.iter_mut().enumerate() {
        let child_key = analysis_child_unit_key(parent_key, child, child_index);
        let should_recheck = invalidated_units.contains(&child_key);
        if !should_recheck && let Some(previous_child) = previous_by_key.get(&child_key) {
            *child = previous_child.clone();
            child_mask.push(false);
            continue;
        }
        child_mask.push(true);
    }

    child_mask
        .iter()
        .any(|should_check| !should_check)
        .then_some(child_mask)
}

fn container_children(statement: &Statement) -> Option<&[Statement]> {
    match statement {
        Statement::Type { fields, .. } => Some(fields.as_slice()),
        | Statement::Impl { methods, .. } => Some(methods.as_slice()),
        _ => None,
    }
}

fn container_children_mut(statement: &mut Statement) -> Option<&mut Vec<Statement>> {
    match statement {
        Statement::Type { fields, .. } => Some(fields),
        | Statement::Impl { methods, .. } => Some(methods),
        _ => None,
    }
}

fn compute_partial_reuse_report(
    current_program: &Program,
    previous_units: &[crate::incremental::AnalysisUnitMetadata],
) -> AnalysisInvalidationReport {
    let current_units = analysis_units_for_program(current_program);
    compute_invalidation_report(previous_units, &current_units)
}
