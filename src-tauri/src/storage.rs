use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local, TimeZone};
use rusqlite::{params, Connection};

use crate::classifier;
use crate::models::{
    AppSummary, CaptureHealth, CategorySummary, Dashboard, LiveWindow, MetricCard, RangeInfo,
    RangeQuery, TimelineApp, TimelinePoint, WindowSample, WindowSummary,
};

pub fn init_database(
    install_data_dir: &Path,
    fallback_dir: &Path,
) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
    match try_init_database(install_data_dir) {
        Ok(path) => Ok((path, "install_dir".to_string())),
        Err(_) => {
            let path = try_init_database(fallback_dir)?;
            Ok((path, "app_data_fallback".to_string()))
        }
    }
}

fn try_init_database(app_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    fs::create_dir_all(app_dir)?;
    let probe_path = app_dir.join(".pctime-write-test");
    fs::write(&probe_path, b"ok")?;
    let _ = fs::remove_file(probe_path);

    let db_path = app_dir.join("pctime.sqlite3");
    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            captured_at INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            app_name TEXT NOT NULL,
            window_title TEXT NOT NULL,
            process_path TEXT NOT NULL,
            pid INTEGER NOT NULL,
            category TEXT NOT NULL,
            visible_area INTEGER NOT NULL,
            visible_share REAL NOT NULL,
            weighted_ms INTEGER NOT NULL,
            focused INTEGER NOT NULL,
            focus_ms INTEGER NOT NULL,
            idle INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_samples_captured_at ON samples(captured_at);
        CREATE INDEX IF NOT EXISTS idx_samples_category ON samples(category);
        CREATE INDEX IF NOT EXISTS idx_samples_app ON samples(app_name);
        ",
    )?;

    Ok(db_path)
}

pub fn refresh_unclassified_categories(db_path: &Path) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let candidates = {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, app_name, window_title, process_path
                FROM samples
                WHERE category = 'Unclassified'
                ",
            )
            .map_err(|error| error.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        collect_rows(rows)?
    };

    if candidates.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|error| error.to_string())?;
    for (id, app_name, window_title, process_path) in candidates {
        let category = classifier::classify(&app_name, &window_title, &process_path);
        if category != "Unclassified" {
            tx.execute(
                "UPDATE samples SET category = ?1 WHERE id = ?2",
                params![category, id],
            )
            .map_err(|error| error.to_string())?;
        }
    }

    tx.commit().map_err(|error| error.to_string())
}

pub fn insert_samples(
    db_path: &Path,
    captured_at: i64,
    duration_ms: u64,
    idle: bool,
    _idle_seconds: u64,
    samples: &[WindowSample],
) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;

    if idle {
        tx.execute(
            "
            INSERT INTO samples (
                captured_at, duration_ms, app_name, window_title, process_path, pid,
                category, visible_area, visible_share, weighted_ms, focused, focus_ms, idle
            )
            VALUES (?1, ?2, 'Idle', 'Away from keyboard', '', 0, 'Idle', 0, 1.0, ?2, 0, 0, 1)
            ",
            params![captured_at, duration_ms as i64],
        )
        .map_err(|error| error.to_string())?;
    } else if samples.is_empty() {
        tx.execute(
            "
            INSERT INTO samples (
                captured_at, duration_ms, app_name, window_title, process_path, pid,
                category, visible_area, visible_share, weighted_ms, focused, focus_ms, idle
            )
            VALUES (?1, ?2, 'Desktop', 'No visible tracked windows', '', 0, 'System', 0, 1.0, ?2, 0, 0, 0)
            ",
            params![captured_at, duration_ms as i64],
        )
        .map_err(|error| error.to_string())?;
    } else {
        for sample in samples {
            let weighted_ms = (duration_ms as f64 * sample.visible_share).round() as i64;
            let focus_ms = if sample.focused {
                duration_ms as i64
            } else {
                0
            };

            tx.execute(
                "
                INSERT INTO samples (
                    captured_at, duration_ms, app_name, window_title, process_path, pid,
                    category, visible_area, visible_share, weighted_ms, focused, focus_ms, idle
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0)
                ",
                params![
                    captured_at,
                    duration_ms as i64,
                    sample.app_name,
                    sample.window_title,
                    sample.process_path,
                    sample.pid as i64,
                    sample.category,
                    sample.visible_area,
                    sample.visible_share,
                    weighted_ms,
                    i64::from(sample.focused),
                    focus_ms,
                ],
            )
            .map_err(|error| error.to_string())?;
        }
    }

    tx.commit().map_err(|error| error.to_string())
}

pub fn dashboard(
    db_path: &Path,
    storage_location: &str,
    monitoring: bool,
    idle_threshold_seconds: u64,
    range_query: Option<RangeQuery>,
    live_windows: Vec<LiveWindow>,
) -> Result<Dashboard, String> {
    let conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let range = resolve_range(range_query);
    let start_ms = range.start_ms;
    let end_ms = range.end_ms;
    let active_ms = sum_ms(&conn, start_ms, end_ms, "idle = 0")?;
    let idle_ms = sum_ms(&conn, start_ms, end_ms, "idle = 1")?;
    let focus_ms: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(focus_ms), 0) FROM samples WHERE captured_at >= ?1 AND captured_at < ?2 AND idle = 0",
            params![start_ms, end_ms],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let unclassified_ms: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(weighted_ms), 0) FROM samples WHERE captured_at >= ?1 AND captured_at < ?2 AND category = 'Unclassified'",
            params![start_ms, end_ms],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let (today_start_ms, today_end_ms) = today_range_ms();
    let samples_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM samples WHERE captured_at >= ?1 AND captured_at < ?2",
            params![today_start_ms, today_end_ms],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let total_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;

    let last_capture_at: Option<i64> = conn
        .query_row("SELECT MAX(captured_at) FROM samples", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(|error| error.to_string())?;

    let active_seconds = active_ms / 1_000;
    let idle_seconds = idle_ms / 1_000;
    let focus_seconds = focus_ms / 1_000;
    let unclassified_seconds = unclassified_ms / 1_000;
    let timeline_points = timeline(&conn, start_ms, end_ms, &range.bucket)?;

    Ok(Dashboard {
        generated_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        range,
        active_seconds,
        idle_seconds,
        focus_seconds,
        unclassified_seconds,
        cards: vec![
            MetricCard {
                label: "Visible time".to_string(),
                value_seconds: active_seconds,
                helper: "Split-screen time is weighted by visible area".to_string(),
            },
            MetricCard {
                label: "Focused time".to_string(),
                value_seconds: focus_seconds,
                helper: "Traditional foreground-window time".to_string(),
            },
            MetricCard {
                label: "Idle time".to_string(),
                value_seconds: idle_seconds,
                helper: format!("No input for {} min", idle_threshold_seconds / 60),
            },
            MetricCard {
                label: "Needs rules".to_string(),
                value_seconds: unclassified_seconds,
                helper: "Unclassified visible time".to_string(),
            },
        ],
        categories: category_summaries(&conn, start_ms, end_ms, active_ms)?,
        apps: app_summaries(&conn, start_ms, end_ms, active_ms)?,
        windows: window_summaries(&conn, start_ms, end_ms, active_ms)?,
        timeline: timeline_points,
        live_windows,
        health: CaptureHealth {
            monitoring,
            database_path: db_path.display().to_string(),
            storage_location: storage_location.to_string(),
            database_size_bytes: database_size_bytes(db_path),
            estimated_daily_bytes: estimated_daily_bytes(db_path, total_rows, samples_today),
            total_rows,
            samples_today,
            last_capture_at: last_capture_at.map(format_timestamp),
            idle_threshold_seconds,
            sample_interval_ms: 1_000,
        },
    })
}

fn database_size_bytes(db_path: &Path) -> u64 {
    let sqlite = fs::metadata(db_path).map(|meta| meta.len()).unwrap_or(0);
    let wal = fs::metadata(format!("{}-wal", db_path.display()))
        .map(|meta| meta.len())
        .unwrap_or(0);
    let shm = fs::metadata(format!("{}-shm", db_path.display()))
        .map(|meta| meta.len())
        .unwrap_or(0);

    sqlite + wal + shm
}

fn estimated_daily_bytes(db_path: &Path, total_rows: i64, samples_today: i64) -> u64 {
    if total_rows <= 0 || samples_today <= 0 {
        return 0;
    }

    let bytes_per_row = database_size_bytes(db_path) as f64 / total_rows as f64;
    (bytes_per_row * samples_today as f64).round() as u64
}

fn sum_ms(conn: &Connection, start_ms: i64, end_ms: i64, condition: &str) -> Result<i64, String> {
    let sql = format!(
        "SELECT COALESCE(SUM(weighted_ms), 0) FROM samples WHERE captured_at >= ?1 AND captured_at < ?2 AND {condition}"
    );

    conn.query_row(&sql, params![start_ms, end_ms], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn category_summaries(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    active_ms: i64,
) -> Result<Vec<CategorySummary>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT category, SUM(weighted_ms), SUM(focus_ms), COUNT(*)
            FROM samples
            WHERE captured_at >= ?1 AND captured_at < ?2 AND idle = 0
            GROUP BY category
            ORDER BY SUM(weighted_ms) DESC
            LIMIT 12
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![start_ms, end_ms], |row| {
            let ms: i64 = row.get(1)?;
            Ok(CategorySummary {
                category: row.get(0)?,
                seconds: ms / 1_000,
                focus_seconds: row.get::<_, i64>(2)? / 1_000,
                share: share(ms, active_ms),
                sample_count: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;

    collect_rows(rows)
}

fn app_summaries(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    active_ms: i64,
) -> Result<Vec<AppSummary>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT app_name, category, SUM(weighted_ms), SUM(focus_ms), MAX(captured_at)
            FROM samples
            WHERE captured_at >= ?1 AND captured_at < ?2 AND idle = 0
            GROUP BY app_name, category
            ORDER BY SUM(weighted_ms) DESC
            LIMIT 16
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![start_ms, end_ms], |row| {
            let ms: i64 = row.get(2)?;
            let last_seen: Option<i64> = row.get(4)?;

            Ok(AppSummary {
                app_name: row.get(0)?,
                category: row.get(1)?,
                seconds: ms / 1_000,
                focus_seconds: row.get::<_, i64>(3)? / 1_000,
                share: share(ms, active_ms),
                last_seen: last_seen.map(format_timestamp),
            })
        })
        .map_err(|error| error.to_string())?;

    collect_rows(rows)
}

fn window_summaries(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    active_ms: i64,
) -> Result<Vec<WindowSummary>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT app_name, window_title, category, SUM(weighted_ms), SUM(focus_ms)
            FROM samples
            WHERE captured_at >= ?1 AND captured_at < ?2 AND idle = 0
            GROUP BY app_name, window_title, category
            ORDER BY SUM(weighted_ms) DESC
            LIMIT 12
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![start_ms, end_ms], |row| {
            let ms: i64 = row.get(3)?;
            Ok(WindowSummary {
                app_name: row.get(0)?,
                window_title: row.get(1)?,
                category: row.get(2)?,
                seconds: ms / 1_000,
                focus_seconds: row.get::<_, i64>(4)? / 1_000,
                share: share(ms, active_ms),
            })
        })
        .map_err(|error| error.to_string())?;

    collect_rows(rows)
}

fn timeline(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    bucket: &str,
) -> Result<Vec<TimelinePoint>, String> {
    let mut points = Vec::new();

    for (label, bucket_start_ms, bucket_end_ms) in timeline_buckets(start_ms, end_ms, bucket)? {
        points.push(TimelinePoint {
            hour: label,
            active_seconds: sum_ms(conn, bucket_start_ms, bucket_end_ms, "idle = 0")? / 1_000,
            idle_seconds: sum_ms(conn, bucket_start_ms, bucket_end_ms, "idle = 1")? / 1_000,
            top_apps: bucket_top_apps(conn, bucket_start_ms, bucket_end_ms)?,
        });
    }

    Ok(points)
}

fn bucket_top_apps(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<TimelineApp>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT app_name, category, SUM(weighted_ms)
            FROM samples
            WHERE captured_at >= ?1 AND captured_at < ?2 AND idle = 0
            GROUP BY app_name, category
            ORDER BY SUM(weighted_ms) DESC
            LIMIT 3
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![start_ms, end_ms], |row| {
            Ok(TimelineApp {
                app_name: row.get(0)?,
                category: row.get(1)?,
                seconds: row.get::<_, i64>(2)? / 1_000,
            })
        })
        .map_err(|error| error.to_string())?;

    collect_rows(rows)
}

fn collect_rows<T>(
    rows: impl Iterator<Item = Result<T, rusqlite::Error>>,
) -> Result<Vec<T>, String> {
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn share(value_ms: i64, active_ms: i64) -> f64 {
    if active_ms <= 0 {
        0.0
    } else {
        value_ms as f64 / active_ms as f64
    }
}

fn today_range_ms() -> (i64, i64) {
    let now = Local::now();
    let start = start_of_day(now);
    let end = start + chrono::Duration::days(1);

    (start.timestamp_millis(), end.timestamp_millis())
}

fn resolve_range(query: Option<RangeQuery>) -> RangeInfo {
    let now = Local::now();
    let requested = query
        .as_ref()
        .map(|value| value.preset.clone())
        .unwrap_or_else(|| "day".to_string());

    if requested == "custom" {
        if let Some(query) = query.as_ref() {
            if let (Some(start_ms), Some(end_ms)) = (query.start_ms, query.end_ms) {
                if end_ms > start_ms {
                    let bucket = bucket_for_span(end_ms - start_ms);
                    return RangeInfo {
                        preset: "custom".to_string(),
                        start_ms,
                        end_ms,
                        label: format!(
                            "{} - {}",
                            format_range_boundary(start_ms),
                            format_range_boundary(end_ms)
                        ),
                        bucket,
                    };
                }
            }
        }
    }

    let (preset, start, end, bucket) = match requested.as_str() {
        "week" => {
            let today = start_of_day(now);
            let start = today - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);
            ("week", start, start + chrono::Duration::days(7), "day")
        }
        "month" => {
            let start = local_date(now.year(), now.month(), 1);
            let end = add_month(start);
            ("month", start, end, "day")
        }
        "year" => {
            let start = local_date(now.year(), 1, 1);
            let end = local_date(now.year() + 1, 1, 1);
            ("year", start, end, "month")
        }
        _ => {
            let start = start_of_day(now);
            ("day", start, start + chrono::Duration::days(1), "hour")
        }
    };

    RangeInfo {
        preset: preset.to_string(),
        start_ms: start.timestamp_millis(),
        end_ms: end.timestamp_millis(),
        label: range_label(preset, start, end),
        bucket: bucket.to_string(),
    }
}

fn timeline_buckets(
    start_ms: i64,
    end_ms: i64,
    bucket: &str,
) -> Result<Vec<(String, i64, i64)>, String> {
    let mut buckets = Vec::new();
    let mut cursor = Local
        .timestamp_millis_opt(start_ms)
        .single()
        .ok_or_else(|| "Invalid range start".to_string())?;
    let end = Local
        .timestamp_millis_opt(end_ms)
        .single()
        .ok_or_else(|| "Invalid range end".to_string())?;

    let max_buckets = match bucket {
        "month" => 60,
        "day" => 120,
        _ => 48,
    };

    while cursor < end && buckets.len() < max_buckets {
        let next = match bucket {
            "month" => add_month(cursor),
            "day" => cursor + chrono::Duration::days(1),
            _ => cursor + chrono::Duration::hours(1),
        }
        .min(end);

        let label = match bucket {
            "month" => cursor.format("%Y-%m").to_string(),
            "day" => cursor.format("%m-%d").to_string(),
            _ => cursor.format("%H").to_string(),
        };

        buckets.push((label, cursor.timestamp_millis(), next.timestamp_millis()));
        cursor = next;
    }

    Ok(buckets)
}

fn bucket_for_span(span_ms: i64) -> String {
    let day_ms = 86_400_000;
    if span_ms <= day_ms * 2 {
        "hour".to_string()
    } else if span_ms <= day_ms * 95 {
        "day".to_string()
    } else {
        "month".to_string()
    }
}

fn start_of_day(dt: DateTime<Local>) -> DateTime<Local> {
    local_date(dt.year(), dt.month(), dt.day())
}

fn local_date(year: i32, month: u32, day: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .unwrap()
}

fn add_month(dt: DateTime<Local>) -> DateTime<Local> {
    let year = dt.year() + if dt.month() == 12 { 1 } else { 0 };
    let month = if dt.month() == 12 { 1 } else { dt.month() + 1 };
    local_date(year, month, 1)
}

fn range_label(preset: &str, start: DateTime<Local>, end: DateTime<Local>) -> String {
    match preset {
        "week" => format!("{} - {}", start.format("%Y-%m-%d"), (end - chrono::Duration::days(1)).format("%Y-%m-%d")),
        "month" => start.format("%Y-%m").to_string(),
        "year" => start.format("%Y").to_string(),
        _ => start.format("%Y-%m-%d").to_string(),
    }
}

fn format_range_boundary(timestamp_ms: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp_ms.to_string())
}

fn format_timestamp(timestamp_ms: i64) -> String {
    let dt: DateTime<Local> = Local.timestamp_millis_opt(timestamp_ms).single().unwrap();
    dt.format("%m-%d %H:%M").to_string()
}
