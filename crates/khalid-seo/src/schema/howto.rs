//! The how-to block: `HowTo`.

use std::time::Duration;

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{Money, SchemaNode, SchemaValue, SeoError};

/// One step of a how-to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HowToStep {
    /// A short name for the step.
    pub name: String,
    /// What to do.
    pub text: String,
}

impl HowToStep {
    /// A step with a name and its instructions.
    pub fn new(name: &str, text: &str) -> Self {
        Self {
            name: name.to_owned(),
            text: text.to_owned(),
        }
    }
}

/// A how-to block: steps, with optional tools, supplies, time and cost.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HowTo {
    /// What the how-to achieves.
    pub name: String,
    /// The steps, in order.
    pub steps: Vec<HowToStep>,
    /// Things used but not used up.
    pub tools: Vec<String>,
    /// Things used up.
    pub supplies: Vec<String>,
    /// How long it takes in all.
    pub total_time: Option<Duration>,
    /// What it costs.
    pub estimated_cost: Option<Money>,
}

impl HowTo {
    /// A how-to with a name and its steps.
    pub fn new(name: &str, steps: Vec<HowToStep>) -> Self {
        Self {
            name: name.to_owned(),
            steps,
            tools: Vec::new(),
            supplies: Vec::new(),
            total_time: None,
            estimated_cost: None,
        }
    }
}

fn named(kind: &str, name: &str) -> SchemaValue {
    SchemaValue::Node(Box::new(
        SchemaNode::new(kind).with("name", SchemaValue::text(name)),
    ))
}

/// A `MonetaryAmount` for `money`, as schema.org writes prices and costs.
pub(crate) fn amount_node(money: &Money) -> SchemaValue {
    let amount = SchemaNode::new("MonetaryAmount")
        .with("currency", SchemaValue::text(money.currency()))
        .with("value", SchemaValue::Text(money.to_string()));
    SchemaValue::Node(Box::new(amount))
}

/// The `HowTo` at `<page>#howto-<number>`.
pub(crate) fn how_to_node(
    how_to: &HowTo,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let steps = (1..)
        .zip(&how_to.steps)
        .map(|(position, step)| {
            let node = SchemaNode::new("HowToStep")
                .with("position", SchemaValue::Integer(position))
                .with("name", SchemaValue::text(&step.name))
                .with("text", SchemaValue::text(&step.text));
            SchemaValue::Node(Box::new(node))
        })
        .collect();
    let mut node = SchemaNode::new("HowTo")
        .with_id(node_id(page, &format!("howto-{number}"))?)
        .with("name", SchemaValue::text(&how_to.name))
        .with("step", SchemaValue::List(steps));
    if !how_to.tools.is_empty() {
        let tools = how_to.tools.iter().map(|t| named("HowToTool", t)).collect();
        node.set("tool", SchemaValue::List(tools));
    }
    if !how_to.supplies.is_empty() {
        let supplies = how_to
            .supplies
            .iter()
            .map(|s| named("HowToSupply", s))
            .collect();
        node.set("supply", SchemaValue::List(supplies));
    }
    if let Some(time) = how_to.total_time {
        node.set("totalTime", SchemaValue::Duration(time));
    }
    if let Some(cost) = &how_to.estimated_cost {
        node.set("estimatedCost", amount_node(cost));
    }
    Ok(node)
}
