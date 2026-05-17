import type { MonitorSnapshot } from "./types";

export function formatNumber(value: number): string {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(value);
}

export function formatCompact(value: number): string {
  return new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

export function maxBucketPercent(snapshot: MonitorSnapshot | null): number {
  if (!snapshot || snapshot.buckets.length === 0) return Number.NaN;
  return Math.max(...snapshot.buckets.map((bucket) => bucket.usedPercent));
}

export function currentLimitPercent(snapshot: MonitorSnapshot | null): number {
  return snapshot?.buckets[0]?.usedPercent ?? Number.NaN;
}

export function weeklyLimitPercent(snapshot: MonitorSnapshot | null): number {
  const primary = snapshot?.buckets[0];
  if (typeof primary?.secondaryUsedPercent === "number") {
    return primary.secondaryUsedPercent;
  }

  const weeklyBucket = snapshot?.buckets.find(
    (bucket) => (bucket.windowDurationMins ?? 0) >= 10_080,
  );
  return weeklyBucket?.usedPercent ?? Number.NaN;
}

export function weeklyLimitResetAt(snapshot: MonitorSnapshot | null): number | null {
  const primary = snapshot?.buckets[0];
  if (typeof primary?.secondaryResetsAt === "number") {
    return primary.secondaryResetsAt;
  }

  const weeklyBucket = snapshot?.buckets.find(
    (bucket) => (bucket.windowDurationMins ?? 0) >= 10_080,
  );
  return weeklyBucket?.resetsAt ?? null;
}

export function currentLimitResetAt(snapshot: MonitorSnapshot | null): number | null {
  return snapshot?.buckets[0]?.resetsAt ?? null;
}

export function resetWindowProgress(
  resetAt: number | null | undefined,
  nowMs = Date.now(),
  windowHours = 5,
): number {
  if (typeof resetAt !== "number" || !Number.isFinite(resetAt)) return Number.NaN;
  const windowMs = windowHours * 60 * 60 * 1000;
  const resetMs = resetAt * 1000;
  const startMs = resetMs - windowMs;
  return Math.max(0, Math.min(100, ((nowMs - startMs) / windowMs) * 100));
}

export function statusTone(percent: number): "ok" | "warn" | "danger" | "muted" {
  if (!Number.isFinite(percent)) return "muted";
  if (percent >= 90) return "danger";
  if (percent >= 70) return "warn";
  return "ok";
}

export function formatCountdown(timestamp: number): string {
  const seconds = Math.max(0, Math.round(timestamp - Date.now() / 1000));
  if (seconds === 0) return "resetting";
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  return `${Math.round(seconds / 3600)}h`;
}

export function formatDateTime(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1000));
}

export function truncateTitle(title: string | null | undefined, max = 74): string {
  const clean = (title ?? "Untitled").trim() || "Untitled";
  if (clean.length <= max) return clean;
  if (max <= 3) return clean.slice(0, max);
  return `${clean.slice(0, max - 3)}...`;
}
