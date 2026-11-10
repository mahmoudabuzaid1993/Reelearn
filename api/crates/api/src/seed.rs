//! Demo tenant "Al Wadi Retail" (`alwadi`). The web and mobile teams build against these exact values.
//! Idempotent: a second run reports the existing tenant and changes nothing.

use crate::error::ApiError;
use crate::s3::reel_media_key;
use crate::state::AppState;
use chrono::{Duration, NaiveDate, Utc};
use reelearn_db::repos::{groups, learning, reels, stats, tenants, topics, users};
use reelearn_db::settings::{local_today, TenantSettings};
use reelearn_db::TenantTx;
use reelearn_domain::recall::{self, MemoryState};
use reelearn_domain::*;
use uuid::Uuid;

pub const TENANT_SLUG: &str = "alwadi";

struct ReelSpec {
    topic: &'static str,
    title: &'static str,
    script: &'static str,
    citation: &'static str,
    status: ReelStatus,
    duration_ms: i32,
    days_ago: i32,
    q_en: (&'static str, [&'static str; 3], &'static str, &'static str),
    q_ar: (&'static str, [&'static str; 3], &'static str, &'static str),
}

const REELS: &[ReelSpec] = &[
    ReelSpec {
        topic: "customer-service",
        title: "Say the customer's name twice in the first minute",
        script: "Use the customer's name once when you greet them and once when you say goodbye. Twice is warm; every sentence is too much.",
        citation: "Al Wadi service playbook, p. 4",
        status: ReelStatus::Published,
        duration_ms: 34_000,
        days_ago: 20,
        q_en: ("A customer, Fatimah, is paying. When do you use her name?", ["Only at checkout", "Once at hello, once at goodbye", "Every sentence"], "B", "Twice feels personal without sounding scripted."),
        q_ar: ("عميلة اسمها فاطمة تدفع الآن. متى تستخدم اسمها؟", ["عند الدفع فقط", "مرة عند الترحيب ومرة عند الوداع", "في كل جملة"], "B", "مرتان تمنحان شعوراً شخصياً دون تكلف."),
    },
    ReelSpec {
        topic: "customer-service",
        title: "Greet within 10 seconds",
        script: "When a customer walks in, pause what you are doing and greet them within ten seconds. A shelf can wait; a person cannot.",
        citation: "Al Wadi service playbook, p. 2",
        status: ReelStatus::Published,
        duration_ms: 30_000,
        days_ago: 18,
        q_en: ("A customer walks in while you restock a shelf. What do you do first?", ["Finish the shelf", "Greet within 10 seconds", "Wait until they ask for help"], "B", "The shelf waits. The greeting sets the whole visit."),
        q_ar: ("يدخل عميل بينما ترتب الرف. ما أول ما تفعله؟", ["أكمل ترتيب الرف", "أرحب به خلال ١٠ ثوانٍ", "أنتظر حتى يطلب المساعدة"], "B", "الرف ينتظر، والترحيب يحدد الزيارة كلها."),
    },
    ReelSpec {
        topic: "customer-service",
        title: "Turn 'no' into 'here's what I can do'",
        script: "Never end on a no. If an item is out of stock, say what you can do: check the other branch, order it, or suggest an alternative.",
        citation: "Al Wadi service playbook, p. 7",
        status: ReelStatus::Published,
        duration_ms: 38_000,
        days_ago: 15,
        q_en: ("An item is out of stock. What is the best reply?", ["We don't have it", "Here's what I can do: let me check the other branch", "Come back next week"], "B", "Offering the next step keeps the customer with you."),
        q_ar: ("المنتج غير متوفر. ما أفضل رد؟", ["غير متوفر لدينا", "إليك ما يمكنني فعله: سأتحقق من الفرع الآخر", "عد الأسبوع القادم"], "B", "تقديم الخطوة التالية يُبقي العميل معك."),
    },
    ReelSpec {
        topic: "store-safety",
        title: "Spill on the floor: cone first, mop second",
        script: "See a spill? Put the cone down first so nobody walks into it, then get the mop. The cone takes five seconds; a fall takes weeks.",
        citation: "Store safety manual, section 3.1",
        status: ReelStatus::Published,
        duration_ms: 32_000,
        days_ago: 19,
        q_en: ("You see a spill on the floor. What is your first action?", ["Mop it right away", "Place the cone, then mop", "Call the manager and wait"], "B", "The cone protects people while you fetch the mop."),
        q_ar: ("ترى سائلاً مسكوباً على الأرض. ما أول إجراء؟", ["أمسحه فوراً", "أضع المخروط ثم أمسح", "أتصل بالمدير وأنتظر"], "B", "المخروط يحمي الناس بينما تحضر الممسحة."),
    },
    ReelSpec {
        topic: "store-safety",
        title: "Lift with your legs, box close to your body",
        script: "Bend your knees, keep the box close to your body and lift with your legs. Never twist while carrying; turn with your feet.",
        citation: "Store safety manual, section 5.2",
        status: ReelStatus::Published,
        duration_ms: 36_000,
        days_ago: 16,
        q_en: ("You are lifting a 15 kg box. Which is right?", ["Bend your back, box at arm's length", "Bend your knees, box close to your body", "Twist your torso while lifting"], "B", "Legs are strong; backs are not."),
        q_ar: ("ترفع صندوقاً وزنه ١٥ كغ. ما الصحيح؟", ["أحني ظهري والصندوق بعيد", "أثني ركبتي والصندوق قريب من جسمي", "ألف جذعي أثناء الرفع"], "B", "الساقان قويتان، أما الظهر فلا."),
    },
    ReelSpec {
        topic: "store-safety",
        title: "Fire exit: never blocked, even for a minute",
        script: "Stock arrives and the only free space is by the fire exit. Find another spot. An exit blocked for a minute is blocked at the wrong minute.",
        citation: "Store safety manual, section 2.4",
        status: ReelStatus::Published,
        duration_ms: 31_000,
        days_ago: 12,
        q_en: ("Stock arrives and the only space is by the fire exit. You?", ["Stack it there briefly", "Find another spot; exits stay clear", "Stack it if under 10 minutes"], "B", "There is no safe minute to block an exit."),
        q_ar: ("وصلت البضاعة والمكان الوحيد بجانب مخرج الطوارئ. ماذا تفعل؟", ["أضعها هناك لفترة قصيرة", "أجد مكاناً آخر؛ المخارج تبقى خالية", "أضعها إن كانت أقل من ١٠ دقائق"], "B", "لا توجد دقيقة آمنة لإغلاق مخرج."),
    },
    ReelSpec {
        topic: "aml",
        title: "Which cash transactions must be reported",
        script: "Any cash transaction of SAR 60,000 or more must be reported, no exceptions. It is not about suspicion; it is the threshold.",
        citation: "SAMA AML rules, article 12",
        status: ReelStatus::Published,
        duration_ms: 40_000,
        days_ago: 17,
        q_en: ("Which cash transaction must be reported?", ["Any cash of SAR 60,000 or more", "Only amounts above SAR 100,000", "Only foreign currency"], "A", "60,000 is the threshold, in any currency."),
        q_ar: ("أي معاملة نقدية يجب الإبلاغ عنها؟", ["أي مبلغ نقدي ٦٠٬٠٠٠ ريال أو أكثر", "فقط المبالغ فوق ١٠٠٬٠٠٠ ريال", "فقط العملات الأجنبية"], "A", "٦٠٬٠٠٠ هو الحد، بأي عملة."),
    },
    ReelSpec {
        topic: "aml",
        title: "Split payments are a red flag",
        script: "A customer pays SAR 59,000 in cash twice in one day. That is structuring: splitting to stay under the threshold. Report it.",
        citation: "SAMA AML rules, article 14",
        status: ReelStatus::Published,
        duration_ms: 35_000,
        days_ago: 10,
        q_en: ("A customer pays SAR 59,000 cash twice in one day. You?", ["Normal, two purchases", "Report it: structuring is a red flag", "Ask them to pay by card"], "B", "Two payments just under the limit are the classic pattern."),
        q_ar: ("عميل يدفع ٥٩٬٠٠٠ ريال نقداً مرتين في يوم واحد. ماذا تفعل؟", ["طبيعي، عمليتا شراء", "أبلغ: التجزئة علامة خطر", "أطلب الدفع بالبطاقة"], "B", "دفعتان تحت الحد مباشرة هما النمط الكلاسيكي."),
    },
    ReelSpec {
        topic: "customer-service",
        title: "Returns without a receipt",
        script: "No receipt? Look up the purchase by card or phone number. If nothing is found, offer store credit, not cash.",
        citation: "Returns policy 2026",
        status: ReelStatus::InReview,
        duration_ms: 33_000,
        days_ago: 0,
        q_en: ("A customer wants to return an item without a receipt. First step?", ["Refuse the return", "Look up the purchase by card or phone", "Refund in cash"], "B", "Most purchases can be found; cash refunds need proof."),
        q_ar: ("يريد عميل إرجاع منتج بلا فاتورة. الخطوة الأولى؟", ["أرفض الإرجاع", "أبحث عن الشراء بالبطاقة أو رقم الجوال", "أعيد المبلغ نقداً"], "B", "معظم المشتريات يمكن العثور عليها؛ النقد يحتاج إثباتاً."),
    },
    ReelSpec {
        topic: "aml",
        title: "Know your customer: ask for ID on large purchases",
        script: "For any purchase at or above the threshold, record the customer's ID before completing the sale. No ID, no sale.",
        citation: "SAMA AML rules, article 9",
        status: ReelStatus::InReview,
        duration_ms: 37_000,
        days_ago: 0,
        q_en: ("A purchase reaches the reporting threshold. What do you record?", ["Nothing extra", "The customer's ID before completing the sale", "Only the phone number"], "B", "The ID is what makes the report useful."),
        q_ar: ("وصلت عملية شراء حد الإبلاغ. ماذا تسجل؟", ["لا شيء إضافي", "هوية العميل قبل إتمام البيع", "رقم الجوال فقط"], "B", "الهوية هي ما يجعل البلاغ مفيداً."),
    },
    ReelSpec {
        topic: "store-safety",
        title: "Ladder safety: three points of contact",
        script: "On a ladder keep three points of contact: two feet and one hand, or two hands and one foot. Never lean past the rails.",
        citation: "Store safety manual, section 6.1",
        status: ReelStatus::Draft,
        duration_ms: 30_000,
        days_ago: 0,
        q_en: ("On a ladder, how many points of contact do you keep?", ["Two", "Three", "Four"], "B", "Three points keep you balanced while you reach."),
        q_ar: ("على السلم، كم نقطة تماس تحافظ عليها؟", ["اثنتان", "ثلاث", "أربع"], "B", "ثلاث نقاط تحفظ توازنك أثناء المد."),
    },
];

fn options(texts: &[&str; 3]) -> serde_json::Value {
    serde_json::json!([
        { "key": "A", "text": texts[0] },
        { "key": "B", "text": texts[1] },
        { "key": "C", "text": texts[2] },
    ])
}

/// A tiny but structurally valid MP4 container (ftyp + empty mdat) so players and CDNs accept the key.
fn placeholder_mp4() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0, 0, 0, 0x18]);
    v.extend_from_slice(b"ftypisom");
    v.extend_from_slice(&[0, 0, 0, 1]);
    v.extend_from_slice(b"isomavc1");
    v.extend_from_slice(&[0, 0, 0, 8]);
    v.extend_from_slice(b"mdat");
    v
}

pub fn placeholder_vtt(line: &str) -> String {
    format!("WEBVTT\n\n00:00:00.000 --> 00:00:30.000\n{line}\n")
}

pub struct SeedOutcome {
    pub tenant_id: Uuid,
    pub created: bool,
    pub summary: String,
}

impl std::fmt::Display for SeedOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.summary)
    }
}

pub async fn seed_demo(state: &AppState) -> Result<SeedOutcome, ApiError> {
    let existing = {
        let mut tx = state.db.admin_tx().await?;
        tenants::find_by_slug(&mut tx, TENANT_SLUG).await?
    };
    if let Some(t) = existing {
        return Ok(SeedOutcome {
            tenant_id: t.id,
            created: false,
            summary: format!(
                "tenant '{TENANT_SLUG}' already exists ({}); nothing changed",
                t.id
            ),
        });
    }

    let settings = TenantSettings::default().to_json();
    let languages = vec![
        "ar".to_string(),
        "en".to_string(),
        "ur".to_string(),
        "bn".to_string(),
    ];
    let tenant = {
        let mut tx = state.db.admin_tx().await?;
        let t = tenants::create(
            &mut tx,
            tenants::NewTenant {
                slug: TENANT_SLUG,
                name: "Al Wadi Retail",
                region: Region::Ksa,
                default_language: "ar",
                languages: &languages,
                timezone: "Asia/Riyadh",
                settings,
                credits: 1200,
            },
        )
        .await?;
        tx.commit().await?;
        t
    };
    let today = local_today(&tenant.timezone, Utc::now());

    let mut tx = state.db.tenant_tx(tenant.id).await?;
    let tid = tenant.id;

    let new_user = |role: Role,
                    name: &'static str,
                    phone: &'static str,
                    email: Option<&'static str>,
                    lang: &'static str,
                    channel: Channel,
                    manager: Option<Uuid>| users::NewUser {
        role,
        status: UserStatus::Active,
        full_name: name,
        phone_e164: Some(phone),
        email,
        employee_no: None,
        preferred_language: lang,
        ui_language: if lang == "en" { "en" } else { "ar" },
        manager_id: manager,
        location: Some("Jeddah"),
        job_title: None,
        delivery_channel: channel,
        delivery_slot: DeliverySlot::ShiftStart,
    };
    let noura = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Admin,
            "Noura Al-Harbi",
            "+966500000001",
            Some("admin@alwadi.example"),
            "en",
            Channel::App,
            None,
        ),
    )
    .await?;
    let khalid = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Manager,
            "Khalid Mansour",
            "+966500000002",
            Some("khalid@alwadi.example"),
            "ar",
            Channel::App,
            None,
        ),
    )
    .await?;
    let hana = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Sme,
            "Hana Yusuf",
            "+966500000003",
            Some("hana@alwadi.example"),
            "en",
            Channel::App,
            None,
        ),
    )
    .await?;
    let sara = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Learner,
            "Sara Al-Qahtani",
            "+966500000010",
            None,
            "ar",
            Channel::App,
            Some(khalid.id),
        ),
    )
    .await?;
    let rizwan = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Learner,
            "Rizwan Ahmed",
            "+966500000011",
            None,
            "ur",
            Channel::Whatsapp,
            Some(khalid.id),
        ),
    )
    .await?;
    let mohammad = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Learner,
            "Mohammad A.",
            "+966500000012",
            None,
            "bn",
            Channel::App,
            Some(khalid.id),
        ),
    )
    .await?;
    let fatimah = users::create(
        tx.conn(),
        tid,
        new_user(
            Role::Learner,
            "Fatimah Noor",
            "+966500000013",
            None,
            "en",
            Channel::App,
            Some(khalid.id),
        ),
    )
    .await?;
    for u in [&noura, &khalid, &hana, &sara, &rizwan, &mohammad, &fatimah] {
        sqlx::query("UPDATE users SET employee_no = $2 WHERE id = $1")
            .bind(u.id)
            .bind(format!(
                "AW-{}",
                &u.phone_e164.clone().unwrap_or_default()[9..]
            ))
            .execute(tx.conn())
            .await?;
    }

    let store12 = groups::create(tx.conn(), tid, "Store 12 · Jeddah", "team").await?;
    let warehouse = groups::create(tx.conn(), tid, "Jeddah warehouse", "location").await?;
    let all_staff = groups::create(tx.conn(), tid, "All staff", "custom").await?;
    groups::replace_members(
        tx.conn(),
        tid,
        store12.id,
        &[sara.id, mohammad.id, fatimah.id],
    )
    .await?;
    groups::replace_members(tx.conn(), tid, warehouse.id, &[rizwan.id]).await?;
    groups::replace_members(
        tx.conn(),
        tid,
        all_staff.id,
        &[
            sara.id,
            rizwan.id,
            mohammad.id,
            fatimah.id,
            khalid.id,
            hana.id,
        ],
    )
    .await?;

    let cs = topics::create(
        tx.conn(),
        tid,
        topics::TopicWrite {
            slug: "customer-service",
            name: "Customer service",
            name_ar: Some("خدمة العملاء"),
            description: Some("Greeting, names, turning a no into a next step."),
            compliance_tag: None,
            is_mandatory: false,
        },
    )
    .await?;
    let safety = topics::create(
        tx.conn(),
        tid,
        topics::TopicWrite {
            slug: "store-safety",
            name: "Store safety",
            name_ar: Some("السلامة في المتجر"),
            description: Some("Spills, lifting, exits, ladders."),
            compliance_tag: Some("HRDF-SAF"),
            is_mandatory: true,
        },
    )
    .await?;
    let aml = topics::create(
        tx.conn(),
        tid,
        topics::TopicWrite {
            slug: "aml",
            name: "AML",
            name_ar: Some("مكافحة غسل الأموال"),
            description: Some("Cash thresholds, structuring, KYC."),
            compliance_tag: Some("SAMA-AML"),
            is_mandatory: true,
        },
    )
    .await?;
    let topic_id = |slug: &str| match slug {
        "customer-service" => cs.id,
        "store-safety" => safety.id,
        _ => aml.id,
    };

    let assign =
        |group_id: Uuid, per_day: i16, channel: Channel, mandatory: bool| topics::AssignmentWrite {
            group_id,
            reels_per_day: per_day,
            channel,
            is_mandatory: mandatory,
            starts_on: Some(today - Duration::days(30)),
            ends_on: None,
        };
    topics::upsert_assignment(
        tx.conn(),
        tid,
        cs.id,
        &assign(store12.id, 2, Channel::App, false),
    )
    .await?;
    topics::upsert_assignment(
        tx.conn(),
        tid,
        safety.id,
        &assign(store12.id, 2, Channel::App, true),
    )
    .await?;
    topics::upsert_assignment(
        tx.conn(),
        tid,
        safety.id,
        &assign(warehouse.id, 2, Channel::Whatsapp, true),
    )
    .await?;
    topics::upsert_assignment(
        tx.conn(),
        tid,
        aml.id,
        &assign(all_staff.id, 1, Channel::App, true),
    )
    .await?;

    let mut reel_ids = Vec::with_capacity(REELS.len());
    let mut question_ids = Vec::with_capacity(REELS.len());
    let mut media: Vec<(String, &'static str, Vec<u8>)> = Vec::new();
    for spec in REELS {
        let reel_id = Uuid::now_v7();
        let video_key = reel_media_key(tid, reel_id, 1, "en", "mp4");
        let captions_key = reel_media_key(tid, reel_id, 1, "en", "vtt");
        let reel = reels::create(
            tx.conn(),
            tid,
            reels::NewReel {
                topic_id: topic_id(spec.topic),
                source_id: None,
                generation_job_id: None,
                kind: ReelKind::AiAvatar,
                status: ReelStatus::Draft,
                title: spec.title,
                language: "en",
                script: Some(spec.script),
                source_citation: Some(spec.citation),
                presenter: serde_json::json!({ "name": "Lina", "kind": "ai", "provider_id": "higgsfield:lina-01" }),
                video_key: Some(&video_key),
                captions_key: Some(&captions_key),
                thumbnail_key: None,
                duration_ms: Some(spec.duration_ms),
                compliance_tag: match spec.topic {
                    "store-safety" => Some("HRDF-SAF"),
                    "aml" => Some("SAMA-AML"),
                    _ => None,
                },
                consent_confirmed: true,
                created_by: noura.id,
            },
        )
        .await?;
        media.push((video_key, "video/mp4", placeholder_mp4()));
        media.push((
            captions_key.clone(),
            "text/vtt",
            placeholder_vtt(spec.script).into_bytes(),
        ));
        let q_en = reels::upsert_question(
            tx.conn(),
            tid,
            reel.id,
            "en",
            spec.q_en.0,
            &options(&spec.q_en.1),
            spec.q_en.2,
            Some(spec.q_en.3),
        )
        .await?;
        reels::upsert_question(
            tx.conn(),
            tid,
            reel.id,
            "ar",
            spec.q_ar.0,
            &options(&spec.q_ar.1),
            spec.q_ar.2,
            Some(spec.q_ar.3),
        )
        .await?;
        let ar_video = reel_media_key(tid, reel.id, 1, "ar", "mp4");
        reels::upsert_variant(
            tx.conn(),
            tid,
            reel.id,
            "ar",
            Some(&ar_video),
            Some(&captions_key),
            None,
            Some(spec.script),
            ExtractionStatus::Done,
        )
        .await?;
        media.push((ar_video, "video/mp4", placeholder_mp4()));
        match spec.status {
            ReelStatus::Published => {
                reels::set_status(tx.conn(), reel.id, ReelStatus::InReview).await?;
                reels::add_approval(
                    tx.conn(),
                    tid,
                    reel.id,
                    noura.id,
                    ApprovalRole::Ld,
                    Decision::Approved,
                    Some("Clear and short."),
                    1,
                )
                .await?;
                reels::add_approval(
                    tx.conn(),
                    tid,
                    reel.id,
                    hana.id,
                    ApprovalRole::Sme,
                    Decision::Approved,
                    Some("Accurate."),
                    1,
                )
                .await?;
                reels::set_status(tx.conn(), reel.id, ReelStatus::Approved).await?;
                let audience = match spec.topic {
                    "customer-service" => vec![store12.id],
                    "store-safety" => vec![store12.id, warehouse.id],
                    _ => vec![all_staff.id],
                };
                reels::set_audience(
                    tx.conn(),
                    tid,
                    reel.id,
                    &audience,
                    Some(today - Duration::days(30)),
                )
                .await?;
                reels::publish(tx.conn(), reel.id).await?;
                reels::backdate_published(tx.conn(), reel.id, spec.days_ago).await?;
            }
            ReelStatus::InReview => {
                reels::set_status(tx.conn(), reel.id, ReelStatus::InReview).await?;
                if spec.topic == "aml" {
                    reels::add_approval(
                        tx.conn(),
                        tid,
                        reel.id,
                        noura.id,
                        ApprovalRole::Ld,
                        Decision::Approved,
                        Some("Good. Waiting for SME."),
                        1,
                    )
                    .await?;
                }
            }
            _ => {}
        }
        reel_ids.push(reel.id);
        question_ids.push(q_en.id);
    }

    seed_sara_history(&mut tx, tid, &sara, &reel_ids, &question_ids, today).await?;
    seed_engine(
        &mut tx,
        tid,
        &reel_ids,
        &[sara.id, rizwan.id, mohammad.id, fatimah.id],
    )
    .await?;
    seed_daily_topic_stats(
        &mut tx,
        tid,
        &[
            (cs.id, store12.id),
            (safety.id, store12.id),
            (safety.id, warehouse.id),
            (aml.id, all_staff.id),
        ],
        today,
    )
    .await?;
    reelearn_db::repos::audit::record(
        tx.conn(),
        tid,
        Some(noura.id),
        "seed.demo",
        "tenant",
        Some(tid),
        Some(serde_json::json!({ "reels": reel_ids.len() })),
        None,
    )
    .await?;
    tx.commit().await?;

    let mut uploaded = 0;
    for (key, mime, bytes) in media {
        match state
            .storage
            .put_bytes(&state.http, &key, mime, bytes)
            .await
        {
            Ok(()) => uploaded += 1,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "placeholder media not uploaded (is MinIO running?)");
                break;
            }
        }
    }

    Ok(SeedOutcome {
        tenant_id: tenant.id,
        created: true,
        summary: format!(
            "seeded tenant '{TENANT_SLUG}' ({}) · 7 users (OTP code 1234 in development) · 3 groups · 3 topics · {} reels ({} published) · {uploaded} placeholder media objects",
            tenant.id,
            REELS.len(),
            REELS.iter().filter(|r| r.status == ReelStatus::Published).count()
        ),
    })
}

/// Sara's history: four reels seen, a chain of correct answers on the first question so that
/// its 10-day interval lands on today (the "Asked again · day 10" review), one lapse on safety.
async fn seed_sara_history(
    tx: &mut TenantTx,
    tid: Uuid,
    sara: &users::UserRow,
    reel_ids: &[Uuid],
    question_ids: &[Uuid],
    today: NaiveDate,
) -> Result<(), ApiError> {
    let at = |days_ago: i64| Utc::now() - Duration::days(days_ago) - Duration::hours(3);
    let day = |days_ago: i64| today - Duration::days(days_ago);
    // (reel index, [(days_ago, correct)...])
    let history: [(usize, &[(i64, bool)]); 4] = [
        (0, &[(14, true), (13, true), (10, true)]),
        (
            3,
            &[(13, true), (12, false), (9, true), (8, true), (5, true)],
        ),
        (1, &[(3, true), (2, true)]),
        (4, &[(2, false)]),
    ];
    let mut answers_total = 0;
    let mut answers_correct = 0;
    for (idx, events) in history {
        let reel_id = reel_ids[idx];
        let question_id = question_ids[idx];
        let first = events[0].0;
        learning::insert_watch_event_at(tx.conn(), tid, sara.id, reel_id, 30_000, at(first))
            .await?;
        let mut state = MemoryState::fresh(day(first));
        for (i, (days_ago, correct)) in events.iter().enumerate() {
            state = recall::review(&state, *correct, day(*days_ago)).state;
            learning::insert_answer(
                tx.conn(),
                tid,
                sara.id,
                question_id,
                reel_id,
                None,
                if *correct { "B" } else { "A" },
                *correct,
                Some(4200),
                i16::try_from(i + 1).unwrap_or(1),
                Channel::App,
                at(*days_ago),
            )
            .await?;
            answers_total += 1;
            if *correct {
                answers_correct += 1;
            }
        }
        learning::upsert_memory_state(
            tx.conn(),
            tid,
            sara.id,
            question_id,
            &state,
            Some(at(events[events.len() - 1].0)),
        )
        .await?;
    }
    // Fix the first question's correct keys: reel 0..=5 use "B"; reel 6 (AML threshold) uses "A" but Sara has not seen it.
    stats::user_stats_for_update(tx.conn(), tid, sara.id).await?;
    stats::write_user_stats(
        tx.conn(),
        sara.id,
        stats::UserStatsWrite {
            streak_current: 4,
            streak_longest: 6,
            last_active_on: Some(day(2)),
            freezes_available: 1,
            reels_completed_delta: 4,
            answers_total_delta: answers_total,
            answers_correct_delta: answers_correct,
            remembered_count: Some(2),
            minutes_on_mandatory_delta: 1.1,
        },
    )
    .await?;
    let readiness = learning::memory_by_topic(tx.conn(), sara.id, today).await?;
    for r in readiness {
        let topic_reels = reels::count_in_topic_published(tx.conn(), r.topic_id).await?;
        let seen = learning::reels_seen_in_topic(tx.conn(), sara.id, r.topic_id).await?;
        let pct = if r.total > 0 {
            i16::try_from(100 * r.remembered / r.total).unwrap_or(0)
        } else {
            0
        };
        stats::upsert_readiness(
            tx.conn(),
            tid,
            sara.id,
            r.topic_id,
            seen >= topic_reels && pct >= 70,
            pct,
            i32::try_from(seen).unwrap_or(0),
            i32::try_from(topic_reels).unwrap_or(0),
        )
        .await?;
    }
    Ok(())
}

async fn seed_daily_topic_stats(
    tx: &mut TenantTx,
    tid: Uuid,
    pairs: &[(Uuid, Uuid)],
    today: NaiveDate,
) -> Result<(), ApiError> {
    for d in 1..=14i64 {
        let day = today - Duration::days(d);
        let weekday = chrono::Datelike::weekday(&day);
        if matches!(weekday, chrono::Weekday::Fri | chrono::Weekday::Sat) {
            continue;
        }
        for (i, (topic_id, group_id)) in pairs.iter().enumerate() {
            let base = 2 + ((d as usize + i) % 3) as i32;
            stats::bump_daily_topic_stats(
                tx.conn(),
                tid,
                *topic_id,
                *group_id,
                day,
                stats::DailyTopicDelta {
                    learners_active: base,
                    reels_completed: base * 2,
                    answers_total: base * 2,
                    answers_correct: base * 2 - (d % 2) as i32,
                    minutes_total: f64::from(base) * 1.1,
                },
            )
            .await?;
        }
    }
    Ok(())
}

/// Tags and difficulty per demo reel (what the studio pipeline writes for generated reels).
fn seed_features(title: &str) -> (Vec<String>, i16) {
    let (tags, d): (&[&str], i16) = match title {
        t if t.starts_with("Say the customer") => (&["customer service", "names", "rapport"], 1),
        t if t.starts_with("Greet within") => (&["customer service", "greeting", "speed"], 1),
        t if t.starts_with("Turn 'no'") => (&["customer service", "complaints", "language"], 3),
        t if t.starts_with("Spill on the floor") => (&["store safety", "spills", "hazards"], 2),
        t if t.starts_with("Lift with your legs") => (&["store safety", "lifting", "warehouse"], 2),
        t if t.starts_with("Fire exit") => (&["store safety", "fire", "exits"], 1),
        t if t.starts_with("Which cash transactions") => (&["aml", "cash", "reporting"], 4),
        t if t.starts_with("Split payments") => (&["aml", "red flags", "payments"], 4),
        t if t.starts_with("Returns without") => (&["aml", "returns", "fraud"], 3),
        t if t.starts_with("Know your customer") => (&["aml", "kyc", "identity"], 3),
        t if t.starts_with("Ladder safety") => (&["store safety", "ladders", "warehouse"], 2),
        _ => (&["general"], 3),
    };
    (tags.iter().map(|t| t.to_string()).collect(), d)
}

/// A week of learning-engine history so practice, "Your topics" and the insights page start
/// with something to show: reel statistics, interest, sessions, events and one colleague share.
async fn seed_engine(
    tx: &mut reelearn_db::TenantTx,
    tid: Uuid,
    reel_ids: &[Uuid],
    learners: &[Uuid],
) -> Result<(), ApiError> {
    use reelearn_db::repos::engine as eng;
    let mut published = Vec::new();
    for (i, id) in reel_ids.iter().enumerate() {
        let reel = reels::get(tx.conn(), *id).await?;
        let (tags, difficulty) = seed_features(&reel.title);
        reels::set_features(tx.conn(), *id, &tags, Some(difficulty)).await?;
        if reel.status != ReelStatus::Published {
            continue;
        }
        published.push(reel.clone());
        // Deterministic but varied: some reels are finished more, some skipped more.
        let n = i as i32;
        let starts = 40 + 7 * n;
        sqlx::query(
            "INSERT INTO reel_engagement_stats (reel_id, tenant_id, impressions, starts, skips, completions, rewatches, saves, shares, not_relevant, answers, answers_correct, watched_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (reel_id) DO NOTHING",
        )
        .bind(*id)
        .bind(tid)
        .bind(starts + 12)
        .bind(starts)
        .bind(3 + (n * 5) % 11)
        .bind(starts - 6 - (n * 3) % 9)
        .bind(n % 4)
        .bind(2 + n % 5)
        .bind(n % 3)
        .bind(n % 2)
        .bind(starts - 10)
        .bind(starts - 14 - (n * 2) % 7)
        .bind(i64::from(starts) * 26_000)
        .execute(tx.conn())
        .await
        .map_err(reelearn_db::DbError::from)?;
    }

    // Interest: each learner leans toward one topic; Sara follows customer service.
    for (k, user) in learners.iter().enumerate() {
        for (j, reel) in published.iter().enumerate() {
            let delta = if (j + k) % 3 == 0 { 2.5 } else { 0.6 };
            eng::add_topic_interest(tx.conn(), tid, *user, reel.topic_id, delta, 14.0).await?;
            let (tags, _) = seed_features(&reel.title);
            eng::add_tag_interest(tx.conn(), tid, *user, &tags, delta * 0.5, 14.0).await?;
        }
    }
    if let (Some(sara), Some(first)) = (learners.first(), published.first()) {
        eng::set_preference(
            tx.conn(),
            tid,
            *sara,
            first.topic_id,
            Some(reelearn_domain::engine::Preference::Follow),
        )
        .await?;
    }

    // A week of practice sessions and their start/complete events.
    for day in 1..=7_i32 {
        for (k, user) in learners.iter().enumerate() {
            if (day as usize + k).is_multiple_of(3) {
                continue;
            }
            let session: Uuid = sqlx::query_scalar(
                "INSERT INTO engine_sessions (tenant_id, user_id, started_at, last_active_at, ended_at, items_served, items_completed, active_ms, stop_reason)
                 VALUES ($1, $2, now() - make_interval(days => $3), now() - make_interval(days => $3) + interval '6 minutes',
                         now() - make_interval(days => $3) + interval '6 minutes', $4, $5, $6, 'closed') RETURNING id",
            )
            .bind(tid)
            .bind(*user)
            .bind(day)
            .bind(6 + (k as i32))
            .bind(4 + (k as i32) % 3)
            .bind(i64::from(180_000 + 45_000 * ((day + k as i32) % 5)))
            .fetch_one(tx.conn())
            .await
            .map_err(reelearn_db::DbError::from)?;
            for (j, reel) in published.iter().enumerate().take(4) {
                for kind in ["impression", "start", "complete"] {
                    if kind == "complete" && (j + k + day as usize).is_multiple_of(4) {
                        continue;
                    }
                    sqlx::query(
                        "INSERT INTO engagement_events (tenant_id, user_id, reel_id, session_id, kind, watched_ms, reason, occurred_at)
                         VALUES ($1, $2, $3, $4, $5::engagement_kind_t, $6, $7, now() - make_interval(days => $8))",
                    )
                    .bind(tid)
                    .bind(*user)
                    .bind(reel.id)
                    .bind(session)
                    .bind(kind)
                    .bind(if kind == "complete" { Some(28_000) } else { None })
                    .bind(if kind == "impression" { Some(["interest", "assigned", "popular", "explore"][j % 4]) } else { None })
                    .bind(day)
                    .execute(tx.conn())
                    .await
                    .map_err(reelearn_db::DbError::from)?;
                }
            }
        }
    }

    // Fatimah sent Sara a reel she has not opened yet.
    if let (Some(sara), Some(fatimah), Some(reel)) =
        (learners.first(), learners.get(3), published.get(2))
    {
        eng::create_share(
            tx.conn(),
            tid,
            reel.id,
            *fatimah,
            *sara,
            Some("Worth a minute before your shift"),
        )
        .await?;
    }
    eng::refresh_band_stats(tx.conn(), tid).await?;
    eng::refresh_cohort_stats(tx.conn(), tid).await?;
    Ok(())
}
