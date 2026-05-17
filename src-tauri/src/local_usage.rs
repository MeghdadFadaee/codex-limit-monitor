use crate::{
    models::{ModelUsage, ThreadUsage, TokenBreakdown, UsageHistory, UsagePoint, UsageSummary},
    settings::AppSettings,
};
use chrono::{Local, TimeZone, Utc};
use regex::Regex;
use rusqlite::{Connection, OpenFlags, Row};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LocalUsageError {
    #[error("Codex state database was not found at {0}")]
    MissingStateDb(String),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub fn read_usage_summary(settings: &AppSettings) -> UsageSummary {
    match read_usage_summary_inner(settings) {
        Ok(summary) => summary,
        Err(error) => UsageSummary {
            source_path: Some(settings.codex_home.clone()),
            error: Some(error.to_string()),
            ..UsageSummary::default()
        },
    }
}

pub fn read_usage_history(
    settings: &AppSettings,
    range: &str,
) -> Result<UsageHistory, LocalUsageError> {
    let state_path = state_db_path(settings);
    if !state_path.exists() {
        return Err(LocalUsageError::MissingStateDb(
            state_path.to_string_lossy().to_string(),
        ));
    }

    let conn = open_read_only(&state_path)?;
    let days = match range {
        "24h" => 1,
        "30d" => 30,
        _ => 7,
    };
    let since = Utc::now().timestamp() - days * 86_400;
    let mut statement = conn.prepare(
        "SELECT date(updated_at, 'unixepoch', 'localtime') AS bucket,
                COALESCE(SUM(tokens_used), 0) AS tokens,
                COUNT(*) AS threads
           FROM threads
          WHERE updated_at >= ?1
          GROUP BY bucket
          ORDER BY bucket ASC",
    )?;

    let points = statement
        .query_map([since], |row| {
            Ok(UsagePoint {
                label: row.get(0)?,
                tokens: row.get(1)?,
                threads: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(UsageHistory {
        range: range.to_string(),
        points,
    })
}

fn read_usage_summary_inner(settings: &AppSettings) -> Result<UsageSummary, LocalUsageError> {
    let state_path = state_db_path(settings);
    if !state_path.exists() {
        return Err(LocalUsageError::MissingStateDb(
            state_path.to_string_lossy().to_string(),
        ));
    }

    let conn = open_read_only(&state_path)?;
    let today_start = local_midnight_timestamp();
    let week_start = Utc::now().timestamp() - 7 * 86_400;

    let (total_tokens, thread_count) = sum_tokens(&conn, None)?;
    let (today_tokens, today_thread_count) = sum_tokens(&conn, Some(today_start))?;
    let (week_tokens, week_thread_count) = sum_tokens(&conn, Some(week_start))?;
    let recent_threads = recent_threads(&conn)?;
    let current_thread = recent_threads.first().cloned();
    let model_breakdown = model_breakdown(&conn)?;
    let token_breakdown = read_token_breakdown(settings, week_start).unwrap_or_default();

    Ok(UsageSummary {
        total_tokens,
        today_tokens,
        week_tokens,
        thread_count,
        today_thread_count,
        week_thread_count,
        current_thread,
        recent_threads,
        model_breakdown,
        token_breakdown,
        source_path: Some(state_path.to_string_lossy().to_string()),
        error: None,
    })
}

fn open_read_only(path: &Path) -> Result<Connection, rusqlite::Error> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
}

fn state_db_path(settings: &AppSettings) -> PathBuf {
    settings.codex_home_path().join("state_5.sqlite")
}

fn logs_db_path(settings: &AppSettings) -> PathBuf {
    settings.codex_home_path().join("logs_2.sqlite")
}

fn local_midnight_timestamp() -> i64 {
    let date = Local::now().date_naive();
    let midnight = date.and_hms_opt(0, 0, 0).expect("valid midnight");
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .expect("valid local midnight")
        .timestamp()
}

fn sum_tokens(conn: &Connection, since: Option<i64>) -> Result<(i64, i64), rusqlite::Error> {
    if let Some(since) = since {
        conn.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0), COUNT(*) FROM threads WHERE updated_at >= ?1",
            [since],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    } else {
        conn.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0), COUNT(*) FROM threads",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }
}

fn recent_threads(conn: &Connection) -> Result<Vec<ThreadUsage>, rusqlite::Error> {
    let mut statement = conn.prepare(
        "SELECT id, title, tokens_used, model, reasoning_effort, updated_at, cwd
           FROM threads
          ORDER BY updated_at DESC
          LIMIT 12",
    )?;

    let rows = statement
        .query_map([], row_to_thread)?
        .collect::<Result<Vec<_>, _>>();
    rows
}

fn model_breakdown(conn: &Connection) -> Result<Vec<ModelUsage>, rusqlite::Error> {
    let mut statement = conn.prepare(
        "SELECT COALESCE(NULLIF(model, ''), 'unknown') AS model,
                COALESCE(SUM(tokens_used), 0) AS tokens,
                COUNT(*) AS threads
           FROM threads
          GROUP BY model
          ORDER BY tokens DESC
          LIMIT 8",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok(ModelUsage {
                model: row.get(0)?,
                tokens: row.get(1)?,
                threads: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>();
    rows
}

fn row_to_thread(row: &Row<'_>) -> Result<ThreadUsage, rusqlite::Error> {
    Ok(ThreadUsage {
        id: row.get(0)?,
        title: row
            .get::<_, String>(1)
            .unwrap_or_else(|_| "Untitled".to_string()),
        tokens_used: row.get(2)?,
        model: row.get(3)?,
        reasoning_effort: row.get(4)?,
        updated_at: row.get(5)?,
        cwd: row.get(6)?,
    })
}

fn read_token_breakdown(
    settings: &AppSettings,
    since: i64,
) -> Result<TokenBreakdown, LocalUsageError> {
    let logs_path = logs_db_path(settings);
    if !logs_path.exists() {
        return Ok(TokenBreakdown::default());
    }

    let conn = open_read_only(&logs_path)?;
    let mut statement = conn.prepare(
        "SELECT feedback_log_body
           FROM logs
          WHERE ts >= ?1
            AND feedback_log_body LIKE '%event.kind=response.completed%'
          ORDER BY ts DESC
          LIMIT 5000",
    )?;

    let mut rows = statement.query([since])?;
    let mut breakdown = TokenBreakdown::default();
    while let Some(row) = rows.next()? {
        let body: Option<String> = row.get(0)?;
        if let Some(body) = body {
            add_response_tokens(&mut breakdown, &body);
        }
    }

    Ok(breakdown)
}

fn add_response_tokens(breakdown: &mut TokenBreakdown, body: &str) {
    let regex = Regex::new(r"(input|output|cached|reasoning|tool)_token_count=(\d+)")
        .expect("static token regex is valid");
    let mut matched = false;

    for capture in regex.captures_iter(body) {
        let count = capture
            .get(2)
            .and_then(|value| value.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        match capture.get(1).map(|value| value.as_str()) {
            Some("input") => breakdown.input += count,
            Some("output") => breakdown.output += count,
            Some("cached") => breakdown.cached += count,
            Some("reasoning") => breakdown.reasoning += count,
            Some("tool") => breakdown.tool += count,
            _ => {}
        }
        matched = true;
    }

    if matched {
        breakdown.responses += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_response_token_counts() {
        let mut breakdown = TokenBreakdown::default();
        add_response_tokens(
            &mut breakdown,
            "event.kind=response.completed input_token_count=40917 output_token_count=410 cached_token_count=39296 reasoning_token_count=32 tool_token_count=41327",
        );

        assert_eq!(breakdown.input, 40917);
        assert_eq!(breakdown.output, 410);
        assert_eq!(breakdown.cached, 39296);
        assert_eq!(breakdown.reasoning, 32);
        assert_eq!(breakdown.tool, 41327);
        assert_eq!(breakdown.responses, 1);
    }
}
