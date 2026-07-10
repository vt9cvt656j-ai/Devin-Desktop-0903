use axum::Json;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_AGENT_TRACES: usize = 200;

#[derive(Clone, Debug, Serialize)]
pub struct AgentTrace {
    pub id: u64,
    pub created_at_ms: u128,
    pub mode: String,
    pub prompt_blocks: Vec<String>,
    pub requested_tool_count: usize,
    pub injected_tool_count: usize,
    pub missing_tool_count: usize,
    pub final_message_count: usize,
}

#[derive(Clone, Debug)]
pub struct AgentTraceInput {
    pub mode: String,
    pub prompt_blocks: Vec<String>,
    pub requested_tool_count: usize,
    pub injected_tool_count: usize,
    pub missing_tool_count: usize,
    pub final_message_count: usize,
}

static TRACES: OnceLock<Mutex<VecDeque<AgentTrace>>> = OnceLock::new();
static NEXT_ID: OnceLock<Mutex<u64>> = OnceLock::new();

fn traces() -> &'static Mutex<VecDeque<AgentTrace>> {
    TRACES.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_AGENT_TRACES)))
}

fn next_id() -> u64 {
    let mut guard = NEXT_ID
        .get_or_init(|| Mutex::new(1))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let id = *guard;
    *guard = guard.saturating_add(1);
    id
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub fn record_agent_trace(input: AgentTraceInput) {
    let trace = AgentTrace {
        id: next_id(),
        created_at_ms: now_ms(),
        mode: input.mode,
        prompt_blocks: input.prompt_blocks,
        requested_tool_count: input.requested_tool_count,
        injected_tool_count: input.injected_tool_count,
        missing_tool_count: input.missing_tool_count,
        final_message_count: input.final_message_count,
    };

    let mut guard = traces()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.len() >= MAX_AGENT_TRACES {
        guard.pop_front();
    }
    guard.push_back(trace);
}

pub async fn list_agent_traces() -> Json<Vec<AgentTrace>> {
    let guard = traces()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Json(guard.iter().rev().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_buffer_keeps_recent_entries() {
        for index in 0..(MAX_AGENT_TRACES + 5) {
            record_agent_trace(AgentTraceInput {
                mode: format!("mode-{index}"),
                prompt_blocks: vec!["base".to_string()],
                requested_tool_count: index,
                injected_tool_count: 1,
                missing_tool_count: 0,
                final_message_count: 2,
            });
        }

        let guard = traces().lock().unwrap();
        assert_eq!(guard.len(), MAX_AGENT_TRACES);
        assert_eq!(guard.front().unwrap().mode, "mode-5");
        assert_eq!(
            guard.back().unwrap().mode,
            format!("mode-{}", MAX_AGENT_TRACES + 4)
        );
    }
}
