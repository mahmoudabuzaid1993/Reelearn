//! `/admin/users`, `/admin/groups`: people, CSV import, groups and membership.

use crate::auth::AuthUser;
use crate::dto::*;
use crate::error::{ApiError, FieldError};
use crate::routes::{page_params, record_audit, Json, Path, Query};
use crate::services::views;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::Router;
use reelearn_db::repos::{groups, jobs, users};
use reelearn_db::TenantTx;
use reelearn_domain::cursor::{page, Cursor};
use reelearn_domain::{phone, Channel, DeliverySlot, Role, UserStatus};
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/users/import", post(import_users))
        .route("/admin/users/{user_id}", get(get_user).patch(patch_user))
        .route("/admin/groups", get(list_groups).post(create_group))
        .route("/admin/groups/{group_id}/members", put(replace_members))
}

#[derive(Deserialize)]
struct UsersQuery {
    q: Option<String>,
    role: Option<Role>,
    status: Option<UserStatus>,
    group_id: Option<Uuid>,
    language: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<UsersQuery>,
) -> Result<Json<Page<User>>, ApiError> {
    let (cursor, limit) = page_params(&q.page)?;
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = users::list(
        tx.conn(),
        &users::UserFilter {
            q: q.q,
            role: q.role,
            status: q.status,
            group_id: q.group_id,
            language: q.language,
        },
        cursor,
        limit,
    )
    .await?;
    let (rows, next) = page(rows, limit, |r| Cursor {
        ts: r.created_at,
        id: r.id,
    });
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        items.push(views::user(&mut tx, r).await?);
    }
    tx.commit().await?;
    Ok(Json(Page {
        items,
        next_cursor: next,
    }))
}

async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = users::get(tx.conn(), user_id).await?;
    let v = views::user(&mut tx, &row).await?;
    tx.commit().await?;
    Ok(Json(v))
}

async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UserCreate>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let row = insert_user(&mut tx, auth.tenant_id, &body).await?;
    if body.send_invite.unwrap_or(true) && row.phone_e164.is_some() {
        jobs::enqueue(
            tx.conn(),
            Some(auth.tenant_id),
            "send_delivery",
            serde_json::json!({ "kind": "invite", "user_id": row.id }),
            None,
        )
        .await?;
    }
    record_audit(
        &mut tx,
        auth.user_id,
        "user.create",
        "user",
        Some(row.id),
        Some(serde_json::json!({ "role": row.role, "full_name": row.full_name })),
    )
    .await?;
    let v = views::user(&mut tx, &row).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn insert_user(
    tx: &mut TenantTx,
    tenant_id: Uuid,
    body: &UserCreate,
) -> Result<users::UserRow, ApiError> {
    let phone_e164 = match body.phone.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(p) => Some(phone::normalize(p).map_err(|e| ApiError::field("phone", &e.to_string()))?),
        None => None,
    };
    let email = body
        .email
        .as_deref()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty());
    if phone_e164.is_none() && email.is_none() {
        return Err(ApiError::field("phone", "phone or email is required"));
    }
    if body.full_name.trim().is_empty() {
        return Err(ApiError::field("full_name", "is required"));
    }
    if let Some(gids) = &body.group_ids {
        if !groups::ids_exist(tx.conn(), gids).await? {
            return Err(ApiError::field("group_ids", "contains an unknown group"));
        }
    }
    let lang = body
        .preferred_language
        .clone()
        .unwrap_or_else(|| "ar".into());
    let row = users::create(
        tx.conn(),
        tenant_id,
        users::NewUser {
            role: body.role,
            status: UserStatus::Invited,
            full_name: body.full_name.trim(),
            phone_e164: phone_e164.as_deref(),
            email: email.as_deref(),
            employee_no: body.employee_no.as_deref(),
            preferred_language: &lang,
            ui_language: if lang == "en" { "en" } else { "ar" },
            manager_id: body.manager_id,
            location: body.location.as_deref(),
            job_title: body.job_title.as_deref(),
            delivery_channel: body.delivery_channel.unwrap_or(Channel::App),
            delivery_slot: body.delivery_slot.unwrap_or(DeliverySlot::ShiftStart),
        },
    )
    .await?;
    if let Some(gids) = &body.group_ids {
        users::set_groups(tx.conn(), tenant_id, row.id, gids).await?;
    }
    Ok(row)
}

async fn patch_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserPatch>,
) -> Result<Json<User>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    if let Some(gids) = &body.group_ids {
        if !groups::ids_exist(tx.conn(), gids).await? {
            return Err(ApiError::field("group_ids", "contains an unknown group"));
        }
    }
    if let Some(Some(m)) = body.manager_id {
        if users::find(tx.conn(), m).await?.is_none() {
            return Err(ApiError::field("manager_id", "unknown person"));
        }
    }
    let update = users::UserUpdate {
        full_name: body.full_name,
        role: body.role,
        status: body.status,
        preferred_language: body.preferred_language,
        manager_id: body.manager_id,
        location: body.location,
        job_title: body.job_title,
        delivery_channel: body.delivery_channel,
        delivery_slot: body.delivery_slot,
        ..Default::default()
    };
    let row = users::update(tx.conn(), user_id, update).await?;
    if let Some(gids) = &body.group_ids {
        users::set_groups(tx.conn(), auth.tenant_id, user_id, gids).await?;
    }
    if row.status == UserStatus::Deactivated {
        reelearn_db::repos::auth::revoke_all_for_user(tx.conn(), user_id).await?;
    }
    record_audit(
        &mut tx,
        auth.user_id,
        "user.update",
        "user",
        Some(row.id),
        Some(serde_json::json!({ "status": row.status, "role": row.role })),
    )
    .await?;
    let v = views::user(&mut tx, &row).await?;
    tx.commit().await?;
    crate::services::learn::invalidate_plan_cache(&state, auth.tenant_id, user_id).await;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct ImportQuery {
    dry_run: Option<bool>,
}

/// CSV columns (header row required, any order): full_name, phone, email, role, employee_no,
/// preferred_language, manager_employee_no, groups (`;`-separated names), location, job_title, channel.
async fn import_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ImportQuery>,
    body: String,
) -> Result<Json<ImportReport>, ApiError> {
    let dry_run = q.dry_run.unwrap_or(false);
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| ApiError::BadRequest(format!("CSV header: {e}")))?
        .iter()
        .map(|h| h.to_ascii_lowercase())
        .collect();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let c_name =
        col("full_name").ok_or_else(|| ApiError::field("csv", "a full_name column is required"))?;
    let (c_phone, c_email, c_role, c_emp, c_lang, c_mgr, c_groups, c_loc, c_title, c_channel) = (
        col("phone"),
        col("email"),
        col("role"),
        col("employee_no"),
        col("preferred_language"),
        col("manager_employee_no"),
        col("groups"),
        col("location"),
        col("job_title"),
        col("channel"),
    );

    let mut report = ImportReport {
        total_rows: 0,
        created: 0,
        updated: 0,
        skipped: 0,
        errors: vec![],
        dry_run,
    };
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let existing_groups = groups::list(tx.conn()).await?;
    for (i, record) in reader.records().enumerate() {
        let row_no = i + 2;
        report.total_rows += 1;
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                report.errors.push(ImportError {
                    row: row_no,
                    message: e.to_string(),
                });
                continue;
            }
        };
        let get = |c: Option<usize>| {
            c.and_then(|c| record.get(c))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let full_name = match get(Some(c_name)) {
            Some(n) => n,
            None => {
                report.errors.push(ImportError {
                    row: row_no,
                    message: "full_name is empty".into(),
                });
                continue;
            }
        };
        let phone_e164 = match get(c_phone).map(|p| phone::normalize(&p)) {
            Some(Ok(p)) => Some(p),
            Some(Err(e)) => {
                report.errors.push(ImportError {
                    row: row_no,
                    message: e.to_string(),
                });
                continue;
            }
            None => None,
        };
        let email = get(c_email).map(|e| e.to_ascii_lowercase());
        if phone_e164.is_none() && email.is_none() {
            report.errors.push(ImportError {
                row: row_no,
                message: "phone or email is required".into(),
            });
            continue;
        }
        let role = match get(c_role).map(|r| r.parse::<Role>()) {
            Some(Ok(r)) => r,
            Some(Err(_)) => {
                report.errors.push(ImportError {
                    row: row_no,
                    message: "role must be admin, manager, sme or learner".into(),
                });
                continue;
            }
            None => Role::Learner,
        };
        let channel = match get(c_channel).map(|c| c.parse::<Channel>()) {
            Some(Ok(c)) => c,
            Some(Err(_)) => {
                report.errors.push(ImportError {
                    row: row_no,
                    message: "channel must be app, whatsapp, both or teams".into(),
                });
                continue;
            }
            None => Channel::App,
        };
        let manager_id = match get(c_mgr) {
            Some(emp) => match users::find_by_employee_no(tx.conn(), &emp).await? {
                Some(m) => Some(m.id),
                None => {
                    report.errors.push(ImportError {
                        row: row_no,
                        message: format!(
                            "manager with employee_no {emp} not found (import managers first)"
                        ),
                    });
                    continue;
                }
            },
            None => None,
        };
        let mut group_ids = Vec::new();
        let mut unknown_group = None;
        for g in get(c_groups)
            .map(|s| {
                s.split(';')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
        {
            match existing_groups.iter().find(|eg| eg.name == g) {
                Some(eg) => group_ids.push(eg.id),
                None => unknown_group = Some(g),
            }
        }
        if let Some(g) = unknown_group {
            report.errors.push(ImportError {
                row: row_no,
                message: format!("group '{g}' does not exist"),
            });
            continue;
        }

        let existing = match (&phone_e164, &email) {
            (Some(p), _) => users::find_by_phone(tx.conn(), p).await?,
            (None, Some(e)) => users::find_by_email(tx.conn(), e).await?,
            _ => None,
        };
        if dry_run {
            if existing.is_some() {
                report.updated += 1;
            } else {
                report.created += 1;
            }
            continue;
        }
        match existing {
            Some(u) => {
                users::update(
                    tx.conn(),
                    u.id,
                    users::UserUpdate {
                        full_name: Some(full_name),
                        role: Some(role),
                        preferred_language: get(c_lang),
                        manager_id: manager_id.map(Some),
                        location: get(c_loc),
                        job_title: get(c_title),
                        delivery_channel: Some(channel),
                        employee_no: get(c_emp),
                        email: email.clone(),
                        ..Default::default()
                    },
                )
                .await?;
                if !group_ids.is_empty() {
                    for g in &group_ids {
                        users::add_to_group(tx.conn(), auth.tenant_id, u.id, *g).await?;
                    }
                }
                report.updated += 1;
            }
            None => {
                let create = UserCreate {
                    full_name,
                    role,
                    phone: phone_e164.clone(),
                    email: email.clone(),
                    employee_no: get(c_emp),
                    preferred_language: get(c_lang),
                    manager_id,
                    location: get(c_loc),
                    job_title: get(c_title),
                    delivery_channel: Some(channel),
                    delivery_slot: None,
                    group_ids: Some(group_ids),
                    send_invite: Some(false),
                };
                match insert_user(&mut tx, auth.tenant_id, &create).await {
                    Ok(_) => report.created += 1,
                    Err(ApiError::Validation(errs)) => report.errors.push(ImportError {
                        row: row_no,
                        message: errs
                            .iter()
                            .map(|FieldError { field, message }| format!("{field}: {message}"))
                            .collect::<Vec<_>>()
                            .join("; "),
                    }),
                    Err(ApiError::Conflict(m)) => report.errors.push(ImportError {
                        row: row_no,
                        message: m,
                    }),
                    Err(e) => return Err(e),
                }
            }
        }
    }
    if dry_run {
        tx.rollback().await?;
    } else {
        record_audit(&mut tx, auth.user_id, "user.import", "user", None, Some(serde_json::json!({ "created": report.created, "updated": report.updated, "errors": report.errors.len() }))).await?;
        tx.commit().await?;
    }
    Ok(Json(report))
}

fn group_view(g: &groups::GroupRow) -> Group {
    Group {
        id: g.id,
        name: g.name.clone(),
        kind: g.kind.clone(),
        member_count: g.member_count,
    }
}

async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Group>>, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let rows = groups::list(tx.conn()).await?;
    tx.commit().await?;
    Ok(Json(rows.iter().map(group_view).collect()))
}

async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<GroupCreate>,
) -> Result<(StatusCode, Json<Group>), ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::field("name", "is required"));
    }
    let kind = body.kind.unwrap_or_else(|| "team".into());
    if !matches!(kind.as_str(), "team" | "location" | "role" | "custom") {
        return Err(ApiError::field(
            "kind",
            "must be team, location, role or custom",
        ));
    }
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    let g = groups::create(tx.conn(), auth.tenant_id, name, &kind).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "group.create",
        "group",
        Some(g.id),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(group_view(&g))))
}

async fn replace_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(body): Json<MemberList>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.db.tenant_tx(auth.tenant_id).await?;
    groups::get(tx.conn(), group_id).await?;
    for u in &body.user_ids {
        if users::find(tx.conn(), *u).await?.is_none() {
            return Err(ApiError::field("user_ids", &format!("unknown person {u}")));
        }
    }
    groups::replace_members(tx.conn(), auth.tenant_id, group_id, &body.user_ids).await?;
    record_audit(
        &mut tx,
        auth.user_id,
        "group.members",
        "group",
        Some(group_id),
        Some(serde_json::json!({ "count": body.user_ids.len() })),
    )
    .await?;
    tx.commit().await?;
    for u in &body.user_ids {
        crate::services::learn::invalidate_plan_cache(&state, auth.tenant_id, *u).await;
    }
    Ok(StatusCode::NO_CONTENT)
}
