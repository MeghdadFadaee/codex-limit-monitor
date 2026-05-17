use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitBucket {
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<i64>,
    pub secondary_used_percent: Option<f64>,
    pub secondary_resets_at: Option<i64>,
    pub reached_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    pub state: String,
    pub detail: Option<String>,
}

impl LiveStatus {
    pub fn online() -> Self {
        Self {
            state: "online".to_string(),
            detail: None,
        }
    }

    pub fn fallback(detail: impl Into<String>) -> Self {
        Self {
            state: "fallback".to_string(),
            detail: Some(detail.into()),
        }
    }

    pub fn offline(detail: impl Into<String>) -> Self {
        Self {
            state: "offline".to_string(),
            detail: Some(detail.into()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input: i64,
    pub output: i64,
    pub cached: i64,
    pub reasoning: i64,
    pub tool: i64,
    pub responses: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub tokens: i64,
    pub threads: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUsage {
    pub id: String,
    pub title: String,
    pub tokens_used: i64,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub updated_at: i64,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub total_tokens: i64,
    pub today_tokens: i64,
    pub week_tokens: i64,
    pub thread_count: i64,
    pub today_thread_count: i64,
    pub week_thread_count: i64,
    pub current_thread: Option<ThreadUsage>,
    pub recent_threads: Vec<ThreadUsage>,
    pub model_breakdown: Vec<ModelUsage>,
    pub token_breakdown: TokenBreakdown,
    pub source_path: Option<String>,
    pub error: Option<String>,
}

impl UsageSummary {
    pub fn has_data(&self) -> bool {
        self.thread_count > 0 || self.total_tokens > 0 || !self.recent_threads.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub source: String,
    pub fetched_at: i64,
    pub live_status: LiveStatus,
    pub buckets: Vec<RateLimitBucket>,
    pub usage_summary: UsageSummary,
    pub current_thread: Option<ThreadUsage>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsagePoint {
    pub label: String,
    pub tokens: i64,
    pub threads: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistory {
    pub range: String,
    pub points: Vec<UsagePoint>,
}
