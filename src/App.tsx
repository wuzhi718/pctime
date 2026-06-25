import {
  Activity,
  BarChart3,
  CalendarDays,
  Clock,
  Database,
  Download,
  ExternalLink,
  Gauge,
  HardDrive,
  Languages,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  PieChart,
  Power,
  RefreshCcw,
  Search,
  Settings,
  Sun,
  Table2,
  TimerReset,
  type LucideIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./App.css";

type ViewId = "overview" | "activity" | "settings";
type Locale = "zh-CN" | "en-US";
type Theme = "dark" | "light";
type RangePreset = "day" | "week" | "month" | "year" | "custom";

type RangePayload = { preset: RangePreset; start_ms?: number; end_ms?: number };
type RangeInfo = { preset: string; start_ms: number; end_ms: number; label: string; bucket: string };
type MetricCard = { label: string; value_seconds: number; helper: string };
type CategorySummary = { category: string; seconds: number; focus_seconds: number; share: number; sample_count: number };
type AppSummary = { app_name: string; category: string; seconds: number; focus_seconds: number; share: number; last_seen: string | null };
type WindowSummary = { app_name: string; window_title: string; category: string; seconds: number; focus_seconds: number; share: number };
type TimelineApp = { app_name: string; category: string; seconds: number };
type TimelinePoint = { hour: string; active_seconds: number; idle_seconds: number; top_apps: TimelineApp[] };
type LiveWindow = { app_name: string; window_title: string; category: string; visible_area: number; visible_share: number; focused: boolean };
type DonutSegment = { key: string; label: string; valueLabel: string; shareLabel: string; color: string; share: number };
type TooltipLine = { color?: string; label: string; value: string };
type FloatingTooltipData = { color?: string; lines?: TooltipLine[]; primary: string; secondary?: string; title: string; x: number; y: number };
type TooltipControls = { hideTooltip: () => void; showTooltip: (tooltip: FloatingTooltipData) => void };
type GitHubReleaseAsset = { browser_download_url: string; name: string };
type GitHubRelease = { assets: GitHubReleaseAsset[]; body?: string; html_url: string; tag_name: string };
type UpdateState = {
  assetName: string | null;
  assetUrl: string | null;
  available: boolean | null;
  checkedAt: string | null;
  checking: boolean;
  error: string | null;
  latestVersion: string | null;
  releaseUrl: string | null;
};
type CaptureHealth = {
  monitoring: boolean;
  database_path: string;
  storage_location: string;
  database_size_bytes: number;
  estimated_daily_bytes: number;
  total_rows: number;
  samples_today: number;
  last_capture_at: string | null;
  idle_threshold_seconds: number;
  sample_interval_ms: number;
};
type Dashboard = {
  generated_at: string;
  range: RangeInfo;
  active_seconds: number;
  idle_seconds: number;
  focus_seconds: number;
  unclassified_seconds: number;
  cards: MetricCard[];
  categories: CategorySummary[];
  apps: AppSummary[];
  windows: WindowSummary[];
  timeline: TimelinePoint[];
  live_windows: LiveWindow[];
  health: CaptureHealth;
};

const copy = {
  "zh-CN": {
    brandSubtitle: "可见时间追踪",
    nav: {
      overview: "总览",
      activity: "分析",
      settings: "设置",
    },
    titles: {
      overview: "桌面活动",
      activity: "时间分配",
      settings: "偏好设置",
    },
    panels: {
      categoryMix: "分类占比",
      timeline: "时间趋势",
      applications: "应用明细",
      topApps: "高频应用",
      appearance: "外观",
      startup: "系统",
      updates: "软件更新",
      storage: "存储与性能",
    },
    table: {
      application: "应用",
      category: "分类",
      visible: "可见时间",
      focus: "焦点时间",
      share: "占比",
      last: "最后记录",
      samples: "样本",
    },
    cards: {
      "Visible time": ["可见时间", "分屏窗口会按实际可见面积计入"],
      "Focused time": ["焦点时间", "传统前台窗口统计"],
      "Idle time": ["离开时间", "超过 5 分钟无输入后计入"],
      "Needs rules": ["待分类", "还没有匹配规则的可见时间"],
    },
    categories: {
      Development: "开发",
      "AI Work": "AI 工作",
      Browser: "浏览器",
      Research: "研究",
      Communication: "沟通",
      Media: "媒体",
      Games: "游戏",
      Documents: "文档",
      Design: "设计",
      Creative: "创作",
      Productivity: "效率",
      Meetings: "会议",
      Cloud: "云盘",
      Finance: "金融",
      Utilities: "工具",
      System: "系统",
      PCTime: "PCTime",
      Idle: "离开",
      Other: "其它",
      Unclassified: "未分类",
    },
    range: {
      label: "时间范围",
      day: "日",
      week: "周",
      month: "月",
      year: "年",
      custom: "自定义",
      start: "开始",
      end: "结束",
    },
    settings: {
      language: "语言",
      theme: "主题",
      dark: "黑色",
      light: "白色",
      startup: "开机自启动",
      startupHint: "登录 Windows 后自动启动 PCTime。",
      closeToTray: "关闭按钮放到后台",
      closeToTrayHint: "开启后点击关闭会隐藏到 Windows 托盘，点击托盘图标可恢复；关闭后点击关闭会直接退出程序。",
      screenOff: "电脑只是息屏但没有睡眠时，PCTime 仍会运行；超过空闲阈值后统计为离开时间。电脑睡眠/休眠时不会采样，也不会把睡眠时长算进任何应用。",
      performanceNote: "默认每秒采样一次，只写入很小的 SQLite 文本记录；正常桌面使用时 CPU 占用应非常低。",
      storageNote: "数据库优先放在软件安装目录的 pctime-data 文件夹，写入失败才回退到用户 AppData。",
    },
    updates: {
      autoNote: "PCTime 会每天自动检查一次 GitHub Releases，不需要自建服务器，也不会影响本地采集。",
      available: "发现新版本",
      availableNote: "GitHub Release 上有新版本，可以下载 Windows 安装包。",
      checkNow: "检查更新",
      checkedAt: "上次检查",
      checking: "检查中",
      current: "已是最新版本",
      currentNote: "当前版本已经是 GitHub Release 上的最新版本。",
      currentVersion: "当前版本",
      downloadWindows: "下载 Windows x64",
      failed: "检查失败",
      latestVersion: "最新版本",
      openRelease: "打开 Release",
      unknown: "未知",
    },
    status: {
      database: "数据库",
      generated: "更新时间",
      idleThreshold: "空闲阈值",
      sampleInterval: "采样间隔",
      storageLocation: "存储位置",
      installDir: "安装目录",
      fallbackDir: "AppData 回退",
      dbSize: "数据库大小",
      dailyEstimate: "今日预计写入",
      rows: "总记录",
      samplesToday: "今日样本",
    },
    actions: {
      refresh: "刷新",
      filterApps: "搜索应用或分类",
      collapseSidebar: "收起侧边栏",
      expandSidebar: "展开侧边栏",
    },
    empty: {
      noActivity: "还没有记录到活动",
      noApps: "没有匹配的应用",
      unavailable: "面板暂不可用",
      loading: "正在启动本地追踪器...",
    },
  },
  "en-US": {
    brandSubtitle: "Visible time tracker",
    nav: {
      overview: "Overview",
      activity: "Activity",
      settings: "Settings",
    },
    titles: {
      overview: "Desktop activity",
      activity: "Time distribution",
      settings: "Preferences",
    },
    panels: {
      categoryMix: "Category mix",
      timeline: "Time trend",
      applications: "Application details",
      topApps: "Top apps",
      appearance: "Appearance",
      startup: "System",
      updates: "Software updates",
      storage: "Storage and performance",
    },
    table: {
      application: "Application",
      category: "Category",
      visible: "Visible",
      focus: "Focus",
      share: "Share",
      last: "Last",
      samples: "Samples",
    },
    cards: {
      "Visible time": ["Visible time", "Split-screen windows are weighted by visible area"],
      "Focused time": ["Focused time", "Traditional foreground-window time"],
      "Idle time": ["Idle time", "No input for 5 minutes"],
      "Needs rules": ["Needs rules", "Unclassified visible time"],
    },
    categories: {
      Development: "Development",
      "AI Work": "AI work",
      Browser: "Browser",
      Research: "Research",
      Communication: "Communication",
      Media: "Media",
      Games: "Games",
      Documents: "Documents",
      Design: "Design",
      Creative: "Creative",
      Productivity: "Productivity",
      Meetings: "Meetings",
      Cloud: "Cloud",
      Finance: "Finance",
      Utilities: "Utilities",
      System: "System",
      PCTime: "PCTime",
      Idle: "Idle",
      Other: "Other",
      Unclassified: "Unclassified",
    },
    range: {
      label: "Range",
      day: "Day",
      week: "Week",
      month: "Month",
      year: "Year",
      custom: "Custom",
      start: "Start",
      end: "End",
    },
    settings: {
      language: "Language",
      theme: "Theme",
      dark: "Dark",
      light: "Light",
      startup: "Start at login",
      startupHint: "Launch PCTime automatically after signing in to Windows.",
      closeToTray: "Close to background",
      closeToTrayHint: "When enabled, the close button hides PCTime to the Windows tray. Turn it off to exit the app when closing.",
      screenOff: "If the display turns off but the PC stays awake, PCTime keeps running; after the idle threshold it is recorded as idle time. During sleep or hibernation, sampling stops and the sleep gap is not counted for any app.",
      performanceNote: "The collector samples once per second and writes small SQLite text rows, so CPU use should stay very low in normal desktop work.",
      storageNote: "The database is stored in a pctime-data folder next to the app when possible, then falls back to user AppData if that location is not writable.",
    },
    updates: {
      autoNote: "PCTime checks GitHub Releases once per day. No custom server is required, and failed checks never affect local tracking.",
      available: "Update available",
      availableNote: "A newer GitHub Release is available. Download the Windows installer when you are ready.",
      checkNow: "Check for updates",
      checkedAt: "Last checked",
      checking: "Checking",
      current: "Up to date",
      currentNote: "This version matches the latest GitHub Release.",
      currentVersion: "Current version",
      downloadWindows: "Download Windows x64",
      failed: "Check failed",
      latestVersion: "Latest version",
      openRelease: "Open Release",
      unknown: "Unknown",
    },
    status: {
      database: "Database",
      generated: "Updated",
      idleThreshold: "Idle threshold",
      sampleInterval: "Sample interval",
      storageLocation: "Location",
      installDir: "Install directory",
      fallbackDir: "AppData fallback",
      dbSize: "Database size",
      dailyEstimate: "Daily estimate",
      rows: "Rows",
      samplesToday: "Today samples",
    },
    actions: {
      refresh: "Refresh",
      filterApps: "Search apps or categories",
      collapseSidebar: "Collapse sidebar",
      expandSidebar: "Expand sidebar",
    },
    empty: {
      noActivity: "No activity recorded yet",
      noApps: "No matching applications",
      unavailable: "Dashboard unavailable",
      loading: "Starting local tracker...",
    },
  },
} as const;

type UiCopy = (typeof copy)[Locale];

const UPDATE_ENDPOINT = "https://api.github.com/repos/wuzhi718/pctime/releases/latest";
const UPDATE_CHECK_KEY = "pctime-last-update-check";
const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1_000;
const emptyUpdateState: UpdateState = {
  assetName: null,
  assetUrl: null,
  available: null,
  checkedAt: null,
  checking: false,
  error: null,
  latestVersion: null,
  releaseUrl: null,
};

const cardIcons: Record<string, LucideIcon> = {
  "Visible time": Activity,
  "Focused time": Gauge,
  "Idle time": TimerReset,
  "Needs rules": Search,
};

const categoryColors: Record<string, string> = {
  Development: "#2f6fed",
  "AI Work": "#8f5bd5",
  Browser: "#16a085",
  Research: "#0f8fb3",
  Communication: "#d85b70",
  Media: "#e0a328",
  Games: "#dd6b20",
  Documents: "#5f7c36",
  Design: "#ec4899",
  Creative: "#f97316",
  Productivity: "#4f46e5",
  Meetings: "#06b6d4",
  Cloud: "#0284c7",
  Finance: "#16a34a",
  Utilities: "#64748b",
  System: "#6b7280",
  PCTime: "#14a38b",
  Idle: "#9ca3af",
  Other: "#94a3b8",
  Unclassified: "#b45309",
};

const navItems: Array<{ id: ViewId; icon: LucideIcon }> = [
  { id: "overview", icon: BarChart3 },
  { id: "activity", icon: Table2 },
  { id: "settings", icon: Settings },
];

const rangePresets: RangePreset[] = ["day", "week", "month", "year", "custom"];

function App() {
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [view, setView] = useState<ViewId>("overview");
  const [locale, setLocale] = useState<Locale>(() => readStorage("pctime-locale", "zh-CN"));
  const [theme, setTheme] = useState<Theme>(() => readStorage("pctime-theme", "dark"));
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem("pctime-sidebar") === "collapsed");
  const [rangePreset, setRangePreset] = useState<RangePreset>(() => readStorage("pctime-range", "day"));
  const [customStart, setCustomStart] = useState(() => toLocalInput(startOfToday()));
  const [customEnd, setCustomEnd] = useState(() => toLocalInput(new Date()));
  const [appVersion, setAppVersion] = useState("0.1.2");
  const [startupEnabled, setStartupEnabled] = useState<boolean | null>(null);
  const [closeToTray, setCloseToTray] = useState<boolean | null>(null);
  const [updateState, setUpdateState] = useState<UpdateState>(emptyUpdateState);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [floatingTooltip, setFloatingTooltip] = useState<FloatingTooltipData | null>(null);
  const [query, setQuery] = useState("");
  const t = copy[locale];

  const range = useMemo<RangePayload>(() => {
    if (rangePreset !== "custom") return { preset: rangePreset };
    return {
      preset: "custom",
      start_ms: customStart ? new Date(customStart).getTime() : undefined,
      end_ms: customEnd ? new Date(customEnd).getTime() : undefined,
    };
  }, [customEnd, customStart, rangePreset]);

  const loadDashboard = useCallback(async () => {
    try {
      setError(null);
      const next = await appInvoke<Dashboard>("get_dashboard", { range });
      setDashboard(next);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [range]);

  const checkForUpdates = useCallback(async (silent = false) => {
    setUpdateState((current) => ({ ...current, checking: true, error: null }));

    try {
      const latest = await fetchLatestRelease(appVersion);
      const checkedAt = new Date().toISOString();
      const next = { ...latest, checkedAt, checking: false, error: null };
      setUpdateState(next);
      localStorage.setItem(UPDATE_CHECK_KEY, String(Date.now()));
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setUpdateState((current) => ({
        ...current,
        available: silent ? current.available : null,
        checkedAt: new Date().toISOString(),
        checking: false,
        error: message,
      }));
    }
  }, [appVersion]);

  useEffect(() => {
    void loadDashboard();
    const timer = window.setInterval(() => void loadDashboard(), 5_000);
    return () => window.clearInterval(timer);
  }, [loadDashboard]);

  useEffect(() => {
    void appInvoke<string>("get_app_version")
      .then(setAppVersion)
      .catch(() => setAppVersion("0.1.2"));
  }, []);

  useEffect(() => {
    const lastCheck = Number(localStorage.getItem(UPDATE_CHECK_KEY) ?? "0");
    if (Date.now() - lastCheck < UPDATE_CHECK_INTERVAL_MS) return;
    void checkForUpdates(true);
  }, [checkForUpdates]);

  useEffect(() => localStorage.setItem("pctime-locale", locale), [locale]);
  useEffect(() => localStorage.setItem("pctime-theme", theme), [theme]);
  useEffect(() => localStorage.setItem("pctime-range", rangePreset), [rangePreset]);
  useEffect(() => localStorage.setItem("pctime-sidebar", sidebarCollapsed ? "collapsed" : "expanded"), [sidebarCollapsed]);

  useEffect(() => {
    void appInvoke<boolean>("get_startup_enabled")
      .then(setStartupEnabled)
      .catch(() => setStartupEnabled(false));
  }, []);

  useEffect(() => {
    void appInvoke<boolean>("get_close_to_tray")
      .then(setCloseToTray)
      .catch(() => setCloseToTray(true));
  }, []);

  const filteredApps = useMemo(() => {
    const value = query.trim().toLowerCase();
    if (!dashboard || !value) return dashboard?.apps ?? [];
    return dashboard.apps.filter(
      (app) =>
        app.app_name.toLowerCase().includes(value) ||
        app.category.toLowerCase().includes(value) ||
        categoryLabel(app.category, t).toLowerCase().includes(value),
    );
  }, [dashboard, query, t]);

  const showTooltip = useCallback((tooltip: FloatingTooltipData) => setFloatingTooltip(tooltip), []);
  const hideTooltip = useCallback(() => setFloatingTooltip(null), []);

  async function refreshNow() {
    setRefreshing(true);
    await appInvoke("record_now");
    await loadDashboard();
  }

  async function changeStartup(enabled: boolean) {
    setStartupEnabled(enabled);
    try {
      const actual = await appInvoke<boolean>("set_startup_enabled", { enabled });
      setStartupEnabled(actual);
    } catch (caught) {
      setStartupEnabled(!enabled);
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  async function changeCloseToTray(enabled: boolean) {
    setCloseToTray(enabled);
    try {
      const actual = await appInvoke<boolean>("set_close_to_tray", { enabled });
      setCloseToTray(actual);
    } catch (caught) {
      setCloseToTray(!enabled);
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  if (loading && !dashboard) {
    return (
      <main className="loading-screen" data-theme={theme}>
        <div className="loading-mark"><Activity size={28} /></div>
        <div>
          <h1>PCTime</h1>
          <p>{t.empty.loading}</p>
        </div>
      </main>
    );
  }

  const showRange = view !== "settings";

  return (
    <main className={sidebarCollapsed ? "app-shell sidebar-collapsed" : "app-shell"} data-theme={theme}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><Activity size={22} /></div>
          <div className="brand-copy">
            <strong>PCTime</strong>
            <span>{t.brandSubtitle}</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                className={view === item.id ? "nav-item active" : "nav-item"}
                key={item.id}
                type="button"
                onClick={() => setView(item.id)}
                title={t.nav[item.id]}
              >
                <Icon size={18} />
                <span>{t.nav[item.id]}</span>
              </button>
            );
          })}
        </nav>

        <button
          className="sidebar-toggle"
          type="button"
          onClick={() => setSidebarCollapsed((value) => !value)}
          title={sidebarCollapsed ? t.actions.expandSidebar : t.actions.collapseSidebar}
        >
          {sidebarCollapsed ? <PanelLeftOpen size={18} /> : <PanelLeftClose size={18} />}
          <span>{sidebarCollapsed ? t.actions.expandSidebar : t.actions.collapseSidebar}</span>
        </button>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div className="title-stack">
            <p className="eyebrow">{showRange ? dashboard?.range.label : dashboard?.generated_at ?? t.status.generated}</p>
            <h1>{t.titles[view]}</h1>
          </div>

          <div className="topbar-actions">
            {showRange ? (
              <RangeControl
                customEnd={customEnd}
                customStart={customStart}
                locale={locale}
                preset={rangePreset}
                setCustomEnd={setCustomEnd}
                setCustomStart={setCustomStart}
                setPreset={setRangePreset}
                t={t}
              />
            ) : null}
            <button className="icon-button" type="button" onClick={refreshNow} title={t.actions.refresh}>
              <RefreshCcw className={refreshing ? "spin" : undefined} size={18} />
            </button>
          </div>
        </header>

        {error ? <div className="error-banner">{error}</div> : null}

        <section className="page-content">
          {dashboard ? (
            <ActivePage
              dashboard={dashboard}
              closeToTray={closeToTray}
              filteredApps={filteredApps}
              locale={locale}
              appVersion={appVersion}
              query={query}
              setLocale={setLocale}
              setCloseToTray={changeCloseToTray}
              setQuery={setQuery}
              setStartupEnabled={changeStartup}
              setTheme={setTheme}
              showTooltip={showTooltip}
              hideTooltip={hideTooltip}
              startupEnabled={startupEnabled}
              t={t}
              theme={theme}
              updateState={updateState}
              checkForUpdates={checkForUpdates}
              view={view}
            />
          ) : (
            <EmptyState label={t.empty.unavailable} />
          )}
        </section>
      </section>
      <div className="floating-layer">
        {floatingTooltip ? <FloatingTooltip {...floatingTooltip} /> : null}
      </div>
    </main>
  );
}

function ActivePage({
  appVersion,
  checkForUpdates,
  dashboard,
  closeToTray,
  filteredApps,
  locale,
  query,
  setLocale,
  setCloseToTray,
  setQuery,
  setStartupEnabled,
  setTheme,
  showTooltip,
  hideTooltip,
  startupEnabled,
  t,
  theme,
  updateState,
  view,
}: {
  appVersion: string;
  checkForUpdates: (silent?: boolean) => Promise<void>;
  dashboard: Dashboard;
  closeToTray: boolean | null;
  filteredApps: AppSummary[];
  locale: Locale;
  query: string;
  setLocale: (value: Locale) => void;
  setCloseToTray: (enabled: boolean) => void;
  setQuery: (value: string) => void;
  setStartupEnabled: (enabled: boolean) => void;
  setTheme: (value: Theme) => void;
  showTooltip: (tooltip: FloatingTooltipData) => void;
  hideTooltip: () => void;
  startupEnabled: boolean | null;
  t: UiCopy;
  theme: Theme;
  updateState: UpdateState;
  view: ViewId;
}) {
  if (view === "activity") {
    return (
      <div className="page-grid activity-page">
        <Panel title={t.panels.categoryMix} action={<Clock size={18} />}>
          <CategoryList categories={dashboard.categories} t={t} />
        </Panel>
        <Panel
          title={t.panels.applications}
          className="fill-panel"
          action={<SearchBox value={query} onChange={setQuery} placeholder={t.actions.filterApps} />}
        >
          <AppTable apps={filteredApps} t={t} />
        </Panel>
      </div>
    );
  }

  if (view === "settings") {
    return (
      <div className="page-grid settings-page">
        <Panel title={t.panels.appearance} action={<Languages size={18} />}>
          <div className="settings-stack">
            <SettingBlock label={t.settings.language}>
              <SegmentedLanguage locale={locale} setLocale={setLocale} />
            </SettingBlock>
            <SettingBlock label={t.settings.theme}>
              <SegmentedTheme setTheme={setTheme} t={t} theme={theme} />
            </SettingBlock>
          </div>
        </Panel>

        <Panel title={t.panels.startup} action={<Power size={18} />}>
          <div className="settings-stack">
            <SwitchRow
              checked={Boolean(startupEnabled)}
              disabled={startupEnabled === null}
              label={t.settings.startup}
              note={t.settings.startupHint}
              onChange={setStartupEnabled}
            />
            <SwitchRow
              checked={Boolean(closeToTray)}
              disabled={closeToTray === null}
              label={t.settings.closeToTray}
              note={t.settings.closeToTrayHint}
              onChange={setCloseToTray}
            />
            <InfoNote icon={<Power size={18} />} text={t.settings.screenOff} />
          </div>
        </Panel>

        <Panel title={t.panels.updates} action={<RefreshCcw size={18} />}>
          <UpdatePanel appVersion={appVersion} onCheck={checkForUpdates} t={t} updateState={updateState} />
        </Panel>

        <Panel title={t.panels.storage} action={<HardDrive size={18} />}>
          <div className="settings-grid">
            <InfoItem label={t.status.storageLocation} value={storageLocationLabel(dashboard.health.storage_location, t)} />
            <InfoItem label={t.status.database} value={dashboard.health.database_path} />
            <InfoItem label={t.status.dbSize} value={formatBytes(dashboard.health.database_size_bytes)} />
            <InfoItem label={t.status.dailyEstimate} value={formatBytes(dashboard.health.estimated_daily_bytes)} />
            <InfoItem label={t.status.rows} value={dashboard.health.total_rows.toLocaleString()} />
            <InfoItem label={t.status.samplesToday} value={dashboard.health.samples_today.toLocaleString()} />
            <InfoItem label={t.status.sampleInterval} value={`${dashboard.health.sample_interval_ms / 1000}s`} />
            <InfoItem label={t.status.idleThreshold} value={formatDuration(dashboard.health.idle_threshold_seconds)} />
          </div>
          <div className="note-stack">
            <InfoNote icon={<Gauge size={18} />} text={t.settings.performanceNote} />
            <InfoNote icon={<Database size={18} />} text={t.settings.storageNote} />
          </div>
        </Panel>
      </div>
    );
  }

  return (
    <div className="page-grid overview-page">
      <section className="metric-grid">
        {dashboard.cards.map((card) => <MetricTile key={card.label} card={card} t={t} />)}
      </section>
      <Panel title={t.panels.timeline} action={<CalendarDays size={18} />}>
        <Timeline hideTooltip={hideTooltip} points={dashboard.timeline} showTooltip={showTooltip} t={t} />
      </Panel>
      <Panel title={t.panels.categoryMix} action={<PieChart size={18} />}>
        <CategoryDonut categories={dashboard.categories} hideTooltip={hideTooltip} showTooltip={showTooltip} t={t} />
      </Panel>
      <Panel title={t.panels.topApps} className="fill-panel" action={<BarChart3 size={18} />}>
        <TopAppsVisual apps={dashboard.apps.slice(0, 12)} hideTooltip={hideTooltip} showTooltip={showTooltip} t={t} />
      </Panel>
    </div>
  );
}

function RangeControl({
  customEnd,
  customStart,
  locale,
  preset,
  setCustomEnd,
  setCustomStart,
  setPreset,
  t,
}: {
  customEnd: string;
  customStart: string;
  locale: Locale;
  preset: RangePreset;
  setCustomEnd: (value: string) => void;
  setCustomStart: (value: string) => void;
  setPreset: (value: RangePreset) => void;
  t: UiCopy;
}) {
  return (
    <div className="range-control" aria-label={t.range.label}>
      <div className="segmented-control compact-segmented">
        {rangePresets.map((item) => (
          <button className={preset === item ? "selected" : undefined} key={item} type="button" onClick={() => setPreset(item)}>
            {t.range[item]}
          </button>
        ))}
      </div>
      {preset === "custom" ? (
        <div className="custom-range">
          <label>
            <span>{t.range.start}</span>
            <input lang={locale} type="datetime-local" value={customStart} onChange={(event) => setCustomStart(event.currentTarget.value)} />
          </label>
          <label>
            <span>{t.range.end}</span>
            <input lang={locale} type="datetime-local" value={customEnd} onChange={(event) => setCustomEnd(event.currentTarget.value)} />
          </label>
        </div>
      ) : null}
    </div>
  );
}

function MetricTile({ card, t }: { card: MetricCard; t: UiCopy }) {
  const Icon = cardIcons[card.label] ?? Activity;
  const localized = t.cards[card.label as keyof typeof t.cards] ?? [card.label, card.helper];
  return (
    <article className="metric-card">
      <div className="metric-icon"><Icon size={20} /></div>
      <div>
        <span>{localized[0]}</span>
        <strong>{formatDuration(card.value_seconds)}</strong>
        <small>{localized[1]}</small>
      </div>
    </article>
  );
}

function Panel({ title, action, children, className }: { title: string; action?: React.ReactNode; children: React.ReactNode; className?: string }) {
  return (
    <section className={["panel", className].filter(Boolean).join(" ")}>
      <div className="panel-header">
        <h2>{title}</h2>
        <div className="panel-action">{action}</div>
      </div>
      <div className="panel-body">{children}</div>
    </section>
  );
}

function CategoryList({ categories, t }: { categories: CategorySummary[]; t: UiCopy }) {
  if (!categories.length) return <EmptyState label={t.empty.noActivity} />;
  return (
    <div className="scroll-region category-list">
      {categories.map((category) => <CategoryRow key={category.category} category={category} t={t} />)}
    </div>
  );
}

function CategoryRow({ category, t }: { category: CategorySummary; t: UiCopy }) {
  const accent = categoryColor(category.category);
  const width = `${Math.max(category.share * 100, category.seconds > 0 ? 2 : 0)}%`;
  const style = { "--accent": accent } as CSSProperties;
  return (
    <div className="category-row" style={style}>
      <div className="category-main">
        <CategoryPill category={category.category} t={t} />
        <span>{formatDuration(category.seconds)}</span>
      </div>
      <div className="bar-track"><div className="bar-fill" style={{ width }} /></div>
      <div className="category-meta">
        <span>{formatPercent(category.share)}</span>
      </div>
    </div>
  );
}

function CategoryDonut({ categories, hideTooltip, showTooltip, t }: { categories: CategorySummary[]; t: UiCopy } & TooltipControls) {
  if (!categories.length) return <EmptyState label={t.empty.noActivity} />;

  const totalSeconds = categories.reduce((sum, category) => sum + category.seconds, 0);
  const segments = buildCategorySegments(categories, t);

  return (
    <div className="donut-layout">
      <RingDonut
        centerLabel={t.table.visible}
        centerValue={formatDuration(totalSeconds)}
        hideTooltip={hideTooltip}
        segments={segments}
        showTooltip={showTooltip}
      />
      <div className="donut-legend">
        {segments.map((segment) => (
          <div className="donut-legend-row" key={segment.key}>
            <span style={{ "--accent": segment.color } as CSSProperties} />
            <strong>{segment.label}</strong>
            <em>{segment.shareLabel}</em>
          </div>
        ))}
      </div>
    </div>
  );
}

function AppBarList({ apps, t }: { apps: AppSummary[]; t: UiCopy }) {
  if (!apps.length) return <EmptyState label={t.empty.noApps} />;
  const maxSeconds = Math.max(...apps.map((app) => app.seconds), 1);

  return (
    <div className="app-bar-list">
      {apps.map((app) => {
        const width = `${Math.max((app.seconds / maxSeconds) * 100, app.seconds > 0 ? 3 : 0)}%`;
        const style = { "--accent": categoryColor(app.category), "--bar-width": width } as CSSProperties;
        return (
          <div className="app-bar-row" key={`${app.app_name}-${app.category}`} style={style}>
            <div className="app-bar-head">
              <CategoryPill category={app.category} t={t} />
              <strong>{app.app_name}</strong>
              <span>{formatDuration(app.seconds)}</span>
              <em>{formatPercent(app.share)}</em>
            </div>
            <div className="app-bar-track"><span /></div>
          </div>
        );
      })}
    </div>
  );
}

function TopAppsVisual({ apps, hideTooltip, showTooltip, t }: { apps: AppSummary[]; t: UiCopy } & TooltipControls) {
  if (!apps.length) return <EmptyState label={t.empty.noApps} />;

  return (
    <div className="top-apps-layout">
      <AppBarList apps={apps} t={t} />
      <AppShareDonut apps={apps.slice(0, 8)} hideTooltip={hideTooltip} showTooltip={showTooltip} t={t} />
    </div>
  );
}

function AppShareDonut({ apps, hideTooltip, showTooltip, t }: { apps: AppSummary[]; t: UiCopy } & TooltipControls) {
  const totalSeconds = Math.max(apps.reduce((sum, app) => sum + app.seconds, 0), 1);
  const segments = buildAppSegments(apps);

  return (
    <div className="app-share-card">
      <RingDonut
        centerLabel={t.table.application}
        centerValue={String(apps.length)}
        hideTooltip={hideTooltip}
        segments={segments}
        showTooltip={showTooltip}
        compact
      />
      <div className="mini-donut-list">
        {apps.slice(0, 5).map((app, index) => (
          <div key={`${app.app_name}-${app.category}-share`}>
            <span style={{ "--accent": appColor(app, index) } as CSSProperties} />
            <strong>{app.app_name}</strong>
            <em>{formatPercent(app.seconds / totalSeconds)}</em>
          </div>
        ))}
      </div>
    </div>
  );
}

function RingDonut({
  centerLabel,
  centerValue,
  compact = false,
  hideTooltip,
  segments,
  showTooltip,
}: {
  centerLabel: string;
  centerValue: string;
  compact?: boolean;
  segments: DonutSegment[];
} & TooltipControls) {
  const [active, setActive] = useState<DonutSegment | null>(null);
  const radius = 46;
  const stroke = compact ? 12 : 14;
  const circumference = 2 * Math.PI * radius;
  const gap = segments.length > 1 ? 1.8 : 0;
  let cursor = 0;

  const showSegment = (segment: DonutSegment, event: { clientX: number; clientY: number }) => {
    setActive(segment);
    showTooltip({
      color: segment.color,
      primary: segment.valueLabel,
      secondary: segment.shareLabel,
      title: segment.label,
      x: event.clientX,
      y: event.clientY,
    });
  };

  const hideSegment = () => {
    setActive(null);
    hideTooltip();
  };

  return (
    <div
      className={compact ? "ring-donut compact-ring" : "ring-donut"}
      onMouseLeave={hideSegment}
      onPointerLeave={hideSegment}
    >
      <svg viewBox="0 0 120 120">
        <circle className="donut-base" cx="60" cy="60" r={radius} strokeWidth={stroke} />
        {segments.map((segment) => {
          const length = segment.share * circumference;
          const dash = Math.max(length - gap, 0);
          const offset = -cursor;
          cursor += length;

          return (
            <circle
              className="donut-segment-path"
              cx="60"
              cy="60"
              data-active={active?.key === segment.key ? "true" : undefined}
              key={segment.key}
              r={radius}
              role="img"
              stroke={segment.color}
              strokeDasharray={`${dash} ${circumference - dash}`}
              strokeDashoffset={offset}
              strokeWidth={stroke}
              style={{ "--segment-color": segment.color } as CSSProperties}
              tabIndex={0}
              transform="rotate(-90 60 60)"
              aria-label={`${segment.label}: ${segment.valueLabel}, ${segment.shareLabel}`}
              onBlur={hideSegment}
              onFocus={(event) => {
                const rect = event.currentTarget.getBoundingClientRect();
                setActive(segment);
                showSegment(segment, { clientX: rect.left + rect.width / 2, clientY: rect.top + rect.height / 2 });
              }}
              onMouseEnter={(event) => showSegment(segment, event)}
              onMouseMove={(event) => showSegment(segment, event)}
              onPointerEnter={(event) => showSegment(segment, event)}
              onPointerMove={(event) => showSegment(segment, event)}
            >
            </circle>
          );
        })}
      </svg>
      <div className="donut-center">
        <strong>{centerValue}</strong>
        <span>{centerLabel}</span>
      </div>
    </div>
  );
}

function Timeline({ hideTooltip, points, showTooltip, t }: { points: TimelinePoint[]; t: UiCopy } & TooltipControls) {
  if (!points.length) return <EmptyState label="No timeline" />;
  const maxSeconds = Math.max(...points.map((point) => point.active_seconds + point.idle_seconds), 1);
  const style = { "--timeline-count": points.length } as CSSProperties;

  const pointLabel = (point: TimelinePoint) => {
    const apps = point.top_apps
      .map((app) => `${app.app_name} - ${categoryLabel(app.category, t)} - ${formatDuration(app.seconds)} - ${point.active_seconds > 0 ? formatPercent(Math.min(app.seconds / point.active_seconds, 1)) : "0%"}`)
      .join("; ");
    return `${point.hour}. ${t.table.visible}: ${formatDuration(point.active_seconds)}${point.idle_seconds > 0 ? `. ${t.cards["Idle time"][0]}: ${formatDuration(point.idle_seconds)}` : ""}${apps ? `. Top apps: ${apps}` : ""}`;
  };

  const showPointTooltip = (point: TimelinePoint, event: { clientX: number; clientY: number }) => {
    showTooltip({
      title: point.hour,
      primary: `${t.table.visible}: ${formatDuration(point.active_seconds)}`,
      secondary: point.idle_seconds > 0 ? `${t.cards["Idle time"][0]}: ${formatDuration(point.idle_seconds)}` : undefined,
      x: event.clientX,
      y: event.clientY,
      lines: point.top_apps.map((app) => ({
        color: categoryColor(app.category),
        label: app.app_name,
        value: `${categoryLabel(app.category, t)} - ${formatDuration(app.seconds)} - ${point.active_seconds > 0 ? formatPercent(Math.min(app.seconds / point.active_seconds, 1)) : "0%"}`,
      })),
    });
  };

  return (
    <div className="timeline-viewport">
      <div className="timeline" style={style}>
        {points.map((point) => {
          const activeHeight = Math.max((point.active_seconds / maxSeconds) * 100, point.active_seconds > 0 ? 4 : 0);
          const idleHeight = Math.max((point.idle_seconds / maxSeconds) * 100, point.idle_seconds > 0 ? 4 : 0);
          return (
            <div
              className="timeline-column"
              key={point.hour}
              aria-label={pointLabel(point)}
              data-active={point.active_seconds}
              tabIndex={0}
              onBlur={hideTooltip}
              onFocus={(event) => {
                const rect = event.currentTarget.getBoundingClientRect();
                showPointTooltip(point, { clientX: rect.left + rect.width / 2, clientY: rect.top + 24 });
              }}
              onMouseEnter={(event) => showPointTooltip(point, event)}
              onMouseLeave={hideTooltip}
              onMouseMove={(event) => showPointTooltip(point, event)}
              onPointerEnter={(event) => showPointTooltip(point, event)}
              onPointerLeave={hideTooltip}
              onPointerMove={(event) => showPointTooltip(point, event)}
            >
              <div className="timeline-bars">
                <span className="timeline-idle" style={{ height: `${idleHeight}%` }} />
                <span className="timeline-active" style={{ height: `${activeHeight}%` }} />
              </div>
              <small>{point.hour}</small>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function FloatingTooltip({ color, lines = [], primary, secondary, title, x, y }: FloatingTooltipData) {
  const targetWidth = Math.min(lines.length ? 330 : 240, window.innerWidth - 24);
  const placeRight = x + 18 + targetWidth < window.innerWidth;
  const left = placeRight ? x + 16 : Math.max(12, x - targetWidth - 16);
  const top = Math.max(12, Math.min(y - 18, window.innerHeight - 188));
  const style = {
    "--tooltip-anchor": placeRight ? "left" : "right",
    "--tooltip-x": `${left}px`,
    "--tooltip-y": `${top}px`,
    "--tooltip-color": color ?? "var(--accent)",
  } as CSSProperties;

  return (
    <div className="chart-tooltip-floating" data-anchor={placeRight ? "left" : "right"} style={style}>
      <div className="tooltip-title">
        <span />
        <strong>{title}</strong>
      </div>
      <div className="tooltip-values">
        <span>{primary}</span>
        {secondary ? <em>{secondary}</em> : null}
      </div>
      {lines.length ? (
        <div className="tooltip-lines">
          {lines.map((line) => (
            <div key={`${line.label}-${line.value}`}>
              <span style={{ "--line-color": line.color ?? "var(--tooltip-color)" } as CSSProperties} />
              <strong>{line.label}</strong>
              <em>{line.value}</em>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function buildCategorySegments(categories: CategorySummary[], t: UiCopy): DonutSegment[] {
  const totalSeconds = Math.max(categories.reduce((sum, category) => sum + category.seconds, 0), 1);
  const top = categories.slice(0, 6);
  const topSeconds = top.reduce((sum, category) => sum + category.seconds, 0);
  const segments = top.map((category) => ({
    key: category.category,
    label: categoryLabel(category.category, t),
    valueLabel: formatDuration(category.seconds),
    shareLabel: formatPercent(category.seconds / totalSeconds),
    color: categoryColor(category.category),
    share: category.seconds / totalSeconds,
  }));
  const otherSeconds = categories.length > top.length ? Math.max(0, totalSeconds - topSeconds) : 0;

  if (otherSeconds > 0) {
    segments.push({
      key: "Other",
      label: categoryLabel("Other", t),
      valueLabel: formatDuration(otherSeconds),
      shareLabel: formatPercent(otherSeconds / totalSeconds),
      color: categoryColor("Other"),
      share: otherSeconds / totalSeconds,
    });
  }

  return normalizeSegments(segments);
}

function buildAppSegments(apps: AppSummary[]): DonutSegment[] {
  const totalSeconds = Math.max(apps.reduce((sum, app) => sum + app.seconds, 0), 1);
  return normalizeSegments(apps.map((app, index) => ({
    key: `${app.app_name}-${app.category}`,
    label: app.app_name,
    valueLabel: formatDuration(app.seconds),
    shareLabel: formatPercent(app.seconds / totalSeconds),
    color: appColor(app, index),
    share: app.seconds / totalSeconds,
  })));
}

function normalizeSegments(segments: DonutSegment[]) {
  const totalShare = segments.reduce((sum, segment) => sum + segment.share, 0);
  if (totalShare <= 0) return [];
  return segments.map((segment) => ({ ...segment, share: segment.share / totalShare }));
}

function appColor(app: AppSummary, index: number) {
  const palette = ["#2f6fed", "#d85b70", "#8f5bd5", "#16a085", "#e0a328", "#0f8fb3", "#f97316", "#64748b"];
  return categoryColors[app.category] ?? palette[index % palette.length];
}

function AppTable({ apps, t, compact = false }: { apps: AppSummary[]; t: UiCopy; compact?: boolean }) {
  if (!apps.length) return <EmptyState label={t.empty.noApps} />;
  return (
    <div className="table-wrap">
      <table className={compact ? "compact-table" : undefined}>
        <thead>
          <tr>
            <th>{t.table.application}</th>
            <th>{t.table.category}</th>
            <th>{t.table.visible}</th>
            <th>{t.table.focus}</th>
            <th>{t.table.share}</th>
            {!compact ? <th>{t.table.last}</th> : null}
          </tr>
        </thead>
        <tbody>
          {apps.map((app) => (
            <tr key={`${app.app_name}-${app.category}`}>
              <td><strong>{app.app_name}</strong></td>
              <td><CategoryPill category={app.category} t={t} /></td>
              <td>{formatDuration(app.seconds)}</td>
              <td>{formatDuration(app.focus_seconds)}</td>
              <td>{formatPercent(app.share)}</td>
              {!compact ? <td>{app.last_seen ?? "-"}</td> : null}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SearchBox({ value, onChange, placeholder }: { value: string; onChange: (value: string) => void; placeholder: string }) {
  return (
    <label className="search-box">
      <Search size={16} />
      <input value={value} onChange={(event) => onChange(event.currentTarget.value)} placeholder={placeholder} />
    </label>
  );
}

function SettingBlock({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <div className="setting-block">
      <span>{label}</span>
      {children}
    </div>
  );
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return <div className="info-item"><span>{label}</span><strong>{value}</strong></div>;
}

function InfoNote({ icon, text }: { icon: React.ReactNode; text: string }) {
  return <div className="info-note">{icon}<span>{text}</span></div>;
}

function UpdatePanel({ appVersion, onCheck, t, updateState }: { appVersion: string; onCheck: (silent?: boolean) => Promise<void>; t: UiCopy; updateState: UpdateState }) {
  const statusText = updateState.checking
    ? t.updates.checking
    : updateState.error
      ? t.updates.failed
      : updateState.available
        ? t.updates.available
        : updateState.available === false
          ? t.updates.current
          : t.updates.unknown;
  const note = updateState.error
    ? updateState.error
    : updateState.available
      ? t.updates.availableNote
      : updateState.available === false
        ? t.updates.currentNote
        : t.updates.autoNote;

  return (
    <div className="settings-stack">
      <div className={updateState.available ? "update-card available" : "update-card"}>
        <div className="update-version-grid">
          <InfoItem label={t.updates.currentVersion} value={`v${appVersion}`} />
          <InfoItem label={t.updates.latestVersion} value={updateState.latestVersion ? `v${updateState.latestVersion}` : t.updates.unknown} />
          <InfoItem label={t.updates.checkedAt} value={updateState.checkedAt ? formatDateTime(updateState.checkedAt) : t.updates.unknown} />
        </div>
        <div className="update-status">
          <strong>{statusText}</strong>
          <span>{note}</span>
        </div>
        <div className="update-actions">
          <button className="secondary-button" disabled={updateState.checking} type="button" onClick={() => void onCheck(false)}>
            <RefreshCcw className={updateState.checking ? "spin" : undefined} size={16} />
            {updateState.checking ? t.updates.checking : t.updates.checkNow}
          </button>
          {updateState.assetUrl ? (
            <button className="primary-button" type="button" onClick={() => void openExternal(updateState.assetUrl!)}>
              <Download size={16} />
              {t.updates.downloadWindows}
            </button>
          ) : null}
          {updateState.releaseUrl ? (
            <button className="secondary-button" type="button" onClick={() => void openExternal(updateState.releaseUrl!)}>
              <ExternalLink size={16} />
              {t.updates.openRelease}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function SwitchRow({ checked, disabled, label, note, onChange }: { checked: boolean; disabled?: boolean; label: string; note: string; onChange: (checked: boolean) => void }) {
  return (
    <label className={disabled ? "switch-row disabled" : "switch-row"}>
      <span>
        <strong>{label}</strong>
        <small>{note}</small>
      </span>
      <input checked={checked} disabled={disabled} type="checkbox" onChange={(event) => onChange(event.currentTarget.checked)} />
      <em />
    </label>
  );
}

function SegmentedLanguage({ locale, setLocale }: { locale: Locale; setLocale: (value: Locale) => void }) {
  return (
    <div className="segmented-control">
      <button className={locale === "zh-CN" ? "selected" : undefined} type="button" onClick={() => setLocale("zh-CN")}>中文</button>
      <button className={locale === "en-US" ? "selected" : undefined} type="button" onClick={() => setLocale("en-US")}>English</button>
    </div>
  );
}

function SegmentedTheme({ setTheme, t, theme }: { setTheme: (value: Theme) => void; t: UiCopy; theme: Theme }) {
  return (
    <div className="segmented-control">
      <button className={theme === "dark" ? "selected" : undefined} type="button" onClick={() => setTheme("dark")}><Moon size={15} />{t.settings.dark}</button>
      <button className={theme === "light" ? "selected" : undefined} type="button" onClick={() => setTheme("light")}><Sun size={15} />{t.settings.light}</button>
    </div>
  );
}

function CategoryPill({ category, t }: { category: string; t: UiCopy }) {
  const style = { "--accent": categoryColor(category) } as CSSProperties;
  return <span className="category-pill" style={style}>{categoryLabel(category, t)}</span>;
}

function EmptyState({ label }: { label: string }) {
  return <div className="empty-state">{label}</div>;
}

function categoryColor(category: string) {
  return categoryColors[category] ?? "#64748b";
}

function categoryLabel(category: string, t: UiCopy) {
  return t.categories[category as keyof typeof t.categories] ?? category;
}

function storageLocationLabel(location: string, t: UiCopy) {
  return location === "install_dir" ? t.status.installDir : t.status.fallbackDir;
}

async function fetchLatestRelease(currentVersion: string): Promise<Omit<UpdateState, "checkedAt" | "checking" | "error">> {
  const response = await fetch(UPDATE_ENDPOINT, {
    headers: { Accept: "application/vnd.github+json" },
  });

  if (!response.ok) {
    throw new Error(`GitHub update check failed: ${response.status}`);
  }

  const release = await response.json() as GitHubRelease;
  const latestVersion = normalizeVersion(release.tag_name);
  const asset = release.assets.find((item) => /windows.*x64.*\.msi$/i.test(item.name))
    ?? release.assets.find((item) => /\.msi$/i.test(item.name))
    ?? null;

  return {
    assetName: asset?.name ?? null,
    assetUrl: asset?.browser_download_url ?? null,
    available: compareVersions(latestVersion, normalizeVersion(currentVersion)) > 0,
    latestVersion,
    releaseUrl: release.html_url,
  };
}

function normalizeVersion(version: string) {
  return version.trim().replace(/^v/i, "").split("-")[0];
}

function compareVersions(left: string, right: string) {
  const leftParts = left.split(".").map((part) => Number.parseInt(part, 10) || 0);
  const rightParts = right.split(".").map((part) => Number.parseInt(part, 10) || 0);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const diff = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (diff !== 0) return diff;
  }

  return 0;
}

async function openExternal(url: string) {
  const hasTauri = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
  if (hasTauri) {
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

async function appInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const hasTauri = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
  if (hasTauri) return invoke<T>(command, args);
  return demoResponse(command, args) as T;
}

function demoResponse(command: string, args?: Record<string, unknown>) {
  if (command === "get_dashboard") return demoDashboard(args?.range as RangePayload | undefined);
  if (command === "get_app_version") return "0.1.2";
  if (command === "get_startup_enabled") return false;
  if (command === "set_startup_enabled") return Boolean(args?.enabled);
  if (command === "get_close_to_tray") return true;
  if (command === "set_close_to_tray") return Boolean(args?.enabled);
  return { captured_at: Date.now(), windows_recorded: 3, idle: false, idle_seconds: 0 };
}

function demoDashboard(range?: RangePayload): Dashboard {
  const pointCount = range?.preset === "year" ? 12 : range?.preset === "week" ? 7 : range?.preset === "month" ? 30 : 24;
  const timeline = Array.from({ length: pointCount }, (_, index) => ({
    hour: range?.preset === "year" ? `${String(index + 1).padStart(2, "0")}` : range?.preset === "month" ? `${String(index + 1).padStart(2, "0")}` : `${String(index).padStart(2, "0")}`,
    active_seconds: index > 1 && index < pointCount - 2 ? Math.round((Math.sin(index / 2) + 1.25) * 1_200) : 0,
    idle_seconds: index === 3 || index === 8 ? 600 : 0,
    top_apps: [
      { app_name: index % 2 === 0 ? "Codex.exe" : "chrome.exe", category: index % 2 === 0 ? "Development" : "Research", seconds: 900 },
      { app_name: "ChatGPT.exe", category: "AI Work", seconds: 420 },
    ],
  }));

  return {
    generated_at: new Date().toLocaleString(),
    range: {
      preset: range?.preset ?? "day",
      start_ms: Date.now() - 86_400_000,
      end_ms: Date.now(),
      label: "Demo",
      bucket: "hour",
    },
    active_seconds: 22_860,
    idle_seconds: 3_420,
    focus_seconds: 16_100,
    unclassified_seconds: 1_260,
    cards: [
      { label: "Visible time", value_seconds: 22_860, helper: "Split-screen time is weighted by visible area" },
      { label: "Focused time", value_seconds: 16_100, helper: "Traditional foreground-window time" },
      { label: "Idle time", value_seconds: 3_420, helper: "No input for 5 min" },
      { label: "Needs rules", value_seconds: 1_260, helper: "Unclassified visible time" },
    ],
    categories: [
      { category: "Development", seconds: 9_880, focus_seconds: 7_440, share: 0.43, sample_count: 5_120 },
      { category: "AI Work", seconds: 4_920, focus_seconds: 3_650, share: 0.22, sample_count: 2_430 },
      { category: "Research", seconds: 3_720, focus_seconds: 1_880, share: 0.16, sample_count: 1_810 },
      { category: "Communication", seconds: 2_100, focus_seconds: 1_560, share: 0.09, sample_count: 980 },
      { category: "Creative", seconds: 1_420, focus_seconds: 900, share: 0.06, sample_count: 540 },
      { category: "Unclassified", seconds: 1_260, focus_seconds: 760, share: 0.06, sample_count: 610 },
    ],
    apps: [
      { app_name: "Codex.exe", category: "Development", seconds: 6_240, focus_seconds: 5_200, share: 0.27, last_seen: "06-25 11:18" },
      { app_name: "chrome.exe", category: "Research", seconds: 5_430, focus_seconds: 2_210, share: 0.24, last_seen: "06-25 11:18" },
      { app_name: "Code.exe", category: "Development", seconds: 3_640, focus_seconds: 1_900, share: 0.16, last_seen: "06-25 11:17" },
      { app_name: "ChatGPT.exe", category: "AI Work", seconds: 2_480, focus_seconds: 1_990, share: 0.11, last_seen: "06-25 11:16" },
      { app_name: "WeChat.exe", category: "Communication", seconds: 1_960, focus_seconds: 1_520, share: 0.09, last_seen: "06-25 11:12" },
      { app_name: "Godot_v4.7-stable_win64.exe", category: "Development", seconds: 1_420, focus_seconds: 900, share: 0.06, last_seen: "06-25 11:10" },
    ],
    windows: [],
    timeline,
    live_windows: [],
    health: {
      monitoring: true,
      database_path: "pctime-data/pctime.sqlite3",
      storage_location: "install_dir",
      database_size_bytes: 345_088,
      estimated_daily_bytes: 1_900_000,
      total_rows: 11_380,
      samples_today: 11_380,
      last_capture_at: new Date().toTimeString().slice(0, 8),
      idle_threshold_seconds: 300,
      sample_interval_ms: 1_000,
    },
  };
}

function readStorage<T extends string>(key: string, fallback: T): T {
  const value = localStorage.getItem(key);
  return value ? (value as T) : fallback;
}

function startOfToday() {
  const date = new Date();
  date.setHours(0, 0, 0, 0);
  return date;
}

function toLocalInput(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function formatPercent(value: number) {
  return `${Math.round(value * 100)}%`;
}

function formatDuration(seconds: number) {
  const total = Math.max(0, Math.round(seconds));
  const hours = Math.floor(total / 3_600);
  const minutes = Math.floor((total % 3_600) / 60);
  const secs = total % 60;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${secs}s`;
  return `${secs}s`;
}

function formatBytes(bytes: number) {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return `${date.toLocaleDateString()} ${date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
}

export default App;
