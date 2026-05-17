import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  BadgePosition,
  MonitorSnapshot,
  UsageHistory,
} from "./types";

export function getSnapshot(): Promise<MonitorSnapshot> {
  return invoke("get_snapshot");
}

export function refreshNow(): Promise<MonitorSnapshot> {
  return invoke("refresh_now");
}

export function getUsageHistory(range: string): Promise<UsageHistory> {
  return invoke("get_usage_history", { range });
}

export function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export function updateSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke("update_settings", { settings });
}

export function toggleBadge(visible: boolean): Promise<AppSettings> {
  return invoke("toggle_badge", { visible });
}

export function setBadgePosition(position: BadgePosition): Promise<AppSettings> {
  return invoke("set_badge_position", { position });
}

export function moveBadge(position: BadgePosition): Promise<void> {
  return invoke("move_badge", { position });
}

export function pauseNotifications(until: number | null): Promise<AppSettings> {
  return invoke("pause_notifications", { until });
}
