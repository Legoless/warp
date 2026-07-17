//! Terminal block read handlers for local-control actions.
use ::local_control::protocol::{BlockQueryParams, BlockReadParams, TargetSelector};
use ::local_control::{Action, ActionKind, ControlError, ErrorCode};
use serde_json::json;
use warp_terminal::model::BlockId;
use warpui::ModelContext;

use crate::local_control::resolver::{decode_params, target_pane_group, target_session_pane_id};
use crate::local_control::LocalControlBridge;
use crate::terminal::model::block::Block;

const DEFAULT_BLOCK_LIST_LIMIT: usize = 50;
const DEFAULT_MAX_OUTPUT_CHARS: usize = 100_000;

pub(crate) fn block_list(
    action: &Action,
    target: &TargetSelector,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = decode_params::<BlockQueryParams>(&action.params)?;
    let pane_group = target_pane_group(ActionKind::BlockList, target, ctx)?;
    let pane_id = target_session_pane_id(ActionKind::BlockList, target, &pane_group, ctx)?;
    let terminal_view = pane_group
        .read(ctx, |pane_group, ctx| {
            pane_group.terminal_view_from_pane_id(pane_id, ctx)
        })
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                "block.list requires a terminal session target",
            )
        })?;
    let limit = params
        .limit
        .map(|limit| limit as usize)
        .unwrap_or(DEFAULT_BLOCK_LIST_LIMIT);
    let max_output_chars = params.max_output_chars.unwrap_or(DEFAULT_MAX_OUTPUT_CHARS);

    terminal_view.read(ctx, |terminal_view, _ctx| {
        let model = terminal_view.model.lock();
        let mut blocks = model
            .block_list()
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, block)| params.include_hidden || !block.is_hidden())
            .rev()
            .take(limit)
            .map(|(index, block)| {
                block_to_json(index, block, params.include_output, max_output_chars)
            })
            .collect::<Vec<_>>();
        blocks.reverse();
        Ok(json!({
            "blocks": blocks,
            "count": blocks.len(),
            "limit": limit,
            "include_hidden": params.include_hidden,
            "include_output": params.include_output,
        }))
    })
}

pub(crate) fn block_read(
    action: &Action,
    target: &TargetSelector,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> Result<serde_json::Value, ControlError> {
    let params = decode_params::<BlockReadParams>(&action.params)?;
    let pane_group = target_pane_group(ActionKind::BlockRead, target, ctx)?;
    let pane_id = target_session_pane_id(ActionKind::BlockRead, target, &pane_group, ctx)?;
    let terminal_view = pane_group
        .read(ctx, |pane_group, ctx| {
            pane_group.terminal_view_from_pane_id(pane_id, ctx)
        })
        .ok_or_else(|| {
            ControlError::new(
                ErrorCode::MissingTarget,
                "block.read requires a terminal session target",
            )
        })?;
    let max_output_chars = params.max_output_chars.unwrap_or(DEFAULT_MAX_OUTPUT_CHARS);

    terminal_view.read(ctx, |terminal_view, _ctx| {
        let model = terminal_view.model.lock();
        let block_list = model.block_list();
        let selected = if let Some(block_id) = params.block_id.as_ref() {
            let block_id = BlockId::from(block_id.clone());
            block_list
                .blocks()
                .iter()
                .enumerate()
                .find(|(_, block)| block.id() == &block_id)
        } else if let Some(index) = params.index {
            block_list
                .blocks()
                .get(index as usize)
                .map(|block| (index as usize, block))
        } else {
            block_list
                .blocks()
                .iter()
                .enumerate()
                .rev()
                .find(|(_, block)| !block.is_hidden())
        };
        let Some((index, block)) = selected else {
            return Err(ControlError::new(
                ErrorCode::MissingTarget,
                "block.read could not resolve the requested block",
            ));
        };
        Ok(json!({
            "block": block_to_json(index, block, params.include_output, max_output_chars),
        }))
    })
}

fn block_to_json(
    index: usize,
    block: &Block,
    include_output: bool,
    max_output_chars: usize,
) -> serde_json::Value {
    let (output, output_truncated_from_start) = if include_output {
        let output = block.output_to_string_force_full_grid_contents();
        let (output, truncated) = truncate_from_start(output, max_output_chars);
        (Some(output), truncated)
    } else {
        (None, false)
    };

    json!({
        "id": block.id().as_str(),
        "index": index,
        "state": format!("{:?}", block.state()),
        "command": block.command_with_secrets_obfuscated(false),
        "pwd": block.pwd().map(String::as_str),
        "session_id": block.session_id().map(|session_id| session_id.as_u64()),
        "exit_code": block.exit_code().value(),
        "created_at": block.creation_ts().to_rfc3339(),
        "started_at": block.start_ts().map(|timestamp| timestamp.to_rfc3339()),
        "completed_at": block.completed_ts().map(|timestamp| timestamp.to_rfc3339()),
        "hidden": block.is_hidden(),
        "output": output,
        "output_truncated_from_start": output_truncated_from_start,
    })
}

fn truncate_from_start(text: String, max_chars: usize) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return (text, false);
    }
    (text.chars().skip(char_count - max_chars).collect(), true)
}
