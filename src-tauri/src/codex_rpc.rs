use crate::{models::RateLimitBucket, settings::AppSettings};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdout, Command},
    time::timeout,
};

const RPC_TIMEOUT: Duration = Duration::from_secs(20);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Error)]
pub enum CodexRpcError {
    #[error("failed to start Codex app-server: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Codex app-server did not expose stdio")]
    MissingPipe,
    #[error("Codex app-server timed out")]
    Timeout,
    #[error("Codex app-server closed stdout")]
    Closed,
    #[error("invalid app-server JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("app-server returned error: {0}")]
    Rpc(String),
    #[error("app-server response was missing result")]
    MissingResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsEnvelope {
    rate_limits: Option<RawRateLimitBucket>,
    rate_limits_by_limit_id: Option<BTreeMap<String, RawRateLimitBucket>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRateLimitBucket {
    limit_id: String,
    limit_name: Option<String>,
    primary: Option<RawLimitWindow>,
    secondary: Option<RawLimitWindow>,
    rate_limit_reached_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLimitWindow {
    used_percent: Option<f64>,
    window_duration_mins: Option<u64>,
    resets_at: Option<i64>,
}

pub async fn fetch_rate_limits(
    settings: &AppSettings,
) -> Result<Vec<RateLimitBucket>, CodexRpcError> {
    let executable = settings.resolve_codex_executable();
    let mut command = Command::new(executable);
    command
        .arg("app-server")
        .arg("--listen")
        .arg("stdio://")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command.spawn()?;

    let mut stdin = child.stdin.take().ok_or(CodexRpcError::MissingPipe)?;
    let stdout = child.stdout.take().ok_or(CodexRpcError::MissingPipe)?;
    let mut reader = BufReader::new(stdout).lines();

    write_json(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "codex_limit_monitor",
                    "title": "Codex Limit Monitor",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )
    .await?;

    read_response(&mut reader, 0).await?;

    write_json(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    )
    .await?;

    write_json(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "account/rateLimits/read",
            "id": 1
        }),
    )
    .await?;

    let response = read_response(&mut reader, 1).await?;
    let _ = child.kill().await;

    let result = response
        .get("result")
        .cloned()
        .ok_or(CodexRpcError::MissingResult)?;
    parse_rate_limits(result)
}

async fn write_json(
    stdin: &mut tokio::process::ChildStdin,
    value: Value,
) -> Result<(), CodexRpcError> {
    let mut line = serde_json::to_vec(&value)?;
    line.push(b'\n');
    stdin.write_all(&line).await?;
    stdin.flush().await?;
    Ok(())
}

async fn read_response(
    reader: &mut tokio::io::Lines<BufReader<ChildStdout>>,
    id: u64,
) -> Result<Value, CodexRpcError> {
    loop {
        let line = timeout(RPC_TIMEOUT, reader.next_line())
            .await
            .map_err(|_| CodexRpcError::Timeout)?
            .map_err(CodexRpcError::Spawn)?
            .ok_or(CodexRpcError::Closed)?;

        let value: Value = serde_json::from_str(&line)?;
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }

        if let Some(error) = value.get("error") {
            return Err(CodexRpcError::Rpc(error.to_string()));
        }

        return Ok(value);
    }
}

fn parse_rate_limits(result: Value) -> Result<Vec<RateLimitBucket>, CodexRpcError> {
    let envelope: RateLimitsEnvelope = serde_json::from_value(result)?;
    let mut buckets: Vec<RateLimitBucket> = Vec::new();

    if let Some(by_id) = envelope.rate_limits_by_limit_id {
        for raw in by_id.into_values() {
            buckets.push(raw.into());
        }
    } else if let Some(raw) = envelope.rate_limits {
        buckets.push(raw.into());
    }

    buckets.sort_by(|a, b| a.limit_id.cmp(&b.limit_id));
    Ok(buckets)
}

impl From<RawRateLimitBucket> for RateLimitBucket {
    fn from(raw: RawRateLimitBucket) -> Self {
        let primary = raw.primary.unwrap_or_default();
        let secondary = raw.secondary;
        Self {
            limit_id: raw.limit_id,
            limit_name: raw.limit_name,
            used_percent: primary.used_percent.unwrap_or(0.0),
            window_duration_mins: primary.window_duration_mins,
            resets_at: primary.resets_at,
            secondary_used_percent: secondary.as_ref().and_then(|value| value.used_percent),
            secondary_resets_at: secondary.and_then(|value| value.resets_at),
            reached_type: raw.rate_limit_reached_type,
        }
    }
}

impl Default for RawLimitWindow {
    fn default() -> Self {
        Self {
            used_percent: Some(0.0),
            window_duration_mins: None,
            resets_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_bucket_rate_limits() {
        let value = json!({
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "limitName": null,
                    "primary": {
                        "usedPercent": 25,
                        "windowDurationMins": 15,
                        "resetsAt": 1730947200
                    },
                    "secondary": null,
                    "rateLimitReachedType": null
                },
                "codex_other": {
                    "limitId": "codex_other",
                    "limitName": "codex_other",
                    "primary": {
                        "usedPercent": 42,
                        "windowDurationMins": 60,
                        "resetsAt": 1730950800
                    }
                }
            }
        });

        let buckets = parse_rate_limits(value).unwrap();

        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].limit_id, "codex");
        assert_eq!(buckets[0].used_percent, 25.0);
        assert_eq!(buckets[1].limit_id, "codex_other");
        assert_eq!(buckets[1].window_duration_mins, Some(60));
    }
}
