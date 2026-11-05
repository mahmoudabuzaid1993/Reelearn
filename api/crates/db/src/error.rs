use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("row not found: {0}")]
    NotFound(&'static str),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(
        "reel needs an approved L&D and SME sign-off on its current version before publishing"
    )]
    DualApprovalRequired,
    #[error("not enough credits: need {needed}, have {available}")]
    InsufficientCredits { needed: i32, available: i32 },
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Sqlx(sqlx::Error),
    #[error("migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e {
            let message = db.message().to_string();
            if message.contains("needs an approved L&D and SME sign-off") {
                return DbError::DualApprovalRequired;
            }
            if db.is_unique_violation() {
                return DbError::Conflict(unique_violation_message(db.constraint()));
            }
            if db.is_check_violation() {
                return DbError::Invalid(format!(
                    "constraint {} violated",
                    db.constraint().unwrap_or("?")
                ));
            }
        }
        if matches!(e, sqlx::Error::RowNotFound) {
            return DbError::NotFound("row");
        }
        DbError::Sqlx(e)
    }
}

fn unique_violation_message(constraint: Option<&str>) -> String {
    match constraint {
        Some("tenants_slug_uq") => "a workspace with this slug already exists".into(),
        Some("users_tenant_phone_uq") => "a person with this phone number already exists".into(),
        Some("users_tenant_email_uq") => "a person with this email already exists".into(),
        Some("users_tenant_employee_no_uq") => {
            "a person with this employee number already exists".into()
        }
        Some("topics_tenant_slug_uq") => "a topic with this slug already exists".into(),
        Some("groups_tenant_name_uq") => "a group with this name already exists".into(),
        Some("questions_reel_id_language_key") => {
            "this reel already has a question in that language".into()
        }
        Some(c) => format!("already exists ({c})"),
        None => "already exists".into(),
    }
}
