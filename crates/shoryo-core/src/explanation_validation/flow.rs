use std::collections::HashSet;

use super::{
    Checked, ExplanationRule, bound, canvas, fail, id, nonblank, number, point, reference,
};
use crate::explanation::{Canvas, FlowEdge, FlowKind, FlowNode};

pub(super) fn validate(
    title: &str,
    nodes: &[FlowNode],
    edges: &[FlowEdge],
    area: Option<&Canvas>,
    path: &str,
) -> Checked {
    nonblank(&format!("{path}.title"), title, 120)?;
    bound(&format!("{path}.nodes"), nodes.len(), 1, 32)?;
    bound(&format!("{path}.edges"), edges.len(), 0, 64)?;
    canvas(&format!("{path}.canvas"), area)?;
    let mut ids = HashSet::new();
    for (i, node) in nodes.iter().enumerate() {
        let npath = format!("{path}.nodes[{i}]");
        id(&format!("{npath}.id"), &node.id)?;
        if !ids.insert(node.id.as_str()) {
            return Err(fail(&format!("{npath}.id"), ExplanationRule::Duplicate));
        }
        nonblank(&format!("{npath}.label"), &node.label, 512)?;
        point(&format!("{npath}.position"), node.position.as_ref())?;
        number(&format!("{npath}.width"), node.width, 80, 640)?;
        number(&format!("{npath}.height"), node.height, 32, 640)?;
    }
    let mut count = 0;
    for (i, edge) in edges.iter().enumerate() {
        let epath = format!("{path}.edges[{i}]");
        reference(&format!("{epath}.from"), &edge.from, &ids)?;
        reference(&format!("{epath}.to"), &edge.to, &ids)?;
        if nodes
            .iter()
            .any(|n| n.id == edge.from && n.kind == FlowKind::Decision)
            && edge.label.is_none()
        {
            return Err(fail(
                &format!("{epath}.label"),
                ExplanationRule::MissingCondition,
            ));
        }
        if let Some(via) = &edge.via {
            bound(&format!("{epath}.via"), via.len(), 0, 12)?;
            count += via.len();
            for (j, p) in via.iter().enumerate() {
                point(&format!("{epath}.via[{j}]"), Some(p))?;
            }
        }
        if let Some(label) = &edge.label {
            nonblank(&format!("{epath}.label.text"), &label.text, 512)?;
            point(&format!("{epath}.label.position"), label.position.as_ref())?;
        }
    }
    bound(&format!("{path}.total via points"), count, 0, 768)
}
