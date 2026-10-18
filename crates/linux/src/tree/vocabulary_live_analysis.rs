use std::collections::BTreeMap;
use std::time::Instant;

use agent_desktop_core::{Deadline, LocatorField, state};
use zbus::Connection as AsyncConnection;

use super::live_tree::{count_failed, failure_kinds};
use super::role_numbers::{DOCUMENT_FRAME, DOCUMENT_WEB};
use super::walker_live_tests::{Settled, settle};
use super::{
    AtspiElement, LabelOutcome, RawNode, ResolvedVocabulary, WalkedNode, read_label,
    resolve_vocabulary,
};
use crate::system::SharedBus;

const TIMED_RUNS: usize = 7;

pub(super) struct Entry {
    pub(super) node: RawNode,
    pub(super) in_web: bool,
    pub(super) vocabulary: ResolvedVocabulary,
    pub(super) label: LabelOutcome,
}

pub(super) struct Analysis {
    pub(super) label: &'static str,
    pub(super) entries: Vec<Entry>,
    pub(super) label_reads: usize,
    pub(super) label_millis: u128,
}

fn flatten(node: &WalkedNode<RawNode>, in_web: bool, out: &mut Vec<(RawNode, bool)>) {
    out.push((node.payload.clone(), in_web));
    let role = node.payload.traits.role.present().copied();
    let below = in_web || matches!(role, Some(DOCUMENT_WEB | DOCUMENT_FRAME));
    for child in &node.children {
        flatten(child, below, out);
    }
}

pub(super) fn name_is_blank(node: &RawNode) -> bool {
    node.properties
        .name
        .present()
        .is_none_or(|name| name.trim().is_empty())
}

fn analyze(label: &'static str, connection: &AsyncConnection, settled: &Settled) -> Analysis {
    let mut flat = Vec::new();
    flatten(&settled.walk.root, false, &mut flat);
    let mut label_reads = 0;
    let started = Instant::now();
    let mut entries = Vec::with_capacity(flat.len());
    for (node, in_web) in flat {
        let outcome = if name_is_blank(&node) {
            label_reads += 1;
            zbus::block_on(read_label(connection, &node.element))
        } else {
            LabelOutcome::Unlabelled
        };
        let vocabulary = resolve_vocabulary(&node, &outcome, in_web);
        entries.push(Entry {
            node,
            in_web,
            vocabulary,
            label: outcome,
        });
    }
    Analysis {
        label,
        entries,
        label_reads,
        label_millis: started.elapsed().as_millis(),
    }
}

pub(super) fn token(field: &LocatorField<String>) -> String {
    match field {
        LocatorField::Known(value) => value.clone(),
        LocatorField::Absent => "<absent>".to_owned(),
        LocatorField::Unknown => "<unknown>".to_owned(),
    }
}

pub(super) fn is_disabled(entry: &Entry) -> bool {
    matches!(&entry.vocabulary.states, LocatorField::Known(tokens) if tokens.iter().any(|t| t == state::DISABLED))
}

fn print_report(analysis: &Analysis) {
    let mut roles: BTreeMap<String, usize> = BTreeMap::new();
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    let (mut unknown_role, mut unknown_states, mut unknown_name, mut incomplete) = (0, 0, 0, 0);
    let mut label_text = 0;
    for entry in &analysis.entries {
        *roles.entry(token(&entry.vocabulary.role)).or_default() += 1;
        match &entry.vocabulary.states {
            LocatorField::Known(tokens) => {
                for t in tokens {
                    *states.entry(t.clone()).or_default() += 1;
                }
            }
            _ => unknown_states += 1,
        }
        unknown_role += usize::from(entry.vocabulary.role.is_unknown());
        unknown_name += usize::from(entry.vocabulary.name.is_unknown());
        incomplete += usize::from(!entry.vocabulary.identifiers.is_complete());
        label_text += usize::from(matches!(entry.label, LabelOutcome::Text(_)));
    }
    println!(
        "vocabulary_live {}: nodes={} unknown role/states/name={unknown_role}/{unknown_states}/{unknown_name} identifiers_incomplete={incomplete} label_reads={} label_text={label_text} label_ms={}",
        analysis.label,
        analysis.entries.len(),
        analysis.label_reads,
        analysis.label_millis
    );
    println!("vocabulary_live {} roles={roles:?}", analysis.label);
    println!("vocabulary_live {} states={states:?}", analysis.label);
    for entry in analysis.entries.iter().filter(|entry| is_disabled(entry)) {
        println!(
            "vocabulary_live {} disabled: role={} name={}",
            analysis.label,
            token(&entry.vocabulary.role),
            token(&entry.vocabulary.name)
        );
    }
}

fn check_common(analysis: &Analysis) -> Result<(), String> {
    let mut problems = Vec::new();
    for entry in &analysis.entries {
        let vocabulary = &entry.vocabulary;
        let label = analysis.label;
        if vocabulary.role.is_unknown() {
            problems.push(format!("{label}: unknown role"));
        }
        match &vocabulary.states {
            LocatorField::Known(tokens) => state::assert_states_in_vocabulary(tokens),
            _ => problems.push(format!("{label}: unknown states")),
        }
        if vocabulary.name.is_unknown() {
            problems.push(format!("{label}: unknown name"));
        }
        if !vocabulary.identifiers.is_complete() {
            problems.push(format!("{label}: incomplete identifiers"));
        }
    }
    problems.dedup();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

pub(super) fn root_is_window(analysis: &Analysis) -> Result<(), String> {
    let root = &analysis.entries[0].vocabulary.role;
    if *root == LocatorField::Known("window".to_owned()) {
        Ok(())
    } else {
        Err(format!("{}: top-level frame is {root:?}", analysis.label))
    }
}

pub(super) fn walk(
    bus: &SharedBus,
    label: &'static str,
    min_nodes: usize,
    find: impl FnMut() -> Option<AtspiElement>,
) -> Result<(Settled, Analysis), String> {
    let settled = settle(bus, min_nodes, find).map_err(|error| format!("{label}: {error}"))?;
    let deadline = Deadline::standard().map_err(|error| error.message)?;
    let connection = bus.get(deadline).map_err(|error| error.message)?;
    let analysis = analyze(label, connection.inner(), &settled);
    print_report(&analysis);
    if !settled.walk.complete {
        return Err(format!("{label}: walk incomplete"));
    }
    let failed = count_failed(&settled.walk.root);
    if failed > 0 {
        for kind in failure_kinds(&settled.walk.root) {
            println!("vocabulary_live {label} read error: {kind}");
        }
        return Err(format!("{label}: {failed} failed reads"));
    }
    check_common(&analysis).map_err(|problem| format!("{label}: {problem}"))?;
    Ok((settled, analysis))
}

pub(super) fn time_pass(entries: &[Entry]) -> (u128, u128, u128) {
    let mut micros = Vec::new();
    for run in 0..=TIMED_RUNS {
        let started = Instant::now();
        let mut checksum = 0usize;
        for entry in entries {
            let resolved = resolve_vocabulary(&entry.node, &LabelOutcome::Unlabelled, entry.in_web);
            checksum += resolved.descriptor.dom_classes.len();
        }
        std::hint::black_box(checksum);
        if run > 0 {
            micros.push(started.elapsed().as_micros());
        }
    }
    micros.sort_unstable();
    (
        micros[0],
        micros[micros.len() / 2],
        micros[micros.len() - 1],
    )
}
