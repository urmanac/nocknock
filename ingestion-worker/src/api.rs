use crate::*;

/// Grace added to the sync window before health calls the data stale; matches the
/// dashboard's own freshness allowance.
pub(crate) const HEALTH_SYNC_GRACE_SECONDS: i64 = 90;

/// Health verdict (issue #87). Pure. A snapshot existing is not enough to be "ok":
/// the most recent cycle must have succeeded and the data must be inside the sync
/// window plus grace.
pub(crate) fn health_status(has_snapshot: bool, lag_seconds: i64, sync_window_seconds: i64, last_cycle_failed: bool) -> &'static str {
    if last_cycle_failed {
        "degraded"
    } else if !has_snapshot {
        "starting"
    } else if lag_seconds > sync_window_seconds + HEALTH_SYNC_GRACE_SECONDS {
        "degraded"
    } else {
        "ok"
    }
}

// ============================================================
// Handlers
// ============================================================

pub(crate) async fn health_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let latest = state.latest_snapshot.lock().await;
    let latest = &*latest;
    let last_error: Option<(DateTime<Utc>, String)> =
        state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
    let last_sync: Option<DateTime<Utc>> = latest.as_ref().map(|s| s.meta.generated_at);
    let ingestion_lag_seconds: i64 = last_sync
        .map(|ts| {
            Utc::now().signed_duration_since(ts).num_seconds().max(0)
        })
        .unwrap_or(0);

    let sync_duration_ms = state.last_sync_duration_ms.load(std::sync::atomic::Ordering::Relaxed);

    let status = health_status(
        latest.is_some(),
        ingestion_lag_seconds,
        state.config.sync_window_seconds as i64,
        last_error.is_some(),
    );
    Json(json!({
        "status": status,
        "last_error": last_error.as_ref().map(|(_, e)| e.clone()),
        "last_error_at": last_error.as_ref().map(|(t, _)| t.to_rfc3339()),
        "last_sync": last_sync.map(|dt| dt.to_rfc3339()),
        "ingestion_lag_seconds": ingestion_lag_seconds,
        "sync_duration_ms": sync_duration_ms,
        "sync_window_seconds": state.config.sync_window_seconds,
    }))
}

pub(crate) async fn get_latest_snapshot_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut latest = state.latest_snapshot.lock().await;

    if latest.is_none() {
        use sqlx::Row;
        if let Ok(Some(r)) = sqlx::query(
            "SELECT snapshot_id, generated_at, state_hash, version, data FROM snapshots ORDER BY generated_at DESC LIMIT 1"
        ).fetch_optional(&state.db).await {
            if let (Ok(sid), Ok(gen), Ok(sh), Ok(ver), Ok(dv)) = (
                r.try_get::<uuid::Uuid, _>("snapshot_id"),
                r.try_get::<DateTime<Utc>, _>("generated_at"),
                r.try_get::<String, _>("state_hash"),
                r.try_get::<String, _>("version"),
                r.try_get::<serde_json::Value, _>("data"),
            ) {
                if let Ok(mut data) = serde_json::from_value::<SnapshotData>(dv) {
                    // Stored quotes are not current ones (issue #115).
                    forget_stored_route_quotes(&mut data);
                    *latest = Some(SnapshotEnvelope {
                        meta: SnapshotMeta {
                            generated_at: gen,
                            state_hash: sh,
                            version: ver,
                            snapshot_id: sid,
                        },
                        data,
                    });
                }
            }
        }
    }

    if let Some(envelope) = &*latest {
        // Check If-None-Match for 304
        if let Some(if_none_match) = headers.get("if-none-match") {
            if if_none_match == envelope.meta.state_hash.as_str() {
                return (StatusCode::NOT_MODIFIED, HeaderMap::new(), Json(serde_json::Value::Null));
            }
        }

        let mut headers = HeaderMap::new();
        if let Ok(value) = envelope.meta.state_hash.parse() {
            headers.insert("etag", value);
        }
        if let Ok(value) = envelope.meta.generated_at.to_rfc3339().parse() {
            headers.insert("last-modified", value);
        }
        let cache_control = "no-cache, must-revalidate";
        if let Ok(value) = cache_control.parse() {
            headers.insert("cache-control", value);
        }

        let mut body = serde_json::to_value(envelope).unwrap_or_else(|_| json!({}));
        body["data_provenance"] = data_provenance(&state.db).await;
        (StatusCode::OK, headers, Json(body))
    } else {
        let last_error = state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
        (
            StatusCode::SERVICE_UNAVAILABLE,
            HeaderMap::new(),
            Json(no_snapshot_body(last_error.as_ref().map(|(_, e)| e.as_str()))),
        )
    }
}

/// 503 NO_SNAPSHOT body. Pure. On a first run the real cause (a placeholder key, a
/// missing price source, an NFT the wallet does not own) is only in the last ingestion
/// error, so it travels as `detail`; the dashboard and MCP show it instead of a bare
/// "No snapshot available yet".
pub(crate) fn no_snapshot_body(last_error: Option<&str>) -> serde_json::Value {
    let mut body = json!({
        "error": "NO_SNAPSHOT",
        "message": "No snapshot available yet"
    });
    if let Some(e) = last_error.filter(|e| !e.trim().is_empty()) {
        body["detail"] = json!(e);
    }
    body
}

pub(crate) async fn trigger_ingest_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    match run_cycle_tracked(&state).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "triggered",
                "message": "Ingestion cycle completed successfully"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Ingestion failed: {}", e)
            })),
        ),
    }
}

pub(crate) async fn get_historical_snapshots_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    // `interval` was declared in api.yaml but never implemented (now removed from the
    // spec); reject it explicitly instead of silently ignoring declared parameters.
    if params.contains_key("interval") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "INVALID_PARAMETER",
                "message": "interval is not supported; use before/after RFC3339 cursors with limit"
            })),
        )
            .into_response();
    }

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 1000);

    let parse_cursor = |key: &str| -> Option<DateTime<Utc>> {
        params
            .get(key)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
    };
    let before = parse_cursor("before");
    let after = parse_cursor("after");

    let base =
        "SELECT snapshot_id, generated_at, state_hash, version, data FROM snapshots".to_string();
    let rows = if let Some(before) = before {
        sqlx::query(&format!(
            "{base} WHERE generated_at < $1 ORDER BY generated_at DESC LIMIT $2"
        ))
        .bind(before)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else if let Some(after) = after {
        sqlx::query(&format!(
            "{base} WHERE generated_at > $1 ORDER BY generated_at DESC LIMIT $2"
        ))
        .bind(after)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query(&format!("{base} ORDER BY generated_at DESC LIMIT $1"))
            .bind(limit)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
    };

    let snapshots: Vec<SnapshotEnvelope> = rows
        .into_iter()
        .filter_map(|r| {
            let data: SnapshotData = serde_json::from_value(r.try_get("data").ok()?).ok()?;
            let envelope = SnapshotEnvelope {
                meta: SnapshotMeta {
                    generated_at: r.try_get("generated_at").ok()?,
                    state_hash: r.try_get("state_hash").ok()?,
                    version: r.try_get("version").ok()?,
                    snapshot_id: r.try_get("snapshot_id").ok()?,
                },
                data,
            };
            Some(envelope)
        })
        .map(|mut envelope| {
            slim_historical_payload(&mut envelope.data);
            envelope
        })
        .collect();

    let returned = snapshots.len();
    Json(json!({
        "snapshots": snapshots,
        "pagination": {
            "limit": limit,
            "returned": returned,
            "has_more": returned == limit as usize,
        }
    }))
    .into_response()
}

/// Bound the historical response payload: charts only read each stored snapshot's
/// scalar / current-point values (the `[0]` element of embedded series), never its
/// multi-point series. Truncating the embedded counterfactual to its current point and
/// dropping the Bollinger channel points keeps a 288-row response ~1 MB instead of
/// ~5 MB. `/snapshot/latest` still serves the full embedded series (MCP consumers and
/// the cold-start chart fallback rely on it; snapshot.yaml declares these fields).
pub(crate) fn slim_historical_payload(data: &mut SnapshotData) {
    let dm = &mut data.derived_metrics;
    dm.strategy_counterfactual.time_points.truncate(1);
    dm.strategy_counterfactual.active_portfolio_series.truncate(1);
    dm.strategy_counterfactual.passive_baseline_series.truncate(1);
    // Channels are recomputed by the worker each cycle from history; historical rows
    // never need to carry them
    dm.structural_channels.channels.clear();
}

pub(crate) async fn get_snapshot_metadata_handler(
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let rows = sqlx::query!(
        r#"
        SELECT snapshot_id, generated_at, state_hash
        FROM snapshots
        ORDER BY generated_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let snapshots: Vec<_> = rows
        .into_iter()
        .map(|r| {
            let generated_at = r.generated_at;
            let age_seconds = Utc::now().signed_duration_since(generated_at).num_seconds().max(0)
                as u64;

            json!({
                "snapshot_id": r.snapshot_id,
                "generated_at": r.generated_at.to_rfc3339(),
                "state_hash": r.state_hash,
                "age_seconds": age_seconds
            })
        })
        .collect();

    Json(json!({ "snapshots": snapshots }))
}

/// Rebuild an envelope from a stored `snapshots` row. Pure. The `data` column holds
/// only the `SnapshotData` payload (see `store_snapshot`); meta comes from the row.
pub(crate) fn envelope_from_stored(
    snapshot_id: Uuid,
    generated_at: DateTime<Utc>,
    state_hash: String,
    version: String,
    data: serde_json::Value,
) -> Result<SnapshotEnvelope, serde_json::Error> {
    Ok(SnapshotEnvelope {
        meta: SnapshotMeta { generated_at, state_hash, version, snapshot_id },
        data: serde_json::from_value::<SnapshotData>(data)?,
    })
}

pub(crate) async fn get_snapshot_by_id_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let snapshot_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(json!({"error": "INVALID_UUID"}))),
    };
    let row = sqlx::query!(
        r#"
        SELECT snapshot_id, generated_at, state_hash, version, data
        FROM snapshots
        WHERE snapshot_id = $1
        "#,
        snapshot_id
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or_else(|_| None);

    if let Some(row) = row {
        // store_snapshot writes only the SnapshotData payload into `data`; the envelope
        // meta lives in the row's own columns.
        if let Ok(envelope) =
            envelope_from_stored(row.snapshot_id, row.generated_at, row.state_hash, row.version, row.data)
        {
            (
                StatusCode::OK,
                HeaderMap::new(),
                Json(json!(envelope))
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                Json(json!({
                    "error": "CORRUPTED_DATA",
                    "message": "Failed to parse snapshot data"
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Json(json!({
                "error": "NOT_FOUND",
                "message": "Snapshot not found"
            })),
        )
    }
}
