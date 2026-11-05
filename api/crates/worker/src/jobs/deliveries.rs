//! `send_delivery`: everything that leaves the platform towards a person.
//!
//! Payload shapes (all enqueued by the API or the scheduler):
//! - `{ delivery_id, kind: "daily_minute" | "nudge" | "reminder" }`   a scheduled `deliveries` row
//! - `{ delivery_id, kind: "whatsapp_message", to, body, video_key }` a bot reply
//! - `{ kind: "invite", user_id }`                                     first invite after import
//! - `{ kind: "review_request", user_id, reel_id }`                    SME sign-off request
//! - `{ kind: "otp", channel, to, code }`                              one-time code
//!
//! WhatsApp goes through the `WhatsAppSender` provider. App push and email are
//! recorded as sent-by-log until a push/email provider is configured (see README).

use crate::jobs::{JobError, JobOutcome};
use crate::providers::ProviderError;
use crate::Ctx;
use chrono::Utc;
use reelearn_api::cache::keys;
use reelearn_db::repos::{deliveries, plans, reels, tenants, users};
use reelearn_domain::{Channel, DeliveryStatus};
use uuid::Uuid;

fn s(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn uuid(payload: &serde_json::Value, key: &str) -> Option<Uuid> {
    s(payload, key).and_then(|v| v.parse().ok())
}

pub async fn send_delivery(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let kind = s(payload, "kind").unwrap_or_default();
    match kind.as_str() {
        "otp" => send_otp(ctx, payload).await,
        "invite" => send_invite(ctx, tenant_id, payload).await,
        "review_request" => send_review_request(ctx, tenant_id, payload).await,
        "whatsapp_message" | "daily_minute" | "nudge" | "reminder" | "bot_reply" => {
            send_scheduled(ctx, tenant_id, payload, &kind).await
        }
        other => Err(JobError::Permanent(format!(
            "unknown delivery kind {other}"
        ))),
    }
}

async fn send_otp(ctx: &Ctx, payload: &serde_json::Value) -> Result<JobOutcome, JobError> {
    let to = s(payload, "to").ok_or_else(|| JobError::Permanent("otp without recipient".into()))?;
    let code = s(payload, "code").ok_or_else(|| JobError::Permanent("otp without code".into()))?;
    let channel = s(payload, "channel").unwrap_or_else(|| "whatsapp".into());
    if channel == "whatsapp" {
        let id = ctx.providers.whatsapp.send_text(&to, &format!("Your Reelearn code is {code}. It expires in 5 minutes. · رمز دخولك إلى ريليرن هو {code}")).await.map_err(provider_err)?;
        return Ok(JobOutcome::done(format!("otp via whatsapp → {id}")));
    }
    tracing::info!(to = %to, channel = %channel, "OTP delivery (no email/SMS provider configured; code logged in development only)");
    Ok(JobOutcome::done(format!("otp via {channel} recorded")))
}

async fn send_invite(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let user_id = uuid(payload, "user_id")
        .ok_or_else(|| JobError::Permanent("invite without user_id".into()))?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let tenant = tenants::get(tx.conn(), tenant_id).await?;
    let user = users::get(tx.conn(), user_id).await?;
    let Some(phone) = user.phone_e164.clone() else {
        tx.commit().await?;
        return Ok(JobOutcome::done("invite skipped: no phone".into()));
    };
    let link = format!(
        "{}/t/{}?inv=welcome",
        ctx.state.config.public_base_url.trim_end_matches('/'),
        tenant.slug
    );
    let body = match user.preferred_language.as_str() {
        "ar" => format!("أهلاً {} 👋 {} أضافتك إلى ريليرن: دقيقة كل يوم عن عملك، على هذا الرقم، في وقت الدوام. أرسلي ابدأ لتصلك أول ريل، أو افتحي التطبيق: {link}", user.first_name(), tenant.name),
        _ => format!("Hi {} 👋 {} added you to Reelearn: one minute a day on your job, on this number, during your shift. Reply START for today's first reel, or open the app: {link}", user.first_name(), tenant.name),
    };
    let delivery = deliveries::schedule(
        tx.conn(),
        tenant_id,
        user.id,
        None,
        Channel::Whatsapp,
        "invite",
        Utc::now(),
    )
    .await?;
    tx.commit().await?;
    deliver_whatsapp(ctx, tenant_id, delivery.id, &phone, &body, None).await
}

async fn send_review_request(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let user_id = uuid(payload, "user_id")
        .ok_or_else(|| JobError::Permanent("review request without user_id".into()))?;
    let reel_id = uuid(payload, "reel_id")
        .ok_or_else(|| JobError::Permanent("review request without reel_id".into()))?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let user = users::get(tx.conn(), user_id).await?;
    let reel = reels::get(tx.conn(), reel_id).await?;
    let link = format!(
        "{}/studio/reels/{}/publish",
        ctx.state.config.public_base_url.trim_end_matches('/'),
        reel.id
    );
    let body = format!(
        "Hi {}, a reel is waiting for your sign-off: “{}”. Review it here: {link}",
        user.first_name(),
        reel.title
    );
    let channel = if user.phone_e164.is_some() {
        Channel::Whatsapp
    } else {
        Channel::App
    };
    let delivery = deliveries::schedule(
        tx.conn(),
        tenant_id,
        user.id,
        None,
        channel,
        "review_request",
        Utc::now(),
    )
    .await?;
    tx.commit().await?;
    match user.phone_e164 {
        Some(phone) => deliver_whatsapp(ctx, tenant_id, delivery.id, &phone, &body, None).await,
        None => mark_logged(ctx, tenant_id, delivery.id, "email").await,
    }
}

async fn send_scheduled(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
    kind: &str,
) -> Result<JobOutcome, JobError> {
    let delivery_id = uuid(payload, "delivery_id")
        .ok_or_else(|| JobError::Permanent(format!("{kind} without delivery_id")))?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let Some(delivery) = deliveries::get(tx.conn(), delivery_id).await? else {
        tx.commit().await?;
        return Ok(JobOutcome::done(
            "delivery row gone; nothing to send".into(),
        ));
    };
    if delivery.status != DeliveryStatus::Scheduled {
        tx.commit().await?;
        return Ok(JobOutcome::done(format!(
            "delivery already {}",
            delivery.status.as_str()
        )));
    }
    let user = users::get(tx.conn(), delivery.user_id).await?;
    let (body, video_key) = match kind {
        "whatsapp_message" | "bot_reply" => (s(payload, "body").unwrap_or_default(), s(payload, "video_key")),
        "nudge" => (
            match user.preferred_language.as_str() {
                "ar" => format!("{}، دقيقتك اليوم لم تبدأ بعد. ثلاث ريلات قصيرة وسؤال واحد لكل منها. أرسل ابدأ.", user.first_name()),
                _ => format!("{}, today's minute is still waiting: three short reels, one question each. Reply START.", user.first_name()),
            },
            None,
        ),
        "reminder" => (format!("{}, a quick reminder: your minute for today is ready whenever you are.", user.first_name()), None),
        _ => daily_minute_message(&mut tx, &user, delivery.plan_id).await?,
    };
    tx.commit().await?;
    match (
        user.phone_e164.clone(),
        delivery.channel.includes_whatsapp(),
    ) {
        (Some(phone), true) => {
            deliver_whatsapp(
                ctx,
                tenant_id,
                delivery.id,
                &phone,
                &body,
                video_key.as_deref(),
            )
            .await
        }
        _ => mark_logged(ctx, tenant_id, delivery.id, delivery.channel.as_str()).await,
    }
}

async fn daily_minute_message(
    tx: &mut reelearn_db::TenantTx,
    user: &users::UserRow,
    plan_id: Option<Uuid>,
) -> Result<(String, Option<String>), JobError> {
    let Some(plan_id) = plan_id else {
        return Ok((
            format!(
                "{}, your minute for today is ready. Reply START.",
                user.first_name()
            ),
            None,
        ));
    };
    let items = plans::items(tx.conn(), plan_id).await?;
    let reel_count = items.iter().filter(|i| i.reel_id.is_some()).count();
    let first_video = match items.iter().find_map(|i| i.reel_id) {
        Some(rid) => reels::find(tx.conn(), rid).await?.and_then(|r| r.video_key),
        None => None,
    };
    let body = match user.preferred_language.as_str() {
        "ar" => format!("صباح الخير {}. دقيقتك اليوم: {} ريل وسؤال واحد لكل منها، حوالي دقيقة. أرسل ابدأ.", user.first_name(), reel_count),
        "ur" => format!("صبح بخیر {}۔ آج کی آپ کی منٹ: {} ریلز، ہر ایک کے بعد ایک سوال، تقریباً ایک منٹ۔ ابدأ لکھیں۔", user.first_name(), reel_count),
        _ => format!("Good morning {}. Today's minute: {} reels, one question each, about a minute. Reply START.", user.first_name(), reel_count),
    };
    Ok((body, first_video))
}

async fn deliver_whatsapp(
    ctx: &Ctx,
    tenant_id: Uuid,
    delivery_id: Uuid,
    phone: &str,
    body: &str,
    video_key: Option<&str>,
) -> Result<JobOutcome, JobError> {
    let result = match video_key {
        Some(key) => {
            let url = ctx.state.storage.presign_get(key);
            ctx.state
                .cache
                .set_string(
                    &keys::media(key),
                    &url,
                    std::time::Duration::from_secs(50 * 60),
                )
                .await;
            ctx.providers.whatsapp.send_video(phone, body, &url).await
        }
        None => ctx.providers.whatsapp.send_text(phone, body).await,
    };
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    match result {
        Ok(provider_ref) => {
            deliveries::mark(
                tx.conn(),
                delivery_id,
                DeliveryStatus::Sent,
                Some(&provider_ref),
                None,
            )
            .await?;
            deliveries::log_whatsapp(
                tx.conn(),
                tenant_id,
                None,
                phone,
                "out",
                Some(body),
                None,
                Some(&provider_ref),
            )
            .await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!("whatsapp → {provider_ref}")))
        }
        Err(e) if e.is_transient() => {
            tx.rollback().await?;
            Err(JobError::Transient(e.to_string()))
        }
        Err(e) => {
            deliveries::mark(
                tx.conn(),
                delivery_id,
                DeliveryStatus::Failed,
                None,
                Some(&e.to_string()),
            )
            .await?;
            tx.commit().await?;
            Err(JobError::Permanent(e.to_string()))
        }
    }
}

async fn mark_logged(
    ctx: &Ctx,
    tenant_id: Uuid,
    delivery_id: Uuid,
    channel: &str,
) -> Result<JobOutcome, JobError> {
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    deliveries::mark(
        tx.conn(),
        delivery_id,
        DeliveryStatus::Sent,
        Some("logged"),
        None,
    )
    .await?;
    tx.commit().await?;
    tracing::info!(%delivery_id, channel, "delivery recorded; no push/email provider configured");
    Ok(JobOutcome::done(format!(
        "{channel} delivery recorded (no provider)"
    )))
}

fn provider_err(e: ProviderError) -> JobError {
    if e.is_transient() {
        JobError::Transient(e.to_string())
    } else {
        JobError::Permanent(e.to_string())
    }
}
