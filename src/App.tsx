import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import {
  getSettings,
  getSnapshot,
  getUsageHistory,
  pauseNotifications,
  refreshNow,
  toggleBadge,
  updateSettings,
} from "./api";
import type { AppSettings, MonitorSnapshot, UsageHistory } from "./types";
import {
  currentLimitPercent,
  formatCompact,
  formatDateTime,
  formatNumber,
  statusTone,
  truncateTitle,
  weeklyLimitPercent,
  weeklyLimitResetAt,
} from "./utils";

function App() {
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [history, setHistory] = useState<UsageHistory | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([getSnapshot(), getSettings(), getUsageHistory("7d")])
      .then(([nextSnapshot, nextSettings, nextHistory]) => {
        setSnapshot(nextSnapshot);
        setSettings(nextSettings);
        setHistory(nextHistory);
      })
      .catch((err) => setError(String(err)));

    const unlisten = listen<MonitorSnapshot>("rate_limit_updated", (event) => {
      setSnapshot(event.payload);
    });
    return () => {
      unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  const percent = currentLimitPercent(snapshot);
  const weeklyPercent = weeklyLimitPercent(snapshot);
  const weeklyReset = weeklyLimitResetAt(snapshot);
  const tone = statusTone(percent);
  const weeklyTone = statusTone(weeklyPercent);
  const primaryBucket = snapshot?.buckets[0] ?? null;
  const currentThreadTitle = useMemo(() => {
    if (!snapshot?.currentThread) return "No active thread";
    if (settings?.privacyMode) return "Hidden by privacy mode";
    return truncateTitle(snapshot.currentThread.title || "Untitled thread", 88);
  }, [settings?.privacyMode, snapshot?.currentThread]);

  async function runRefresh() {
    setBusy(true);
    setError(null);
    try {
      setSnapshot(await refreshNow());
      setHistory(await getUsageHistory("7d"));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings(next: AppSettings) {
    setSettings(next);
    try {
      setSettings(await updateSettings(next));
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand-block">
          <div className="brand-mark">CL</div>
          <div>
            <h1>Codex Limit Monitor</h1>
            <p>Live limit and local usage telemetry</p>
          </div>
        </div>
        <StatusPill state={snapshot?.liveStatus.state ?? "loading"} />
        <button className="primary-action" onClick={runRefresh} disabled={busy}>
          {busy ? "Refreshing" : "Refresh Now"}
        </button>
        {settings && (
          <button
            className="secondary-action"
            onClick={async () => setSettings(await toggleBadge(!settings.badgeVisible))}
          >
            {settings.badgeVisible ? "Hide Badge" : "Show Badge"}
          </button>
        )}
        <div className="sidebar-note">
          <span>Source</span>
          <strong>{snapshot?.source ?? "loading"}</strong>
        </div>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">Windows desktop monitor</p>
            <h2>Codex capacity at a glance</h2>
          </div>
          <div className={`connection ${snapshot?.liveStatus.state ?? "loading"}`}>
            {snapshot?.liveStatus.detail ?? snapshot?.message ?? "Live app-server connection"}
          </div>
        </header>

        {error && <div className="alert danger">{error}</div>}
        {snapshot?.message && <div className="alert">{snapshot.message}</div>}

        <section className="overview-grid">
          <div className="limit-panel">
            <div className="panel-header">
              <div>
                <span>Codex limits</span>
                <h3>{primaryBucket?.limitName ?? primaryBucket?.limitId ?? "Codex"}</h3>
              </div>
              <span className={`tone-dot ${tone}`} />
            </div>
            <div className="limit-gauge-grid">
              <LimitGauge
                label="Current window"
                percent={percent}
                tone={tone}
                detail={primaryBucket?.windowDurationMins ? `${primaryBucket.windowDurationMins}m` : "live"}
              />
              <LimitGauge
                label="Weekly limit"
                percent={weeklyPercent}
                tone={weeklyTone}
                detail="7d"
              />
            </div>
            <div className="limit-meta">
              <Metric label="Current reset" value={primaryBucket?.resetsAt ? formatDateTime(primaryBucket.resetsAt) : "Unavailable"} />
              <Metric label="Weekly reset" value={weeklyReset ? formatDateTime(weeklyReset) : "Unavailable"} />
              <Metric label="Reached" value={primaryBucket?.reachedType ?? "No"} />
            </div>
          </div>

          <div className="metric-panel">
            <Metric label="Today" value={formatCompact(snapshot?.usageSummary.todayTokens ?? 0)} detail={`${snapshot?.usageSummary.todayThreadCount ?? 0} threads`} />
            <Metric label="Local Weekly Usage" value={formatCompact(snapshot?.usageSummary.weekTokens ?? 0)} detail={`${snapshot?.usageSummary.weekThreadCount ?? 0} threads`} />
            <Metric label="All Local" value={formatCompact(snapshot?.usageSummary.totalTokens ?? 0)} detail={`${snapshot?.usageSummary.threadCount ?? 0} threads`} />
            <Metric label="Responses" value={formatNumber(snapshot?.usageSummary.tokenBreakdown.responses ?? 0)} detail="from logs" />
          </div>
        </section>

        <section className="work-grid">
          <div className="panel">
            <div className="panel-header">
              <div>
                <span>Current thread</span>
                <h3>{currentThreadTitle}</h3>
              </div>
            </div>
            <div className="thread-current">
              <Metric label="Tokens" value={formatNumber(snapshot?.currentThread?.tokensUsed ?? 0)} />
              <Metric label="Model" value={snapshot?.currentThread?.model ?? "Unknown"} />
              <Metric label="Reasoning" value={snapshot?.currentThread?.reasoningEffort ?? "Default"} />
            </div>
          </div>

          <div className="panel">
            <div className="panel-header">
              <div>
                <span>Token shape</span>
                <h3>Last 7 days</h3>
              </div>
            </div>
            <TokenBars snapshot={snapshot} />
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <div>
              <span>Usage trend</span>
              <h3>Daily tokens</h3>
            </div>
          </div>
          <Trend history={history} />
        </section>

        {settings && (
          <section className="settings-panel">
            <div className="panel-header">
              <div>
                <span>Settings</span>
                <h3>Monitor behavior</h3>
              </div>
            </div>
            <div className="settings-grid">
              <label>
                Codex executable
                <input
                  value={settings.codexExecutable ?? ""}
                  placeholder="codex or full path to codex.cmd"
                  onChange={(event) =>
                    setSettings({ ...settings, codexExecutable: event.target.value || null })
                  }
                  onBlur={() => saveSettings(settings)}
                />
              </label>
              <label>
                Codex home
                <input
                  value={settings.codexHome}
                  onChange={(event) => setSettings({ ...settings, codexHome: event.target.value })}
                  onBlur={() => saveSettings(settings)}
                />
              </label>
              <label>
                Refresh seconds
                <input
                  type="number"
                  min={15}
                  value={settings.refreshIntervalSecs}
                  onChange={(event) =>
                    setSettings({ ...settings, refreshIntervalSecs: Number(event.target.value) })
                  }
                  onBlur={() => saveSettings(settings)}
                />
              </label>
              <label className="toggle-row">
                Privacy mode
                <input
                  type="checkbox"
                  checked={settings.privacyMode}
                  onChange={(event) => saveSettings({ ...settings, privacyMode: event.target.checked })}
                />
              </label>
              <button
                className="secondary-action"
                onClick={() => pauseNotifications(Math.round(Date.now() / 1000) + 3600)}
              >
                Mute 1 Hour
              </button>
            </div>
          </section>
        )}
      </section>
    </main>
  );
}

function StatusPill({ state }: { state: string }) {
  return <div className={`status-pill ${state}`}>{state}</div>;
}

function Metric({
  label,
  value,
  detail,
}: {
  label: string;
  value: string | number;
  detail?: string;
}) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
      {detail && <em>{detail}</em>}
    </div>
  );
}

function LimitGauge({
  label,
  percent,
  tone,
  detail,
}: {
  label: string;
  percent: number;
  tone: string;
  detail: string;
}) {
  const value = Number.isFinite(percent) ? Math.max(0, Math.min(100, percent)) : 0;
  return (
    <div className="limit-gauge-card">
      <div className={`gauge ${tone}`}>
        <div style={{ "--value": value } as React.CSSProperties}>
          <strong>{Number.isFinite(percent) ? `${Math.round(percent)}%` : "--"}</strong>
          <span>used</span>
        </div>
      </div>
      <div>
        <strong>{label}</strong>
        <span>{detail}</span>
      </div>
    </div>
  );
}

function TokenBars({ snapshot }: { snapshot: MonitorSnapshot | null }) {
  const breakdown = snapshot?.usageSummary.tokenBreakdown;
  const values = [
    ["Input", breakdown?.input ?? 0],
    ["Output", breakdown?.output ?? 0],
    ["Cached", breakdown?.cached ?? 0],
    ["Reasoning", breakdown?.reasoning ?? 0],
    ["Tool", breakdown?.tool ?? 0],
  ] as const;
  const max = Math.max(1, ...values.map(([, value]) => value));

  return (
    <div className="bars">
      {values.map(([label, value]) => (
        <div className="bar-row" key={label}>
          <span>{label}</span>
          <div>
            <i style={{ width: `${Math.max(4, (value / max) * 100)}%` }} />
          </div>
          <strong>{formatCompact(value)}</strong>
        </div>
      ))}
    </div>
  );
}

function Trend({ history }: { history: UsageHistory | null }) {
  const points = history?.points ?? [];
  const max = Math.max(1, ...points.map((point) => point.tokens));
  if (points.length === 0) {
    return <div className="empty-state">No local usage history found for this range.</div>;
  }

  return (
    <div className="trend">
      {points.map((point) => (
        <div className="trend-column" key={point.label} title={`${point.label}: ${formatNumber(point.tokens)}`}>
          <i style={{ height: `${Math.max(8, (point.tokens / max) * 100)}%` }} />
          <span>{point.label.slice(5)}</span>
        </div>
      ))}
    </div>
  );
}

export default App;
