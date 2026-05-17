import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getSettings, getSnapshot, moveBadge, setBadgePosition } from "./api";
import type { BadgePosition, MonitorSnapshot } from "./types";
import {
  currentLimitPercent,
  currentLimitResetAt,
  formatCountdown,
  resetWindowProgress,
  statusTone,
  weeklyLimitPercent,
} from "./utils";
import "./styles.css";

function Badge() {
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [position, setPosition] = useState<BadgePosition>({ x: 1530, y: 18 });
  const [refreshing, setRefreshing] = useState(true);
  const [nowMs, setNowMs] = useState(Date.now());

  useEffect(() => {
    Promise.all([getSnapshot(), getSettings()])
      .then(([nextSnapshot, settings]) => {
        setSnapshot(nextSnapshot);
        setPosition(settings.badgePosition);
      })
      .catch(() => undefined)
      .finally(() => setRefreshing(false));
    const started = listen("refresh_started", () => {
      setRefreshing(true);
    });
    const unlisten = listen<MonitorSnapshot>("rate_limit_updated", (event) => {
      setSnapshot(event.payload);
      setRefreshing(false);
    });
    const timer = window.setInterval(() => setNowMs(Date.now()), 30_000);
    return () => {
      window.clearInterval(timer);
      started.then((dispose) => dispose()).catch(() => undefined);
      unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    const blockContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("contextmenu", blockContextMenu, true);
    return () => window.removeEventListener("contextmenu", blockContextMenu, true);
  }, []);

  const currentPercent = currentLimitPercent(snapshot);
  const weeklyPercent = weeklyLimitPercent(snapshot);
  const resetAt = currentLimitResetAt(snapshot);
  const resetProgress = resetWindowProgress(resetAt, nowMs);
  const peakPercent = Math.max(
    Number.isFinite(currentPercent) ? currentPercent : 0,
    Number.isFinite(weeklyPercent) ? weeklyPercent : 0,
  );
  const tone = statusTone(peakPercent);

  function dragBadge(event: React.PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);

    const startMouse = { x: event.screenX, y: event.screenY };
    const startPosition = position;
    let latest = startPosition;
    let scheduled = false;

    const calculate = (pointer: PointerEvent): BadgePosition => ({
      x: Math.round(startPosition.x + pointer.screenX - startMouse.x),
      y: Math.round(startPosition.y + pointer.screenY - startMouse.y),
    });

    const onMove = (pointer: PointerEvent) => {
      latest = calculate(pointer);
      if (scheduled) return;
      scheduled = true;
      requestAnimationFrame(() => {
        scheduled = false;
        setPosition(latest);
        moveBadge(latest).catch(() => undefined);
      });
    };

    const onUp = (pointer: PointerEvent) => {
      latest = calculate(pointer);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      setPosition(latest);
      setBadgePosition(latest).catch(() => undefined);
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }

  return (
    <div
      className={`badge-shell ${tone}`}
      onPointerDown={dragBadge}
      title="Drag to move. Current and weekly Codex limits."
    >
      <div className={`badge-activity ${refreshing ? "active" : ""}`} aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      <div className="badge-reset" title={resetAt ? `Reset in ${formatCountdown(resetAt)}` : "Reset unavailable"}>
        <i style={{ width: `${Number.isFinite(resetProgress) ? resetProgress : 0}%` }} />
      </div>
      <div className="badge-charts" aria-label="Codex current and weekly usage charts">
        <MiniUsage label="Current" percent={currentPercent} />
        <MiniUsage label="Weekly" percent={weeklyPercent} />
      </div>
    </div>
  );
}

function MiniUsage({ label, percent }: { label: string; percent: number }) {
  const value = Number.isFinite(percent) ? Math.max(0, Math.min(100, percent)) : 0;
  return (
    <div className="badge-chart" title={`${label}: ${Math.round(value)}%`}>
      <header>
        <span>{label}</span>
        <strong>{Number.isFinite(percent) ? `${Math.round(value)}%` : "--"}</strong>
      </header>
      <div className="badge-chart-track">
        <i style={{ width: `${value}%` }} />
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("badge-root") as HTMLElement).render(
  <React.StrictMode>
    <Badge />
  </React.StrictMode>,
);
