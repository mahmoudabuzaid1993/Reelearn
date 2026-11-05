use crate::error::DbError;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupRow {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub member_count: i64,
}

const SQL_GROUPS: &str = "SELECT g.id, g.name, g.kind, (SELECT count(*) FROM group_members gm WHERE gm.group_id = g.id) AS member_count FROM groups g";

pub async fn list(conn: &mut PgConnection) -> Result<Vec<GroupRow>, DbError> {
    Ok(
        sqlx::query_as::<_, GroupRow>(&format!("{SQL_GROUPS} ORDER BY g.name"))
            .fetch_all(conn)
            .await?,
    )
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<GroupRow, DbError> {
    sqlx::query_as::<_, GroupRow>(&format!("{SQL_GROUPS} WHERE g.id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound("group"))
}

pub async fn find_by_name(
    conn: &mut PgConnection,
    name: &str,
) -> Result<Option<GroupRow>, DbError> {
    Ok(
        sqlx::query_as::<_, GroupRow>(&format!("{SQL_GROUPS} WHERE g.name = $1"))
            .bind(name)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    name: &str,
    kind: &str,
) -> Result<GroupRow, DbError> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO groups (tenant_id, name, kind) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(name)
    .bind(kind)
    .fetch_one(&mut *conn)
    .await?;
    get(conn, id).await
}

pub async fn replace_members(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    group_id: Uuid,
    user_ids: &[Uuid],
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM group_members WHERE group_id = $1")
        .bind(group_id)
        .execute(&mut *conn)
        .await?;
    for u in user_ids {
        sqlx::query("INSERT INTO group_members (group_id, user_id, tenant_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(group_id)
            .bind(u)
            .bind(tenant_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn member_ids(conn: &mut PgConnection, group_id: Uuid) -> Result<Vec<Uuid>, DbError> {
    Ok(
        sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(conn)
            .await?,
    )
}

pub async fn ids_exist(conn: &mut PgConnection, ids: &[Uuid]) -> Result<bool, DbError> {
    if ids.is_empty() {
        return Ok(true);
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM groups WHERE id = ANY($1)")
        .bind(ids)
        .fetch_one(conn)
        .await?;
    Ok(usize::try_from(n).unwrap_or(0) == ids.len())
}
