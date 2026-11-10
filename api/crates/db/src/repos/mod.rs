//! Repositories: the only SQL in the workspace, one module per aggregate.
//! Every function takes a `&mut PgConnection` obtained from [`crate::TenantTx::conn`]
//! (tenant rows) or from [`crate::Db::admin_tx`] (cross-tenant lookups, explicitly named `*_admin`).

pub mod audit;
pub mod auth;
pub mod deliveries;
pub mod engine;
pub mod exports;
pub mod groups;
pub mod integrations;
pub mod jobs;
pub mod learning;
pub mod media;
pub mod plans;
pub mod reels;
pub mod stats;
pub mod studio;
pub mod tenants;
pub mod topics;
pub mod users;

use reelearn_domain::cursor::Cursor;

/// Shared keyset-pagination helper for `ORDER BY ts DESC, id DESC` lists.
pub(crate) fn push_cursor_filter<'a>(
    qb: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>,
    column_ts: &str,
    column_id: &str,
    cursor: Option<Cursor>,
) {
    if let Some(c) = cursor {
        qb.push(format!(" AND ({column_ts}, {column_id}) < ("));
        qb.push_bind(c.ts);
        qb.push(", ");
        qb.push_bind(c.id);
        qb.push(")");
    }
}
