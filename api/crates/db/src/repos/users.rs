use crate::error::DbError;
use crate::repos::push_cursor_filter;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use reelearn_domain::cursor::Cursor;
use reelearn_domain::{Channel, DeliverySlot, Role, UserStatus};
use sqlx::{PgConnection, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub role: Role,
    pub status: UserStatus,
    pub full_name: String,
    pub phone_e164: Option<String>,
    pub email: Option<String>,
    pub employee_no: Option<String>,
    pub preferred_language: String,
    pub ui_language: String,
    pub manager_id: Option<Uuid>,
    pub location: Option<String>,
    pub job_title: Option<String>,
    pub delivery_channel: Channel,
    pub delivery_slot: DeliverySlot,
    pub shift_start: Option<NaiveTime>,
    pub shift_end: Option<NaiveTime>,
    pub captions_on: bool,
    pub reminders_on: bool,
    pub extra_reels_on: bool,
    pub paused_until: Option<NaiveDate>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    pub fn first_name(&self) -> String {
        self.full_name
            .split_whitespace()
            .next()
            .unwrap_or(&self.full_name)
            .to_string()
    }

    pub fn is_paused_on(&self, day: NaiveDate) -> bool {
        self.status == UserStatus::Paused || self.paused_until.map(|p| p >= day).unwrap_or(false)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct GroupRef {
    pub id: Uuid,
    pub name: String,
}

const COLS: &str = "id, tenant_id, role, status, full_name, phone_e164, email, employee_no, preferred_language, ui_language, manager_id, location, job_title, delivery_channel, delivery_slot, shift_start, shift_end, captions_on, reminders_on, extra_reels_on, paused_until, last_seen_at, created_at, updated_at";

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<UserRow, DbError> {
    find(conn, id).await?.ok_or(DbError::NotFound("user"))
}

pub async fn find(conn: &mut PgConnection, id: Uuid) -> Result<Option<UserRow>, DbError> {
    Ok(
        sqlx::query_as::<_, UserRow>(&format!("SELECT {COLS} FROM users WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn find_by_phone(
    conn: &mut PgConnection,
    phone: &str,
) -> Result<Option<UserRow>, DbError> {
    Ok(
        sqlx::query_as::<_, UserRow>(&format!("SELECT {COLS} FROM users WHERE phone_e164 = $1"))
            .bind(phone)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn find_by_email(
    conn: &mut PgConnection,
    email: &str,
) -> Result<Option<UserRow>, DbError> {
    Ok(sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {COLS} FROM users WHERE lower(email) = lower($1)"
    ))
    .bind(email)
    .fetch_optional(conn)
    .await?)
}

/// Cross-tenant: all users with this phone (used by the WhatsApp webhook to find the tenant).
pub async fn find_by_phone_admin(
    conn: &mut PgConnection,
    phone: &str,
) -> Result<Vec<UserRow>, DbError> {
    Ok(sqlx::query_as::<_, UserRow>(&format!("SELECT {COLS} FROM users WHERE phone_e164 = $1 AND status <> 'deactivated' ORDER BY created_at"))
        .bind(phone)
        .fetch_all(conn)
        .await?)
}

pub async fn find_by_employee_no(
    conn: &mut PgConnection,
    employee_no: &str,
) -> Result<Option<UserRow>, DbError> {
    Ok(
        sqlx::query_as::<_, UserRow>(&format!("SELECT {COLS} FROM users WHERE employee_no = $1"))
            .bind(employee_no)
            .fetch_optional(conn)
            .await?,
    )
}

#[derive(Debug, Default, Clone)]
pub struct UserFilter {
    pub q: Option<String>,
    pub role: Option<Role>,
    pub status: Option<UserStatus>,
    pub group_id: Option<Uuid>,
    pub language: Option<String>,
}

pub async fn list(
    conn: &mut PgConnection,
    filter: &UserFilter,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<Vec<UserRow>, DbError> {
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new(format!("SELECT {COLS} FROM users u WHERE true"));
    if let Some(q) = filter.q.as_deref().filter(|s| !s.trim().is_empty()) {
        qb.push(" AND (u.full_name ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(" OR u.phone_e164 LIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(" OR u.email ILIKE ");
        qb.push_bind(format!("%{}%", q.trim()));
        qb.push(")");
    }
    if let Some(r) = filter.role {
        qb.push(" AND u.role = ");
        qb.push_bind(r);
    }
    if let Some(s) = filter.status {
        qb.push(" AND u.status = ");
        qb.push_bind(s);
    }
    if let Some(g) = filter.group_id {
        qb.push(" AND EXISTS (SELECT 1 FROM group_members gm WHERE gm.user_id = u.id AND gm.group_id = ");
        qb.push_bind(g);
        qb.push(")");
    }
    if let Some(l) = filter.language.as_deref() {
        qb.push(" AND u.preferred_language = ");
        qb.push_bind(l.to_string());
    }
    push_cursor_filter(&mut qb, "u.created_at", "u.id", cursor);
    qb.push(" ORDER BY u.created_at DESC, u.id DESC LIMIT ");
    qb.push_bind(limit + 1);
    Ok(qb.build_query_as::<UserRow>().fetch_all(conn).await?)
}

pub struct NewUser<'a> {
    pub role: Role,
    pub status: UserStatus,
    pub full_name: &'a str,
    pub phone_e164: Option<&'a str>,
    pub email: Option<&'a str>,
    pub employee_no: Option<&'a str>,
    pub preferred_language: &'a str,
    pub ui_language: &'a str,
    pub manager_id: Option<Uuid>,
    pub location: Option<&'a str>,
    pub job_title: Option<&'a str>,
    pub delivery_channel: Channel,
    pub delivery_slot: DeliverySlot,
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    u: NewUser<'_>,
) -> Result<UserRow, DbError> {
    Ok(sqlx::query_as::<_, UserRow>(&format!(
        "INSERT INTO users (tenant_id, role, status, full_name, phone_e164, email, employee_no, preferred_language, ui_language, manager_id, location, job_title, delivery_channel, delivery_slot)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING {COLS}"
    ))
    .bind(tenant_id)
    .bind(u.role)
    .bind(u.status)
    .bind(u.full_name)
    .bind(u.phone_e164)
    .bind(u.email)
    .bind(u.employee_no)
    .bind(u.preferred_language)
    .bind(u.ui_language)
    .bind(u.manager_id)
    .bind(u.location)
    .bind(u.job_title)
    .bind(u.delivery_channel)
    .bind(u.delivery_slot)
    .fetch_one(conn)
    .await?)
}

/// Fields an admin may change (`UserPatch`) and a learner may change on themselves (`MePatch`).
#[derive(Debug, Default, Clone)]
pub struct UserUpdate {
    pub full_name: Option<String>,
    pub role: Option<Role>,
    pub status: Option<UserStatus>,
    pub preferred_language: Option<String>,
    pub ui_language: Option<String>,
    /// `Some(None)` clears the manager.
    pub manager_id: Option<Option<Uuid>>,
    pub location: Option<String>,
    pub job_title: Option<String>,
    pub delivery_channel: Option<Channel>,
    pub delivery_slot: Option<DeliverySlot>,
    pub captions_on: Option<bool>,
    pub reminders_on: Option<bool>,
    pub extra_reels_on: Option<bool>,
    pub paused_until: Option<Option<NaiveDate>>,
    pub phone_e164: Option<String>,
    pub email: Option<String>,
    pub employee_no: Option<String>,
}

pub async fn update(conn: &mut PgConnection, id: Uuid, p: UserUpdate) -> Result<UserRow, DbError> {
    let clear_manager = matches!(p.manager_id, Some(None));
    let manager = p.manager_id.flatten();
    let clear_pause = matches!(p.paused_until, Some(None));
    let paused = p.paused_until.flatten();
    Ok(sqlx::query_as::<_, UserRow>(&format!(
        "UPDATE users SET
           full_name = coalesce($2, full_name),
           role = coalesce($3, role),
           status = coalesce($4, status),
           preferred_language = coalesce($5, preferred_language),
           ui_language = coalesce($6, ui_language),
           manager_id = CASE WHEN $7 THEN NULL ELSE coalesce($8, manager_id) END,
           location = coalesce($9, location),
           job_title = coalesce($10, job_title),
           delivery_channel = coalesce($11, delivery_channel),
           delivery_slot = coalesce($12, delivery_slot),
           captions_on = coalesce($13, captions_on),
           reminders_on = coalesce($14, reminders_on),
           extra_reels_on = coalesce($15, extra_reels_on),
           paused_until = CASE WHEN $16 THEN NULL ELSE coalesce($17, paused_until) END,
           phone_e164 = coalesce($18, phone_e164),
           email = coalesce($19, email),
           employee_no = coalesce($20, employee_no)
         WHERE id = $1 RETURNING {COLS}"
    ))
    .bind(id)
    .bind(p.full_name)
    .bind(p.role)
    .bind(p.status)
    .bind(p.preferred_language)
    .bind(p.ui_language)
    .bind(clear_manager)
    .bind(manager)
    .bind(p.location)
    .bind(p.job_title)
    .bind(p.delivery_channel)
    .bind(p.delivery_slot)
    .bind(p.captions_on)
    .bind(p.reminders_on)
    .bind(p.extra_reels_on)
    .bind(clear_pause)
    .bind(paused)
    .bind(p.phone_e164)
    .bind(p.email)
    .bind(p.employee_no)
    .fetch_one(conn)
    .await?)
}

pub async fn touch_last_seen(conn: &mut PgConnection, id: Uuid) -> Result<(), DbError> {
    sqlx::query("UPDATE users SET last_seen_at = now(), status = CASE WHEN status = 'invited' THEN 'active'::user_status_t ELSE status END WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn groups_of(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<GroupRef>, DbError> {
    Ok(sqlx::query_as::<_, GroupRef>(
        "SELECT g.id, g.name FROM group_members gm JOIN groups g ON g.id = gm.group_id WHERE gm.user_id = $1 ORDER BY g.name",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

pub async fn set_groups(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    group_ids: &[Uuid],
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM group_members WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    for g in group_ids {
        sqlx::query("INSERT INTO group_members (group_id, user_id, tenant_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(g)
            .bind(user_id)
            .bind(tenant_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn add_to_group(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO group_members (group_id, user_id, tenant_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(group_id)
        .bind(user_id)
        .bind(tenant_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Direct reports of a manager.
pub async fn team_of(conn: &mut PgConnection, manager_id: Uuid) -> Result<Vec<UserRow>, DbError> {
    Ok(sqlx::query_as::<_, UserRow>(&format!("SELECT {COLS} FROM users WHERE manager_id = $1 AND status <> 'deactivated' ORDER BY full_name"))
        .bind(manager_id)
        .fetch_all(conn)
        .await?)
}

/// Active and invited learners (everyone who gets a daily plan).
pub async fn plan_recipients(conn: &mut PgConnection) -> Result<Vec<UserRow>, DbError> {
    Ok(sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {COLS} FROM users WHERE status IN ('invited', 'active') ORDER BY created_at"
    ))
    .fetch_all(conn)
    .await?)
}

pub async fn count_learners(conn: &mut PgConnection) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE status IN ('invited','active','paused')",
    )
    .fetch_one(conn)
    .await?)
}

pub async fn count_active_since(conn: &mut PgConnection, since: NaiveDate) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM user_stats WHERE last_active_on >= $1")
            .bind(since)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn upsert_device(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    device_id: &str,
    platform: &str,
    app_version: Option<&str>,
    push_token: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO devices (tenant_id, user_id, device_id, platform, app_version, push_token)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, device_id) DO UPDATE SET platform = EXCLUDED.platform, app_version = coalesce(EXCLUDED.app_version, devices.app_version), push_token = coalesce(EXCLUDED.push_token, devices.push_token), last_seen_at = now()",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(device_id)
    .bind(platform)
    .bind(app_version)
    .bind(push_token)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn set_push_token(
    conn: &mut PgConnection,
    user_id: Uuid,
    token: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE devices SET push_token = $2, last_seen_at = now() WHERE user_id = $1")
        .bind(user_id)
        .bind(token)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn has_devices(conn: &mut PgConnection, user_id: Uuid) -> Result<bool, DbError> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM devices WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(conn)
            .await?
            > 0,
    )
}
