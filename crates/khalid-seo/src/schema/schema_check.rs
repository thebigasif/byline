//! [`check_graph`]: a graph measured against Google's required and
//! recommended properties.

use khalid_core::UserMessage;

use crate::schema::google_rules::{Rule, rule_for};
use crate::{SchemaGraph, SchemaNode, SchemaValue};

/// How much a missing property matters (plan section 5.9 and D-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum IssueLevel {
    /// Google needs it, so the post does not publish until it is there.
    Blocks,
    /// Google shows more with it, and the post still publishes.
    Warns,
}

/// A property Google asks for that a node in the graph does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaIssue {
    level: IssueLevel,
    kind: String,
    property: String,
    message: UserMessage,
}

impl SchemaIssue {
    /// Whether the issue blocks publishing or only warns.
    pub fn level(&self) -> IssueLevel {
        self.level
    }

    /// The schema.org type of the node, such as `Event`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// The missing property, or the choice of them, such as `startDate` or
    /// `review, aggregateRating or offers`.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// What is missing, why it matters and how to fix it.
    pub fn message(&self) -> &UserMessage {
        &self.message
    }
}

/// Every property Google asks for that a node of `graph` lacks, nodes inside
/// nodes included, in the order the nodes are written. A node that stands
/// alone may need more than the same node inside another, such as a
/// `Review`'s `itemReviewed`. The item a review is about needs only its
/// `name`, as Google's review guide says, not what it would need as a rich
/// result of its own. A type Google shows no rich result for raises nothing.
pub fn check_graph(graph: &SchemaGraph) -> Vec<SchemaIssue> {
    let mut issues = Vec::new();
    for node in graph.nodes() {
        check_node(node, Placement::Alone, &mut issues);
    }
    issues
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    Alone,
    Nested,
    Reviewed,
}

fn check_node(node: &SchemaNode, placement: Placement, issues: &mut Vec<SchemaIssue>) {
    if placement == Placement::Reviewed {
        if !has(node, "name") {
            let feature = "a review with its stars";
            issues.push(issue(IssueLevel::Blocks, node.kind(), &["name"], feature));
        }
    } else if let Some(rule) = rule_for(node.kind()) {
        check_rule(node, rule, placement, issues);
    }
    for (name, value) in node.properties() {
        let placement = if name == "itemReviewed" {
            Placement::Reviewed
        } else {
            Placement::Nested
        };
        check_value(value, placement, issues);
    }
}

fn check_value(value: &SchemaValue, placement: Placement, issues: &mut Vec<SchemaIssue>) {
    match value {
        SchemaValue::Node(inner) => check_node(inner, placement, issues),
        SchemaValue::List(values) => {
            for value in values {
                check_value(value, placement, issues);
            }
        }
        _ => {}
    }
}

fn check_rule(node: &SchemaNode, rule: Rule, placement: Placement, issues: &mut Vec<SchemaIssue>) {
    let alone: Vec<Vec<&str>> = match placement {
        Placement::Alone => rule.required_alone().collect(),
        Placement::Nested | Placement::Reviewed => Vec::new(),
    };
    for choice in rule.required().chain(alone) {
        if !choice.iter().any(|name| has(node, name)) {
            issues.push(issue(
                IssueLevel::Blocks,
                node.kind(),
                &choice,
                rule.feature,
            ));
        }
    }
    for name in rule.recommended() {
        if !has(node, name) {
            issues.push(issue(IssueLevel::Warns, node.kind(), &[name], rule.feature));
        }
    }
}

/// Whether `node` has `name` with something in it.
fn has(node: &SchemaNode, name: &str) -> bool {
    match node.get(name) {
        None => false,
        Some(SchemaValue::Text(text)) => !text.trim().is_empty(),
        Some(SchemaValue::List(values)) => !values.is_empty(),
        Some(_) => true,
    }
}

/// `a`, `a or b`, `a, b or c`.
fn either(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

fn issue(level: IssueLevel, kind: &str, choice: &[&str], feature: &str) -> SchemaIssue {
    let property = either(choice);
    let (what, them) = if choice.len() > 1 {
        (
            format!("This {kind} has none of {property}."),
            "one of them",
        )
    } else {
        (format!("This {kind} has no {property}."), "one")
    };
    let why = match level {
        IssueLevel::Blocks => format!("Google shows {feature} only when it has {them}."),
        IssueLevel::Warns => format!("Google can still show {feature} without it, but shows less."),
    };
    let fix = if choice.len() > 1 {
        format!("Add one of {property} to the {kind}.")
    } else {
        format!("Add {property} to the {kind}.")
    };
    SchemaIssue {
        level,
        kind: kind.to_owned(),
        property,
        message: UserMessage::new(what, why, fix),
    }
}
