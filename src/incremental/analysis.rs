use super::*;
use hashing::stable_statement_hash_pair;

pub fn analysis_units_for_program(program: &Program) -> Vec<AnalysisUnitMetadata> {
    let mut units = Vec::new();
    for statement in &program.statements {
        collect_analysis_units(statement, &mut units);
    }
    units
}

pub fn compute_invalidation_report(
    previous_units: &[AnalysisUnitMetadata],
    current_units: &[AnalysisUnitMetadata],
) -> AnalysisInvalidationReport {
    let previous_by_key: HashMap<_, _> = previous_units
        .iter()
        .map(|unit| (unit.unit_key.clone(), unit))
        .collect();
    let current_by_key: HashMap<_, _> = current_units
        .iter()
        .map(|unit| (unit.unit_key.clone(), unit))
        .collect();

    let mut changed_units = Vec::new();
    let mut added_units = Vec::new();
    let mut removed_units = Vec::new();

    for (key, current) in &current_by_key {
        match previous_by_key.get(key) {
            Some(previous) => {
                if previous.body_hash != current.body_hash
                    || previous.body_hash2 != current.body_hash2
                    || previous.dependencies != current.dependencies
                    || previous.unit_kind != current.unit_kind
                {
                    changed_units.push(key.clone());
                }
            }
            None => added_units.push(key.clone()),
        }
    }

    for key in previous_by_key.keys() {
        if !current_by_key.contains_key(key) {
            removed_units.push(key.clone());
        }
    }

    let mut reverse_dependencies: HashMap<String, Vec<String>> = HashMap::new();
    for current in current_units {
        for dep in &current.dependencies {
            reverse_dependencies
                .entry(dep.clone())
                .or_default()
                .push(current.unit_key.clone());
        }
    }

    let mut invalidated: HashMap<String, ()> = HashMap::new();
    let mut queue = changed_units.clone();
    queue.extend(added_units.clone());
    queue.extend(removed_units.clone());
    let mut queued: HashMap<String, ()> = queue.iter().cloned().map(|k| (k, ())).collect();

    while let Some(unit) = queue.pop() {
        queued.remove(&unit);
        if invalidated.insert(unit.clone(), ()).is_some() {
            continue;
        }

        let mut keys = vec![unit.clone()];
        if let Some((_, suffix)) = unit.rsplit_once('.') {
            keys.push(suffix.to_string());
        }
        if let Some((_, suffix)) = unit.rsplit_once('#') {
            keys.push(suffix.to_string());
        }

        for key in keys {
            if let Some(dependents) = reverse_dependencies.get(&key) {
                for dependent in dependents {
                    if !invalidated.contains_key(dependent) && !queued.contains_key(dependent) {
                        queue.push(dependent.clone());
                        queued.insert(dependent.clone(), ());
                    }
                }
            }
        }
    }

    let mut invalidated_units: Vec<_> = invalidated.into_keys().collect();
    changed_units.sort();
    added_units.sort();
    removed_units.sort();
    invalidated_units.sort();

    AnalysisInvalidationReport {
        changed_units,
        invalidated_units,
        added_units,
        removed_units,
    }
}

fn collect_analysis_units(statement: &Statement, units: &mut Vec<AnalysisUnitMetadata>) {
    let unit = analysis_unit_for_statement(statement);
    let parent_key = unit.unit_key.clone();
    let parent_kind = unit.unit_kind;
    units.push(unit);

    if let Some(children) = direct_analysis_children(statement) {
        for (child_index, child) in children.iter().enumerate() {
            units.push(analysis_child_unit_for_statement(
                &parent_key,
                parent_kind,
                child,
                child_index,
            ));
        }
    }
}

fn analysis_unit_for_statement(statement: &Statement) -> AnalysisUnitMetadata {
    let mut dependencies = Vec::new();
    collect_statement_dependencies(statement, &mut dependencies);
    dependencies.sort();
    dependencies.dedup();

    let (unit_key, unit_kind) = match statement {
        Statement::Function { name, .. } => (name.clone(), AnalysisUnitKind::Function),
        Statement::Type { name, .. } => (name.clone(), AnalysisUnitKind::Type),
        Statement::Enum { name, .. } => (name.clone(), AnalysisUnitKind::Enum),
        Statement::Impl { type_name, .. } => (format!("impl::{type_name}"), AnalysisUnitKind::Impl),
        other => (
            statement_export_name(other)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("{other:?}")),
            AnalysisUnitKind::Other,
        ),
    };

    let (body_hash, body_hash2) = stable_statement_hash_pair(statement);
    AnalysisUnitMetadata {
        unit_key,
        unit_kind,
        body_hash,
        body_hash2,
        dependencies,
        origin: None,
    }
}

pub fn analysis_unit_key(statement: &Statement) -> String {
    analysis_unit_for_statement(statement).unit_key
}

pub fn analysis_child_unit_key(parent_key: &str, child: &Statement, child_index: usize) -> String {
    match child {
        Statement::Function { name, .. } => {
            if let Some(type_name) = parent_key.strip_prefix("impl::") {
                format!("{type_name}.{name}")
            } else {
                format!("{parent_key}.{name}")
            }
        }
        Statement::Let { name, .. } => format!("{parent_key}#{name}"),
        Statement::Type { name, .. } | Statement::Enum { name, .. } => {
            format!("{parent_key}::{name}")
        }
        Statement::Impl {
            trait_name,
            type_name,
            ..
        } => format!(
            "{parent_key}::impl::{}::{type_name}",
            trait_name.as_deref().unwrap_or("_")
        ),
        _ => format!("{parent_key}::item::{child_index}"),
    }
}

fn analysis_child_unit_for_statement(
    parent_key: &str,
    parent_kind: AnalysisUnitKind,
    child: &Statement,
    child_index: usize,
) -> AnalysisUnitMetadata {
    let mut dependencies = Vec::new();
    collect_statement_dependencies(child, &mut dependencies);
    dependencies.push(parent_key.to_string());
    dependencies.sort();
    dependencies.dedup();

    let unit_kind = match child {
        Statement::Function { .. } => AnalysisUnitKind::Function,
        Statement::Let { .. } if matches!(parent_kind, AnalysisUnitKind::Type) => {
            AnalysisUnitKind::Field
        }
        Statement::Type { .. } => AnalysisUnitKind::Type,
        Statement::Enum { .. } => AnalysisUnitKind::Enum,
        Statement::Impl { .. } => AnalysisUnitKind::Impl,
        _ => AnalysisUnitKind::Other,
    };

    let (body_hash, body_hash2) = stable_statement_hash_pair(child);
    AnalysisUnitMetadata {
        unit_key: analysis_child_unit_key(parent_key, child, child_index),
        unit_kind,
        body_hash,
        body_hash2,
        dependencies,
        origin: None,
    }
}

fn direct_analysis_children(statement: &Statement) -> Option<&[Statement]> {
    match statement {
        Statement::Type { fields, .. } => Some(fields.as_slice()),
        Statement::Impl { methods, .. } => Some(methods.as_slice()),
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn dependency_matches_unit(dependency: &str, unit_key: &str) -> bool {
    dependency == unit_key
        || dependency
            == unit_key
                .rsplit_once('.')
                .map(|(_, suffix)| suffix)
                .unwrap_or_default()
        || dependency
            == unit_key
                .rsplit_once('#')
                .map(|(_, suffix)| suffix)
                .unwrap_or_default()
}
