//! WhatsApp-only mode (E10): the same plan, the same gating. START sends the first
//! item, a letter answers the current question, NEXT moves on only once answered.
//! Outbound messages are queued as `send_delivery` jobs; nothing here calls a provider.

use crate::dto::{AnswerRequest, DailyPlan, DailyPlanItem, WatchRequest};
use crate::error::ApiError;
use crate::services::learn::{self, LearnerCtx};
use crate::state::AppState;
use reelearn_db::repos::{deliveries, jobs, reels};
use reelearn_db::TenantTx;
use reelearn_domain::{Channel, PlanItemKind, PlanItemState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Outbound {
    pub body: String,
    /// Object key of a reel video to attach.
    pub video_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Start,
    Next,
    Answer(String),
    Help,
}

pub fn parse_command(text: &str) -> Command {
    let t = text.trim().to_uppercase();
    match t.as_str() {
        "START" | "ابدأ" | "ابدا" | "HI" | "HELLO" | "مرحبا" | "TODAY" | "اليوم" => {
            Command::Start
        }
        "NEXT" | "التالي" | "N" | "MORE" => Command::Next,
        "A" | "B" | "C" | "D" | "E" => Command::Answer(t),
        "1" | "2" | "3" | "4" | "5" => {
            let idx = t.as_bytes()[0] - b'1';
            Command::Answer(((b'A' + idx) as char).to_string())
        }
        "أ" => Command::Answer("A".into()),
        "ب" => Command::Answer("B".into()),
        "ج" => Command::Answer("C".into()),
        "د" => Command::Answer("D".into()),
        _ => Command::Help,
    }
}

fn t(lang: &str, en: &str, ar: &str) -> String {
    if lang == "ar" {
        ar.to_string()
    } else {
        en.to_string()
    }
}

/// Handle one inbound text and return the replies to queue.
pub async fn handle(
    state: &AppState,
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    text: &str,
) -> Result<Vec<Outbound>, ApiError> {
    let lang = ctx.user.preferred_language.as_str();
    let command = parse_command(text);
    let plan = learn::build_today(state, tx, ctx).await?;
    let pending = plan
        .items
        .iter()
        .find(|i| i.state != PlanItemState::Answered);

    match command {
        Command::Help => Ok(vec![Outbound {
            body: t(
                lang,
                "Reply START for today's reels, NEXT for the next one, or A/B/C to answer.",
                "أرسل ابدأ لريلز اليوم، التالي للريل التالي، أو A/B/C للإجابة.",
            ),
            video_key: None,
        }]),
        Command::Start | Command::Next => match pending {
            None => Ok(vec![Outbound {
                body: done_line(lang, &plan),
                video_key: None,
            }]),
            Some(item) => {
                // Gating: a watched reel or a review question must be answered before moving on.
                let awaiting_answer = item.question.is_some()
                    && (item.kind == PlanItemKind::QuestionOnly
                        || item.state == PlanItemState::Watched);
                if awaiting_answer && command == Command::Next {
                    return Ok(vec![Outbound {
                        body: format!(
                            "{}\n\n{}",
                            t(lang, "Answer first, then NEXT.", "أجب أولاً ثم أرسل التالي."),
                            question_text(item)
                        ),
                        video_key: None,
                    }]);
                }
                if awaiting_answer {
                    return Ok(vec![Outbound {
                        body: question_text(item),
                        video_key: None,
                    }]);
                }
                let mut out = Vec::new();
                if let (PlanItemKind::Reel, Some(reel)) = (item.kind, &item.reel) {
                    // Sending the reel counts as the play; the question follows in the same turn.
                    learn::record_watch(
                        state,
                        tx,
                        ctx,
                        &WatchRequest {
                            plan_item_id: Some(item.id),
                            reel_id: reel.id,
                            watched_ms: reel.duration_ms,
                            completed: true,
                            channel: Some(Channel::Whatsapp),
                        },
                        None,
                    )
                    .await?;
                    let row = reels::get(tx.conn(), reel.id).await?;
                    let caption = format!(
                        "{} · {}\n{}",
                        reel.topic_name,
                        reel.title,
                        reel.caption_text.clone().unwrap_or_default()
                    );
                    out.push(Outbound {
                        body: caption,
                        video_key: row.video_key.clone(),
                    });
                }
                if item.question.is_some() {
                    out.push(Outbound {
                        body: question_text(item),
                        video_key: None,
                    });
                } else {
                    out.push(Outbound {
                        body: t(
                            lang,
                            "Reply NEXT for the next reel.",
                            "أرسل التالي للريل التالي.",
                        ),
                        video_key: None,
                    });
                }
                Ok(out)
            }
        },
        Command::Answer(key) => match pending {
            Some(item) if item.question.is_some() => {
                let Some(question) = item.question.as_ref() else {
                    return Ok(vec![]);
                };
                if !question.options.iter().any(|o| o.key == key) {
                    return Ok(vec![Outbound {
                        body: question_text(item),
                        video_key: None,
                    }]);
                }
                let outcome = learn::record_answer(
                    state,
                    tx,
                    ctx,
                    &AnswerRequest {
                        plan_item_id: Some(item.id),
                        question_id: question.id,
                        chosen_key: key,
                        response_ms: None,
                        channel: Some(Channel::Whatsapp),
                    },
                )
                .await?;
                let answer_text = question
                    .options
                    .iter()
                    .find(|o| o.key == outcome.correct_key)
                    .map(|o| o.text.clone())
                    .unwrap_or_default();
                // No red: the right answer in mint and when it comes back.
                let mut body = if outcome.is_correct {
                    t(
                        lang,
                        &format!("Remembered. {} · {}", outcome.correct_key, answer_text),
                        &format!("أحسنت. {} · {}", outcome.correct_key, answer_text),
                    )
                } else {
                    t(
                        lang,
                        &format!("The answer is {} · {}", outcome.correct_key, answer_text),
                        &format!("الإجابة هي {} · {}", outcome.correct_key, answer_text),
                    )
                };
                if let Some(e) = &outcome.explanation {
                    body.push('\n');
                    body.push_str(e);
                }
                body.push_str(&t(
                    lang,
                    &format!("\nComes back on {}.", outcome.next_due_on),
                    &format!("\nيعود يوم {}.", outcome.next_due_on),
                ));
                if outcome.plan_status == "done" {
                    let plan = learn::build_today(state, tx, ctx).await?;
                    body.push_str("\n\n");
                    body.push_str(&done_line(lang, &plan));
                } else {
                    body.push_str(&t(
                        lang,
                        "\n\nReply NEXT for the next reel.",
                        "\n\nأرسل التالي للريل التالي.",
                    ));
                }
                Ok(vec![Outbound {
                    body,
                    video_key: None,
                }])
            }
            _ => Ok(vec![Outbound {
                body: t(
                    lang,
                    "Nothing to answer right now. Reply START.",
                    "لا يوجد سؤال الآن. أرسل ابدأ.",
                ),
                video_key: None,
            }]),
        },
    }
}

fn question_text(item: &DailyPlanItem) -> String {
    let Some(q) = &item.question else {
        return String::new();
    };
    let mut s = String::new();
    if let Some(note) = &item.review_note {
        s.push_str(note);
        s.push('\n');
    }
    s.push_str(&q.prompt);
    for o in &q.options {
        s.push_str(&format!("\n{}) {}", o.key, o.text));
    }
    s
}

fn done_line(lang: &str, plan: &DailyPlan) -> String {
    let streak = plan.stats.streak_current;
    t(
        lang,
        &format!("Done for today ✓ Streak: {streak} days. See you tomorrow."),
        &format!("انتهى يومك ✓ سلسلتك: {streak} يوم. نراك غداً."),
    )
}

/// Queue outbound replies as deliveries + jobs and log them.
pub async fn queue_replies(
    tx: &mut TenantTx,
    ctx: &LearnerCtx,
    phone: &str,
    replies: &[Outbound],
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    for r in replies {
        let d = deliveries::schedule(
            tx.conn(),
            tenant_id,
            ctx.user.id,
            None,
            Channel::Whatsapp,
            "bot_reply",
            chrono::Utc::now(),
        )
        .await?;
        jobs::enqueue(tx.conn(), Some(tenant_id), "send_delivery", serde_json::json!({ "delivery_id": d.id, "scheduled_for": d.scheduled_for, "kind": "whatsapp_message", "to": phone, "body": r.body, "video_key": r.video_key }), None).await?;
        deliveries::log_whatsapp(
            tx.conn(),
            tenant_id,
            Some(ctx.user.id),
            phone,
            "out",
            Some(&r.body),
            None,
            None,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_parse_in_both_languages() {
        assert_eq!(parse_command(" start "), Command::Start);
        assert_eq!(parse_command("ابدأ"), Command::Start);
        assert_eq!(parse_command("next"), Command::Next);
        assert_eq!(parse_command("b"), Command::Answer("B".into()));
        assert_eq!(parse_command("2"), Command::Answer("B".into()));
        assert_eq!(parse_command("ج"), Command::Answer("C".into()));
        assert_eq!(parse_command("what?"), Command::Help);
    }
}
