export type RateLimitBucket = {
  limitId: string;
  limitName: string | null;
  usedPercent: number;
  windowDurationMins: number | null;
  resetsAt: number | null;
  secondaryUsedPercent: number | null;
  secondaryResetsAt: number | null;
  reachedType: string | null;
};

export type LiveStatus = {
  state: "online" | "fallback" | "offline" | string;
  detail: string | null;
};

export type TokenBreakdown = {
  input: number;
  output: number;
  cached: number;
  reasoning: number;
  tool: number;
  responses: number;
};

export type ThreadUsage = {
  id: string;
  title: string;
  tokensUsed: number;
  model: string | null;
  reasoningEffort: string | null;
  updatedAt: number;
  cwd: string | null;
};

export type ModelUsage = {
  model: string;
  tokens: number;
  threads: number;
};

export type UsageSummary = {
  totalTokens: number;
  todayTokens: number;
  weekTokens: number;
  threadCount: number;
  todayThreadCount: number;
  weekThreadCount: number;
  currentThread: ThreadUsage | null;
  recentThreads: ThreadUsage[];
  modelBreakdown: ModelUsage[];
  tokenBreakdown: TokenBreakdown;
  sourcePath: string | null;
  error: string | null;
};

export type MonitorSnapshot = {
  source: string;
  fetchedAt: number;
  liveStatus: LiveStatus;
  buckets: RateLimitBucket[];
  usageSummary: UsageSummary;
  currentThread: ThreadUsage | null;
  message: string | null;
};

export type UsagePoint = {
  label: string;
  tokens: number;
  threads: number;
};

export type UsageHistory = {
  range: string;
  points: UsagePoint[];
};

export type BadgePosition = {
  x: number;
  y: number;
};

export type AppSettings = {
  codexExecutable: string | null;
  codexHome: string;
  refreshIntervalSecs: number;
  badgeVisible: boolean;
  badgePosition: BadgePosition;
  privacyMode: boolean;
  thresholds: {
    warning: number;
    critical: number;
    exhausted: number;
  };
  notificationsMutedUntil: number | null;
};
