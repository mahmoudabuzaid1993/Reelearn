//! Wire types: field names and shapes follow `openapi/openapi.yaml` exactly.

use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Page<T: Serialize> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PageQuery {
    pub cursor: Option<String>,
    /// Accepts `limit=25` from a query string even when this struct is `#[serde(flatten)]`ed
    /// into a filter (flatten buffers values as strings, which would reject plain numbers).
    #[serde(default, deserialize_with = "de_opt_i64")]
    pub limit: Option<i64>,
}

pub(crate) fn de_opt_i64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Int(i64),
        Str(String),
    }
    Ok(match Option::<Raw>::deserialize(d)? {
        None => None,
        Some(Raw::Int(i)) => Some(i),
        Some(Raw::Str(s)) if s.trim().is_empty() => None,
        Some(Raw::Str(s)) => Some(s.trim().parse().map_err(serde::de::Error::custom)?),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPublic {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub languages: Vec<String>,
    pub region: Region,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tenant {
    #[serde(flatten)]
    pub public: TenantPublic,
    pub default_language: String,
    pub timezone: String,
    pub credits: i32,
    pub settings: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TenantPatch {
    pub name: Option<String>,
    pub languages: Option<Vec<String>>,
    pub default_language: Option<String>,
    pub timezone: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Me {
    pub id: Uuid,
    pub tenant: TenantPublic,
    pub role: Role,
    pub full_name: String,
    pub first_name: String,
    pub phone_e164: Option<String>,
    pub email: Option<String>,
    pub preferred_language: String,
    pub ui_language: String,
    pub delivery_channel: Channel,
    pub delivery_slot: DeliverySlot,
    pub captions_on: bool,
    pub reminders_on: bool,
    pub extra_reels_on: bool,
    pub paused_until: Option<NaiveDate>,
    pub is_first_open: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct MePatch {
    pub preferred_language: Option<String>,
    pub ui_language: Option<String>,
    pub delivery_channel: Option<Channel>,
    pub delivery_slot: Option<DeliverySlot>,
    pub captions_on: Option<bool>,
    pub reminders_on: Option<bool>,
    pub extra_reels_on: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub paused_until: Option<Option<NaiveDate>>,
    pub push_token: Option<String>,
}

/// Distinguishes a missing key (`None`) from an explicit `null` (`Some(None)`).
pub fn deserialize_double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: Me,
}

#[derive(Debug, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub platform: String,
    pub app_version: Option<String>,
    pub push_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupRef {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub role: Role,
    pub status: UserStatus,
    pub full_name: String,
    pub phone_e164: Option<String>,
    pub email: Option<String>,
    pub employee_no: Option<String>,
    pub preferred_language: String,
    pub manager_id: Option<Uuid>,
    pub location: Option<String>,
    pub job_title: Option<String>,
    pub delivery_channel: Channel,
    pub delivery_slot: DeliverySlot,
    pub groups: Vec<GroupRef>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UserCreate {
    pub full_name: String,
    pub role: Role,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub employee_no: Option<String>,
    pub preferred_language: Option<String>,
    pub manager_id: Option<Uuid>,
    pub location: Option<String>,
    pub job_title: Option<String>,
    pub delivery_channel: Option<Channel>,
    pub delivery_slot: Option<DeliverySlot>,
    pub group_ids: Option<Vec<Uuid>>,
    pub send_invite: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UserPatch {
    pub full_name: Option<String>,
    pub role: Option<Role>,
    pub status: Option<UserStatus>,
    pub preferred_language: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub manager_id: Option<Option<Uuid>>,
    pub location: Option<String>,
    pub job_title: Option<String>,
    pub delivery_channel: Option<Channel>,
    pub delivery_slot: Option<DeliverySlot>,
    pub group_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct ImportError {
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub total_rows: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<ImportError>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub member_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct GroupCreate {
    pub name: String,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MemberList {
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ReelCounts {
    pub published: i64,
    pub in_review: i64,
    pub draft: i64,
}

#[derive(Debug, Serialize)]
pub struct Topic {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub name_ar: Option<String>,
    pub description: Option<String>,
    pub compliance_tag: Option<String>,
    pub is_mandatory: bool,
    pub status: String,
    pub reel_counts: ReelCounts,
    pub recall_pct: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct TopicCreate {
    pub name: String,
    pub name_ar: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub compliance_tag: Option<String>,
    pub is_mandatory: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Assignment {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub group: GroupRef,
    pub reels_per_day: i16,
    pub channel: Channel,
    pub is_mandatory: bool,
    pub starts_on: NaiveDate,
    pub ends_on: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentWrite {
    pub group_id: Uuid,
    pub reels_per_day: Option<i16>,
    pub channel: Option<Channel>,
    pub is_mandatory: Option<bool>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct UploadSlot {
    pub id: Uuid,
    pub upload_url: String,
    pub method: &'static str,
    pub headers: std::collections::BTreeMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub filename: String,
    pub mime: String,
    pub bytes: i64,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceCreate {
    pub kind: SourceKind,
    pub title: String,
    pub url: Option<String>,
    pub media_asset_id: Option<Uuid>,
    pub prompt: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Source {
    pub id: Uuid,
    pub kind: SourceKind,
    pub title: String,
    pub url: Option<String>,
    pub language: Option<String>,
    pub extraction_status: ExtractionStatus,
    pub extraction_error: Option<String>,
    pub summary: Option<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Moment {
    pub id: Uuid,
    pub start_ms: i32,
    pub end_ms: i32,
    pub excerpt: String,
    pub title: Option<String>,
    pub speaker: Option<String>,
    pub score: f32,
    pub selected: bool,
    pub reel_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CutRequest {
    pub topic_id: Uuid,
    pub start_ms: Option<i32>,
    pub end_ms: Option<i32>,
    pub title: Option<String>,
    pub speaker_consent: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerationRequest {
    pub source_id: Uuid,
    pub topic_id: Uuid,
    pub languages: Vec<String>,
    pub reel_count: i32,
    pub style: GenerationStyle,
    pub presenter: Option<serde_json::Value>,
    pub reviewer_id: Option<Uuid>,
    pub brief: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub cost_credits: i32,
    pub credits_available: i32,
    pub estimated_minutes: i32,
    pub breakdown: Vec<reelearn_domain::pricing::QuoteLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationStep {
    pub position: i16,
    pub step: GenStep,
    pub status: JobStatus,
    pub provider: Option<String>,
    pub attempt: i16,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationJob {
    pub id: Uuid,
    pub source_id: Uuid,
    pub topic_id: Option<Uuid>,
    pub status: JobStatus,
    pub progress_pct: i16,
    pub cost_credits: i32,
    pub params: serde_json::Value,
    pub error: Option<String>,
    pub steps: Vec<GenerationStep>,
    pub reel_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: Uuid,
    pub language: String,
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    /// Present only in admin responses, never in learner payloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct_key: Option<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionWrite {
    pub prompt: Option<String>,
    pub options: Option<Vec<QuestionOption>>,
    pub correct_key: Option<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VariantView {
    pub language: String,
    pub dub_status: ExtractionStatus,
    pub video_url: Option<String>,
    pub captions_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovalView {
    pub role: ApprovalRole,
    pub decision: Decision,
    pub reviewer_name: String,
    pub comment: Option<String>,
    pub decided_at: DateTime<Utc>,
    pub reel_version: i32,
}

#[derive(Debug, Serialize)]
pub struct Reel {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub source_id: Option<Uuid>,
    pub generation_job_id: Option<Uuid>,
    pub kind: ReelKind,
    pub status: ReelStatus,
    pub title: String,
    pub language: String,
    pub script: Option<String>,
    pub source_citation: Option<String>,
    pub presenter: serde_json::Value,
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub captions_url: Option<String>,
    pub duration_ms: Option<i32>,
    pub version: i32,
    pub compliance_tag: Option<String>,
    pub consent_confirmed: bool,
    pub variants: Vec<VariantView>,
    pub question: Option<Question>,
    pub approvals: Vec<ApprovalView>,
    pub audience: Vec<GroupRef>,
    pub recall_pct: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TopicRef {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ReelListItem {
    pub id: Uuid,
    pub topic: TopicRef,
    pub kind: ReelKind,
    pub status: ReelStatus,
    pub title: String,
    pub language: String,
    pub languages: Vec<String>,
    pub duration_ms: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub version: i32,
    pub recall_pct: Option<i32>,
    pub plays: i64,
    pub awaiting: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ReelPatch {
    pub title: Option<String>,
    pub script: Option<String>,
    pub source_citation: Option<String>,
    pub presenter: Option<serde_json::Value>,
    pub compliance_tag: Option<String>,
    pub question: Option<QuestionWrite>,
}

#[derive(Debug, Serialize)]
pub struct ReelCard {
    pub id: Uuid,
    pub title: String,
    pub topic_name: String,
    pub presenter_name: Option<String>,
    pub duration_ms: i32,
    pub language: String,
    pub thumbnail_url: Option<String>,
    pub saved: bool,
    pub seen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReelPlayable {
    pub id: Uuid,
    pub title: String,
    pub topic_name: String,
    pub presenter_name: Option<String>,
    pub language: String,
    pub duration_ms: i32,
    pub video_url: String,
    pub captions_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub captions_on_by_default: bool,
    pub caption_text: Option<String>,
    pub source_citation: Option<String>,
    pub why_am_i_seeing_this: String,
    pub question: Option<Question>,
    pub saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub streak_current: i32,
    pub streak_longest: i32,
    pub freezes_available: i16,
    pub remembered_count: i32,
    pub reels_completed: i32,
    pub last_active_on: Option<NaiveDate>,
}

impl Default for UserStats {
    fn default() -> Self {
        UserStats {
            streak_current: 0,
            streak_longest: 0,
            freezes_available: 1,
            remembered_count: 0,
            reels_completed: 0,
            last_active_on: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPlanItem {
    pub id: Uuid,
    pub position: i16,
    pub kind: PlanItemKind,
    pub reason: PlanItemReason,
    pub state: PlanItemState,
    pub reel: Option<ReelPlayable>,
    pub question: Option<Question>,
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Yesterday {
    pub remembered: i64,
    pub total: i64,
    pub next_review_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPlan {
    pub id: Uuid,
    pub plan_date: NaiveDate,
    pub status: String,
    pub greeting_name: String,
    pub estimated_seconds: i32,
    pub items: Vec<DailyPlanItem>,
    pub stats: UserStats,
    pub yesterday: Option<Yesterday>,
}

#[derive(Debug, Deserialize)]
pub struct WatchRequest {
    pub plan_item_id: Option<Uuid>,
    pub reel_id: Uuid,
    pub watched_ms: i32,
    pub completed: bool,
    pub channel: Option<Channel>,
}

#[derive(Debug, Deserialize)]
pub struct AnswerRequest {
    pub plan_item_id: Option<Uuid>,
    pub question_id: Uuid,
    pub chosen_key: String,
    pub response_ms: Option<i32>,
    pub channel: Option<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerOutcome {
    pub is_correct: bool,
    pub correct_key: String,
    pub explanation: Option<String>,
    pub next_due_on: NaiveDate,
    pub plan_status: String,
    pub stats: UserStats,
}

#[derive(Debug, Serialize)]
pub struct ProgressWeek {
    pub week: String,
    pub remembered_pct: i32,
}

#[derive(Debug, Serialize)]
pub struct ProgressTopic {
    pub topic_id: Uuid,
    pub name: String,
    pub recall_pct: i32,
    pub next_review_on: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct Milestone {
    pub key: String,
    pub label: String,
    pub reached: bool,
}

#[derive(Debug, Serialize)]
pub struct Progress {
    pub remembered_pct: i32,
    pub series: Vec<ProgressWeek>,
    pub by_topic: Vec<ProgressTopic>,
    pub milestones: Vec<Milestone>,
    pub stats: UserStats,
}

#[derive(Debug, Serialize)]
pub struct Privacy {
    pub manager_sees: Vec<String>,
    pub hr_sees: Vec<String>,
    pub nobody_sees: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TeamTopic {
    pub id: Uuid,
    pub name: String,
    pub ready_count: i64,
}

#[derive(Debug, Serialize)]
pub struct MemberReadiness {
    pub topic_id: Uuid,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub user_id: Uuid,
    pub full_name: String,
    pub language: String,
    pub last_active_on: Option<NaiveDate>,
    pub nudged_today: bool,
    pub readiness: Vec<MemberReadiness>,
}

#[derive(Debug, Serialize)]
pub struct TeamReadiness {
    pub team_recall_pct: i32,
    pub active_this_week: i64,
    pub team_size: i64,
    pub topics: Vec<TeamTopic>,
    pub members: Vec<TeamMember>,
}

#[derive(Debug, Deserialize)]
pub struct NudgeRequest {
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct NudgeResponse {
    pub queued: usize,
    pub skipped_already_nudged: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attention {
    pub kind: String,
    pub text: String,
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardPoint {
    pub day: NaiveDate,
    pub active: i64,
    pub recall_pct: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dashboard {
    pub weekly_active_pct: i32,
    pub recall_pct: i32,
    pub learners_total: i64,
    pub learners_active: i64,
    pub hrdf_hours: f64,
    pub credits: i32,
    pub attention: Vec<Attention>,
    pub series: Vec<DashboardPoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportOverview {
    pub learners_active: i64,
    pub reels_completed: i64,
    pub answers_total: i64,
    pub recall_pct: i32,
    pub minutes_on_mandatory: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportRow {
    pub id: Uuid,
    pub name: String,
    pub learners_active: i64,
    pub learners_total: i64,
    pub recall_pct: i32,
    pub reels_completed: i64,
    pub minutes_total: f64,
    pub ready_pct: i32,
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    pub range: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Export {
    pub id: Uuid,
    pub kind: ExportKind,
    pub status: JobStatus,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub kind: ExportKind,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct Integration {
    pub kind: IntegrationKind,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub public_config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AuditActor {
    pub id: Uuid,
    pub full_name: String,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor: Option<AuditActor>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub diff: Option<serde_json::Value>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TenantCreate {
    pub company_name: String,
    pub slug: String,
    pub admin_name: String,
    pub admin_email: String,
    pub admin_phone: Option<String>,
    pub default_language: String,
    pub region: Region,
    pub sector: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TenantCreated {
    pub tenant: TenantPublic,
    pub challenge_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RecordingRequest {
    pub topic_id: Uuid,
    pub title: Option<String>,
    pub mime: String,
    pub bytes: i64,
    pub consent_confirmed: bool,
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub group_ids: Vec<Uuid>,
    pub starts_on: Option<NaiveDate>,
    pub compliance_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub decision: Decision,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SubmitReviewRequest {
    pub sme_user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RegenerateRequest {
    pub what: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct VariantsRequest {
    pub languages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReelRequestCreate {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub postgres: bool,
    pub redis: bool,
    pub version: String,
}

pub fn parse_options(v: &serde_json::Value) -> Vec<QuestionOption> {
    serde_json::from_value(v.clone()).unwrap_or_default()
}
