use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};

use crate::classifier;
use crate::models::{
    ActivityTrack, ActivityTrackSegment, AppSummary, CaptureHealth, CategorySummary, Dashboard,
    LiveWindow, MetricCard, RangeInfo, RangeQuery, TimelineApp, TimelinePoint, WindowSample,
    WindowSummary,
};

const TRACKABLE_WINDOW_SQL: &str = "
    NOT (
        lower(sw.app_name) = 'explorer.exe'
        AND lower(trim(sw.window_title)) IN ('', 'program manager')
    )
";

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

        CREATE TABLE IF NOT EXISTS activity_segments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start_at INTEGER NOT NULL,
            end_at INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            state_hash TEXT NOT NULL,
            idle INTEGER NOT NULL,
            focused_app TEXT NOT NULL,
            focused_title TEXT NOT NULL,
            focused_category TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS segment_windows (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            segment_id INTEGER NOT NULL,
            app_name TEXT NOT NULL,
            window_title TEXT NOT NULL,
            process_path TEXT NOT NULL,
            pid INTEGER NOT NULL,
            category TEXT NOT NULL,
            visible_area INTEGER NOT NULL,
            visible_share REAL NOT NULL,
            focused INTEGER NOT NULL,
            FOREIGN KEY(segment_id) REFERENCES activity_segments(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_segments_time ON activity_segments(start_at, end_at);
        CREATE INDEX IF NOT EXISTS idx_segments_state ON activity_segments(state_hash);
        CREATE INDEX IF NOT EXISTS idx_segment_windows_segment ON segment_windows(segment_id);
        CREATE INDEX IF NOT EXISTS idx_segment_windows_category ON segment_windows(category);
        CREATE INDEX IF NOT EXISTS idx_segment_windows_app ON segment_windows(app_name);
        ",
    )?;
    migrate_legacy_samples(&conn)?;

    Ok(db_path)
}

fn migrate_legacy_samples(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let segment_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM activity_segments", [], |row| {
            row.get(0)
        })?;
    let sample_count: i64 = conn.query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))?;

    if segment_count > 0 || sample_count == 0 {
        return Ok(());
    }

    conn.execute_batch(
        "
        BEGIN;
        INSERT INTO activity_segments (
            start_at, end_at, duration_ms, state_hash, idle,
            focused_app, focused_title, focused_category
        )
        SELECT
            captured_at,
            captured_at + MAX(duration_ms),
            MAX(duration_ms),
            'legacy-' || captured_at,
            MAX(idle),
            COALESCE(MAX(CASE WHEN focused = 1 THEN app_name END), ''),
            COALESCE(MAX(CASE WHEN focused = 1 THEN window_title END), ''),
            COALESCE(MAX(CASE WHEN focused = 1 THEN category END), '')
        FROM samples
        GROUP BY captured_at;

        INSERT INTO segment_windows (
            segment_id, app_name, window_title, process_path, pid,
            category, visible_area, visible_share, focused
        )
        SELECT
            activity_segments.id,
            samples.app_name,
            samples.window_title,
            samples.process_path,
            samples.pid,
            samples.category,
            samples.visible_area,
            samples.visible_share,
            samples.focused
        FROM samples
        JOIN activity_segments
            ON activity_segments.state_hash = 'legacy-' || samples.captured_at;
        COMMIT;
        ",
    )?;

    Ok(())
}

pub fn refresh_unclassified_categories(db_path: &Path) -> Result<(), String> {
    let mut conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let sample_candidates = {
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
    let segment_candidates = {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, app_name, window_title, process_path
                FROM segment_windows
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

    if sample_candidates.is_empty() && segment_candidates.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|error| error.to_string())?;
    for (id, app_name, window_title, process_path) in sample_candidates {
        let category = classifier::classify(&app_name, &window_title, &process_path);
        if category != "Unclassified" {
            tx.execute(
                "UPDATE samples SET category = ?1 WHERE id = ?2",
                params![category, id],
            )
            .map_err(|error| error.to_string())?;
            tx.execute(
                "
                UPDATE segment_windows
                SET category = ?1
                WHERE category = 'Unclassified'
                    AND app_name = ?2
                    AND window_title = ?3
                    AND process_path = ?4
                ",
                params![category, app_name, window_title, process_path],
            )
            .map_err(|error| error.to_string())?;
        }
    }
    for (id, app_name, window_title, process_path) in segment_candidates {
        let category = classifier::classify(&app_name, &window_title, &process_path);
        if category != "Unclassified" {
            tx.execute(
                "UPDATE segment_windows SET category = ?1 WHERE id = ?2",
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
    let duration_ms = duration_ms as i64;
    let end_at = captured_at + duration_ms;
    let state_hash = segment_state_hash(idle, samples);
    let merge_gap_ms = (duration_ms * 2).max(5_000);

    let previous = tx
        .query_row(
            "
            SELECT id, end_at, state_hash
            FROM activity_segments
            ORDER BY end_at DESC, id DESC
            LIMIT 1
            ",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if let Some((segment_id, previous_end_at, previous_hash)) = previous {
        let gap_ms = captured_at - previous_end_at;
        if previous_hash == state_hash && gap_ms >= -merge_gap_ms && gap_ms <= merge_gap_ms {
            tx.execute(
                "
                UPDATE activity_segments
                SET end_at = MAX(end_at, ?1),
                    duration_ms = MAX(end_at, ?1) - start_at
                WHERE id = ?2
                ",
                params![end_at, segment_id],
            )
            .map_err(|error| error.to_string())?;
            tx.commit().map_err(|error| error.to_string())?;
            return Ok(());
        }
    }

    let (focused_app, focused_title, focused_category) = focused_window(samples);
    tx.execute(
        "
        INSERT INTO activity_segments (
            start_at, end_at, duration_ms, state_hash, idle,
            focused_app, focused_title, focused_category
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ",
        params![
            captured_at,
            end_at,
            duration_ms,
            state_hash,
            i64::from(idle),
            focused_app,
            focused_title,
            focused_category,
        ],
    )
    .map_err(|error| error.to_string())?;

    let segment_id = tx.last_insert_rowid();
    if !idle && samples.is_empty() {
        tx.execute(
            "
            INSERT INTO segment_windows (
                segment_id, app_name, window_title, process_path, pid,
                category, visible_area, visible_share, focused
            )
            VALUES (?1, 'Desktop', 'No visible tracked windows', '', 0, 'System', 0, 1.0, 0)
            ",
            params![segment_id],
        )
        .map_err(|error| error.to_string())?;
    } else if !idle {
        for sample in samples {
            tx.execute(
                "
                INSERT INTO segment_windows (
                    segment_id, app_name, window_title, process_path, pid,
                    category, visible_area, visible_share, focused
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ",
                params![
                    segment_id,
                    sample.app_name,
                    sample.window_title,
                    sample.process_path,
                    sample.pid as i64,
                    sample.category,
                    sample.visible_area,
                    sample.visible_share,
                    i64::from(sample.focused),
                ],
            )
            .map_err(|error| error.to_string())?;
        }
    }

    tx.commit().map_err(|error| error.to_string())
}

fn segment_state_hash(idle: bool, samples: &[WindowSample]) -> String {
    if idle {
        return "idle".to_string();
    }

    if samples.is_empty() {
        return "active:desktop".to_string();
    }

    let mut parts = samples
        .iter()
        .map(|sample| {
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                sample.app_name,
                sample.window_title,
                sample.process_path,
                sample.category,
                sample.focused
            )
        })
        .collect::<Vec<_>>();
    parts.sort();

    format!("active:{}", parts.join("\u{1e}"))
}

fn focused_window(samples: &[WindowSample]) -> (&str, &str, &str) {
    samples
        .iter()
        .find(|sample| sample.focused)
        .map(|sample| {
            (
                sample.app_name.as_str(),
                sample.window_title.as_str(),
                sample.category.as_str(),
            )
        })
        .unwrap_or(("", "", ""))
}

pub fn dashboard(
    db_path: &Path,
    storage_location: &str,
    monitoring: bool,
    idle_threshold_seconds: u64,
    sample_interval_ms: u64,
    range_query: Option<RangeQuery>,
    live_windows: Vec<LiveWindow>,
) -> Result<Dashboard, String> {
    let conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let range = resolve_range(range_query);
    let start_ms = range.start_ms;
    let end_ms = range.end_ms;
    let active_ms = segment_sum_ms(&conn, start_ms, end_ms, "idle = 0")?;
    let idle_ms = segment_sum_ms(&conn, start_ms, end_ms, "idle = 1")?;
    let focus_ms = focused_visible_ms(&conn, start_ms, end_ms)?;
    let unclassified_ms = visible_ms(
        &conn,
        start_ms,
        end_ms,
        "s.idle = 0 AND sw.category = 'Unclassified'",
    )?;
    let category_visible_ms = deduped_presence_total_ms(&conn, start_ms, end_ms, "sw.category")?;
    let app_visible_ms = deduped_presence_total_ms(
        &conn,
        start_ms,
        end_ms,
        "sw.app_name || char(31) || sw.category",
    )?;

    let (today_start_ms, today_end_ms) = today_range_ms();
    let samples_today: i64 = conn
        .query_row(
            "
            SELECT COUNT(*)
            FROM activity_segments
            WHERE end_at > ?1 AND start_at < ?2
            ",
            params![today_start_ms, today_end_ms],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let window_rows_today: i64 = conn
        .query_row(
            "
            SELECT COUNT(*)
            FROM segment_windows sw
            JOIN activity_segments s ON s.id = sw.segment_id
            WHERE s.end_at > ?1 AND s.start_at < ?2
            ",
            params![today_start_ms, today_end_ms],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let segment_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM activity_segments", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    let window_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM segment_windows", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let total_rows = segment_rows + window_rows;

    let last_capture_at: Option<i64> = conn
        .query_row("SELECT MAX(end_at) FROM activity_segments", [], |row| {
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
                helper: "Elapsed desktop time while at least one window is visible".to_string(),
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
        categories: category_summaries(&conn, start_ms, end_ms, category_visible_ms)?,
        apps: app_summaries(&conn, start_ms, end_ms, app_visible_ms)?,
        windows: window_summaries(&conn, start_ms, end_ms)?,
        timeline: timeline_points,
        tracks: activity_tracks(&conn, start_ms, end_ms)?,
        live_windows,
        health: CaptureHealth {
            monitoring,
            database_path: db_path.display().to_string(),
            storage_location: storage_location.to_string(),
            database_size_bytes: database_size_bytes(db_path),
            estimated_daily_bytes: estimated_daily_bytes(
                db_path,
                total_rows,
                samples_today + window_rows_today,
            ),
            total_rows,
            samples_today,
            last_capture_at: last_capture_at.map(format_timestamp),
            idle_threshold_seconds,
            sample_interval_ms,
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

fn segment_sum_ms(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    condition: &str,
) -> Result<i64, String> {
    let sql = format!(
        "
        SELECT COALESCE(SUM(MAX(0, MIN(end_at, ?2) - MAX(start_at, ?1))), 0)
        FROM activity_segments
        WHERE end_at > ?1 AND start_at < ?2 AND {condition}
        "
    );

    conn.query_row(&sql, params![start_ms, end_ms], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn visible_ms(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    condition: &str,
) -> Result<i64, String> {
    let sql = format!(
        "
        SELECT COALESCE(SUM(MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1))), 0)
        FROM segment_windows sw
        JOIN activity_segments s ON s.id = sw.segment_id
        WHERE s.end_at > ?1 AND s.start_at < ?2 AND {TRACKABLE_WINDOW_SQL} AND {condition}
        "
    );

    conn.query_row(&sql, params![start_ms, end_ms], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn focused_visible_ms(conn: &Connection, start_ms: i64, end_ms: i64) -> Result<i64, String> {
    visible_ms(conn, start_ms, end_ms, "s.idle = 0 AND sw.focused = 1")
}

fn deduped_presence_total_ms(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    key_sql: &str,
) -> Result<i64, String> {
    let sql = format!(
        "
        SELECT COALESCE(SUM(overlap_ms), 0)
        FROM (
            SELECT
                s.id,
                {key_sql} AS presence_key,
                MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms
            FROM segment_windows sw
            JOIN activity_segments s ON s.id = sw.segment_id
            WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0 AND {TRACKABLE_WINDOW_SQL}
            GROUP BY s.id, presence_key
        )
        "
    );

    conn.query_row(&sql, params![start_ms, end_ms], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn category_summaries(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    total_ms: i64,
) -> Result<Vec<CategorySummary>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT category, SUM(overlap_ms), SUM(focus_ms), COUNT(*)
            FROM (
                SELECT
                    s.id,
                    sw.category,
                    MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms,
                    MAX(
                        CASE WHEN sw.focused = 1
                            THEN MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1))
                            ELSE 0
                        END
                    ) AS focus_ms
                FROM segment_windows sw
                JOIN activity_segments s ON s.id = sw.segment_id
                WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0
                    AND NOT (
                        lower(sw.app_name) = 'explorer.exe'
                        AND lower(trim(sw.window_title)) IN ('', 'program manager')
                    )
                GROUP BY s.id, sw.category
            )
            GROUP BY category
            ORDER BY SUM(overlap_ms) DESC
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
                share: share(ms, total_ms),
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
    total_ms: i64,
) -> Result<Vec<AppSummary>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT app_name, category, SUM(overlap_ms), SUM(focus_ms), MAX(last_seen)
            FROM (
                SELECT
                    s.id,
                    sw.app_name,
                    sw.category,
                    MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms,
                    MAX(
                        CASE WHEN sw.focused = 1
                            THEN MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1))
                            ELSE 0
                        END
                    ) AS focus_ms,
                    MAX(s.end_at) AS last_seen
                FROM segment_windows sw
                JOIN activity_segments s ON s.id = sw.segment_id
                WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0
                    AND NOT (
                        lower(sw.app_name) = 'explorer.exe'
                        AND lower(trim(sw.window_title)) IN ('', 'program manager')
                    )
                GROUP BY s.id, sw.app_name, sw.category
            )
            GROUP BY app_name, category
            ORDER BY SUM(overlap_ms) DESC
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
                share: share(ms, total_ms),
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
) -> Result<Vec<WindowSummary>, String> {
    let total_ms = visible_ms(conn, start_ms, end_ms, "s.idle = 0")?;
    let mut stmt = conn
        .prepare(
            "
            SELECT app_name, window_title, category, SUM(overlap_ms), SUM(focus_ms)
            FROM (
                SELECT
                    s.id,
                    sw.app_name,
                    sw.window_title,
                    sw.category,
                    MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms,
                    CASE WHEN sw.focused = 1
                        THEN MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1))
                        ELSE 0
                    END AS focus_ms
                FROM segment_windows sw
                JOIN activity_segments s ON s.id = sw.segment_id
                WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0
                    AND NOT (
                        lower(sw.app_name) = 'explorer.exe'
                        AND lower(trim(sw.window_title)) IN ('', 'program manager')
                    )
            )
            GROUP BY app_name, window_title, category
            ORDER BY SUM(overlap_ms) DESC
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
                share: share(ms, total_ms),
            })
        })
        .map_err(|error| error.to_string())?;

    collect_rows(rows)
}

fn activity_tracks(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<ActivityTrack>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT
                sw.app_name,
                sw.category,
                MAX(s.start_at, ?1) AS clipped_start,
                MIN(s.end_at, ?2) AS clipped_end,
                MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms
            FROM segment_windows sw
            JOIN activity_segments s ON s.id = sw.segment_id
            WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0
                AND NOT (
                    lower(sw.app_name) = 'explorer.exe'
                    AND lower(trim(sw.window_title)) IN ('', 'program manager')
                )
            GROUP BY s.id, sw.app_name, sw.category
            ORDER BY clipped_start ASC
            ",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![start_ms, end_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;

    let mut builders: BTreeMap<String, ActivityTrack> = BTreeMap::new();

    for row in collect_rows(rows)? {
        let (app_name, category, clipped_start, clipped_end, overlap_ms) = row;
        if overlap_ms <= 0 {
            continue;
        }

        let key = format!("{app_name}\u{1f}{category}");
        let track = builders.entry(key).or_insert_with(|| ActivityTrack {
            app_name,
            category,
            seconds: 0,
            segments: Vec::new(),
        });
        track.seconds += overlap_ms / 1_000;

        if let Some(last) = track.segments.last_mut() {
            if clipped_start <= last.end_ms + 1 {
                last.end_ms = last.end_ms.max(clipped_end);
                last.seconds = (last.end_ms - last.start_ms).max(0) / 1_000;
                continue;
            }
        }

        track.segments.push(ActivityTrackSegment {
            start_ms: clipped_start,
            end_ms: clipped_end,
            seconds: overlap_ms / 1_000,
        });
    }

    let mut tracks = builders.into_values().collect::<Vec<_>>();
    for track in &mut tracks {
        track.segments.sort_by_key(|segment| segment.start_ms);
    }
    tracks.sort_by(|left, right| right.seconds.cmp(&left.seconds));
    tracks.truncate(8);

    Ok(tracks)
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
            active_seconds: segment_sum_ms(conn, bucket_start_ms, bucket_end_ms, "idle = 0")?
                / 1_000,
            idle_seconds: segment_sum_ms(conn, bucket_start_ms, bucket_end_ms, "idle = 1")? / 1_000,
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
            SELECT app_name, category, SUM(overlap_ms)
            FROM (
                SELECT
                    s.id,
                    sw.app_name,
                    sw.category,
                    MAX(0, MIN(s.end_at, ?2) - MAX(s.start_at, ?1)) AS overlap_ms
                FROM segment_windows sw
                JOIN activity_segments s ON s.id = sw.segment_id
                WHERE s.end_at > ?1 AND s.start_at < ?2 AND s.idle = 0
                    AND NOT (
                        lower(sw.app_name) = 'explorer.exe'
                        AND lower(trim(sw.window_title)) IN ('', 'program manager')
                    )
                GROUP BY s.id, sw.app_name, sw.category
            )
            GROUP BY app_name, category
            ORDER BY SUM(overlap_ms) DESC
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
        "week" => format!(
            "{} - {}",
            start.format("%Y-%m-%d"),
            (end - chrono::Duration::days(1)).format("%Y-%m-%d")
        ),
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::models::{Rect, WindowSample};

    #[test]
    fn merges_unchanged_desktop_state_and_counts_parallel_app_tracks() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pctime-storage-test-{unique}"));
        let (db_path, _) = init_database(&root, &root).unwrap();
        let start_ms = 1_800_000_000_000_i64;
        let duration_ms = 600_000_u64;
        let samples = vec![
            test_sample("Codex.exe", "PCTime", "Development", true),
            test_sample("chrome.exe", "Docs", "Research", false),
        ];

        insert_samples(&db_path, start_ms, duration_ms, false, 0, &samples).unwrap();
        insert_samples(
            &db_path,
            start_ms + duration_ms as i64,
            duration_ms,
            false,
            0,
            &samples,
        )
        .unwrap();

        let dashboard = dashboard(
            &db_path,
            "install_dir",
            true,
            300,
            1_000,
            Some(RangeQuery {
                preset: "custom".to_string(),
                start_ms: Some(start_ms),
                end_ms: Some(start_ms + duration_ms as i64 * 2),
            }),
            Vec::new(),
        )
        .unwrap();
        let segment_rows: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM activity_segments", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(segment_rows, 1);
        assert_eq!(dashboard.active_seconds, 1_200);
        assert_eq!(
            dashboard
                .apps
                .iter()
                .find(|app| app.app_name == "Codex.exe")
                .unwrap()
                .seconds,
            1_200
        );
        assert_eq!(
            dashboard
                .apps
                .iter()
                .find(|app| app.app_name == "chrome.exe")
                .unwrap()
                .seconds,
            1_200
        );

        let _ = fs::remove_dir_all(root);
    }

    fn test_sample(app_name: &str, title: &str, category: &str, focused: bool) -> WindowSample {
        WindowSample {
            app_name: app_name.to_string(),
            window_title: title.to_string(),
            process_path: String::new(),
            pid: 1,
            rect: Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
            category: category.to_string(),
            visible_area: 10_000,
            visible_share: 0.5,
            focused,
        }
    }
}
