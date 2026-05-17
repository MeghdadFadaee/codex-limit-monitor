import { describe, expect, it } from "vitest";
import {
  currentLimitPercent,
  formatCountdown,
  resetWindowProgress,
  statusTone,
  truncateTitle,
  weeklyLimitPercent,
  weeklyLimitResetAt,
} from "./utils";
import type { MonitorSnapshot } from "./types";

describe("statusTone", () => {
  it("maps percentages to tones", () => {
    expect(statusTone(Number.NaN)).toBe("muted");
    expect(statusTone(10)).toBe("ok");
    expect(statusTone(70)).toBe("warn");
    expect(statusTone(90)).toBe("danger");
  });
});

describe("formatCountdown", () => {
  it("formats future reset timestamps", () => {
    const future = Math.round(Date.now() / 1000) + 120;
    expect(formatCountdown(future)).toBe("2m");
  });
});

describe("limit helpers", () => {
  it("separates current and weekly app-server percentages", () => {
    const snapshot = {
      buckets: [
        {
          limitId: "codex",
          limitName: null,
          usedPercent: 8,
          windowDurationMins: 300,
          resetsAt: 100,
          secondaryUsedPercent: 44,
          secondaryResetsAt: 200,
          reachedType: null,
        },
      ],
    } as MonitorSnapshot;

    expect(currentLimitPercent(snapshot)).toBe(8);
    expect(weeklyLimitPercent(snapshot)).toBe(44);
    expect(weeklyLimitResetAt(snapshot)).toBe(200);
  });
});

describe("truncateTitle", () => {
  it("limits long titles", () => {
    expect(truncateTitle("abcdefghijklmnopqrstuvwxyz", 10)).toBe("abcdefg...");
  });
});

describe("resetWindowProgress", () => {
  it("fills based on elapsed time in the five hour reset window", () => {
    const resetAt = Date.UTC(2026, 0, 1, 5, 0, 0) / 1000;
    const nowMs = Date.UTC(2026, 0, 1, 1, 0, 0);

    expect(resetWindowProgress(resetAt, nowMs)).toBe(20);
  });
});
