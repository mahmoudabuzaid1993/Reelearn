//! The AI Reel Studio pipeline.
//!
//! A generation job is a row in `generation_jobs` with ordered `generation_steps`
//! (extract → script → question → render → voice → caption → translate). Each step
//! runs as one durable queue job carrying `{generation_job_id, position}`; finishing a
//! step enqueues the next. Media providers may answer `Pending`: the step then waits
//! with a `provider_ref` until a `provider_callback` job completes it.
//!
//! Source-only jobs (`extract_source`, `transcribe_podcast`, `find_moments`) have no
//! step row; `cut_moment` creates a one-step job so async cuts work the same way.

use crate::jobs::{JobError, JobOutcome};
use crate::providers::{
    real, vtt_from_script, CutRequest, ExtractInput, MediaResult, ProviderError, QuestionDraft,
    RenderRequest, ScriptInput, TranscriptResult, TranscriptSegment,
};
use crate::Ctx;
use reelearn_api::s3::reel_media_key;
use reelearn_db::repos::{jobs, media, reels, studio, topics};
use reelearn_db::TenantTx;
use reelearn_domain::{
    ExtractionStatus, GenStep, GenerationStyle, JobStatus, ReelKind, ReelStatus,
};
use uuid::Uuid;

fn perr(e: ProviderError) -> JobError {
    if e.is_transient() {
        JobError::Transient(e.to_string())
    } else {
        JobError::Permanent(e.to_string())
    }
}

fn payload_uuid(payload: &serde_json::Value, key: &str) -> Result<Uuid, JobError> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| JobError::Permanent(format!("payload is missing {key}")))
}

/// What a step left behind while a vendor renders asynchronously.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pending {
    kind: String,
    reel_id: Uuid,
    language: String,
    remaining: Vec<Uuid>,
}

// ------------------------------------------------------------------ source-only jobs

pub async fn extract_source(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let source_id = payload_uuid(payload, "source_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let result = run_extract(ctx, &mut tx, source_id).await;
    match result {
        Ok(words) => {
            tx.commit().await?;
            Ok(JobOutcome::done(format!("extracted {words} words")))
        }
        Err(JobError::Permanent(msg)) => {
            studio::set_extraction_status(
                tx.conn(),
                source_id,
                ExtractionStatus::Failed,
                Some(&msg),
            )
            .await?;
            tx.commit().await?;
            Err(JobError::Permanent(msg))
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn run_extract(ctx: &Ctx, tx: &mut TenantTx, source_id: Uuid) -> Result<usize, JobError> {
    let tid = tx.tenant_id();
    let source = studio::get_source(tx.conn(), source_id).await?;
    if let Some(existing) = studio::extraction(tx.conn(), source_id).await? {
        if source.extraction_status == ExtractionStatus::Done && !existing.text.is_empty() {
            return Ok(existing.text.split_whitespace().count());
        }
    }
    studio::set_extraction_status(tx.conn(), source_id, ExtractionStatus::Running, None).await?;
    let (media_url, mime) = match source.media_asset_id {
        Some(id) => {
            let asset = media::get(tx.conn(), id).await?;
            (
                Some(ctx.state.storage.presign_get(&asset.storage_key)),
                Some(asset.mime),
            )
        }
        None => (None, None),
    };
    let input = ExtractInput {
        kind: source.kind.as_str().to_string(),
        title: source.title.clone(),
        url: source.url.clone(),
        prompt: source.prompt.clone(),
        media_url,
        mime,
        language_hint: source.language.clone(),
    };
    let extraction = ctx.providers.writer.extract(input).await.map_err(perr)?;
    let summary = serde_json::json!({ "key_ideas": extraction.key_ideas, "suggested_reels": extraction.suggested_reels, "word_count": extraction.text.split_whitespace().count() });
    studio::upsert_extraction(
        tx.conn(),
        tid,
        source_id,
        &extraction.text,
        &summary,
        extraction.language.as_deref(),
    )
    .await?;
    studio::set_extraction_status(tx.conn(), source_id, ExtractionStatus::Done, None).await?;
    Ok(extraction.text.split_whitespace().count())
}

pub async fn transcribe_podcast(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let source_id = payload_uuid(payload, "source_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let source = studio::get_source(tx.conn(), source_id).await?;
    if source.extraction_status == ExtractionStatus::Done {
        tx.commit().await?;
        return Ok(JobOutcome::done("already transcribed".into()));
    }
    let Some(asset_id) = source.media_asset_id else {
        studio::set_extraction_status(
            tx.conn(),
            source_id,
            ExtractionStatus::Failed,
            Some("no uploaded file"),
        )
        .await?;
        tx.commit().await?;
        return Err(JobError::Permanent("podcast source has no media".into()));
    };
    let asset = media::get(tx.conn(), asset_id).await?;
    studio::set_extraction_status(tx.conn(), source_id, ExtractionStatus::Running, None).await?;
    let url = ctx.state.storage.presign_get(&asset.storage_key);
    match ctx
        .providers
        .asr
        .transcribe(&url, source.language.as_deref())
        .await
    {
        Ok(TranscriptResult::Ready { segments, language }) => {
            let n =
                finish_transcript(ctx, &mut tx, source_id, &segments, language.as_deref()).await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!(
                "{} segments, {n} moments",
                segments.len()
            )))
        }
        Ok(TranscriptResult::Pending { provider_ref }) => {
            studio::upsert_extraction(tx.conn(), tenant_id, source_id, "", &serde_json::json!({ "pending_ref": provider_ref, "provider": ctx.providers.asr.name() }), source.language.as_deref()).await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!(
                "transcription pending at {} ({provider_ref})",
                ctx.providers.asr.name()
            )))
        }
        Err(e) if e.is_transient() => {
            tx.rollback().await?;
            Err(JobError::Transient(e.to_string()))
        }
        Err(e) => {
            studio::set_extraction_status(
                tx.conn(),
                source_id,
                ExtractionStatus::Failed,
                Some(&e.to_string()),
            )
            .await?;
            tx.commit().await?;
            Err(JobError::Permanent(e.to_string()))
        }
    }
}

async fn finish_transcript(
    ctx: &Ctx,
    tx: &mut TenantTx,
    source_id: Uuid,
    segments: &[TranscriptSegment],
    language: Option<&str>,
) -> Result<usize, JobError> {
    let tid = tx.tenant_id();
    let text = segments
        .iter()
        .map(|s| {
            format!(
                "[{:02}:{:02}] {}{}",
                s.start_ms / 60_000,
                (s.start_ms / 1000) % 60,
                s.speaker
                    .as_deref()
                    .map(|sp| format!("{sp}: "))
                    .unwrap_or_default(),
                s.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let seg_json: Vec<serde_json::Value> = segments.iter().map(|s| serde_json::json!({ "start_ms": s.start_ms, "end_ms": s.end_ms, "text": s.text, "speaker": s.speaker })).collect();
    let duration_ms = segments.last().map(|s| s.end_ms).unwrap_or(0);
    studio::upsert_extraction(
        tx.conn(),
        tid,
        source_id,
        &text,
        &serde_json::json!({ "segments": seg_json, "duration_ms": duration_ms }),
        language,
    )
    .await?;
    let moments = ctx
        .providers
        .writer
        .find_moments(segments)
        .await
        .map_err(perr)?;
    studio::delete_moments(tx.conn(), source_id).await?;
    for m in &moments {
        let end = m.end_ms.min(m.start_ms + 90_000);
        if end > m.start_ms {
            studio::insert_moment(
                tx.conn(),
                tid,
                source_id,
                m.start_ms,
                end,
                &m.excerpt,
                Some(&m.title),
                m.score,
                m.speaker.as_deref(),
            )
            .await?;
        }
    }
    studio::set_extraction_status(tx.conn(), source_id, ExtractionStatus::Done, None).await?;
    Ok(moments.len())
}

pub async fn find_moments(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let source_id = payload_uuid(payload, "source_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let extraction = studio::extraction(tx.conn(), source_id)
        .await?
        .ok_or_else(|| JobError::Permanent("source is not transcribed yet".into()))?;
    let segments = real::segments_from_callback(&extraction.summary["segments"]);
    if segments.is_empty() {
        return Err(JobError::Permanent("transcript has no segments".into()));
    }
    let n = finish_transcript(
        ctx,
        &mut tx,
        source_id,
        &segments,
        extraction.language.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok(JobOutcome::done(format!("{n} moments")))
}

/// `cut_moment` from the API carries `{reel_id, source_id, moment_id, start_ms, end_ms}`.
/// It becomes a one-step generation job so a pending vendor cut completes through the
/// same callback path as a render.
pub async fn cut_moment(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let reel_id = payload_uuid(payload, "reel_id")?;
    let source_id = payload_uuid(payload, "source_id")?;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let reel = reels::get(tx.conn(), reel_id).await?;
    if reel.video_key.is_some() {
        tx.commit().await?;
        return Ok(JobOutcome::done("reel already has video".into()));
    }
    let job_id = match studio::find_job_by_param(tx.conn(), "reel_id", &reel_id.to_string()).await?
    {
        Some(j) => j.id,
        None => {
            let params = serde_json::json!({ "kind": "cut", "reel_id": reel_id, "start_ms": payload["start_ms"], "end_ms": payload["end_ms"], "languages": [reel.language] });
            let job = studio::create_job(
                tx.conn(),
                tenant_id,
                source_id,
                reel.created_by,
                None,
                &params,
                0,
            )
            .await?;
            studio::insert_steps(tx.conn(), tenant_id, job.id, &[GenStep::Cut]).await?;
            job.id
        }
    };
    tx.commit().await?;
    run_step(ctx, tenant_id, &serde_json::json!({ "generation_job_id": job_id, "position": 0, "source_id": source_id, "reel_id": reel_id })).await
}

// ------------------------------------------------------------------ generation steps

pub async fn run_step(
    ctx: &Ctx,
    tenant_id: Uuid,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let job_id = payload_uuid(payload, "generation_job_id")?;
    let position = payload
        .get("position")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i16;
    let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
    let job = studio::get_job(tx.conn(), job_id).await?;
    if matches!(job.status, JobStatus::Cancelled) {
        tx.commit().await?;
        return Ok(JobOutcome::done("job cancelled".into()));
    }
    let step = studio::step_at(tx.conn(), job_id, position)
        .await?
        .ok_or_else(|| JobError::Permanent(format!("job {job_id} has no step {position}")))?;
    if step.status == JobStatus::Done {
        advance(&mut tx, &job, position).await?;
        tx.commit().await?;
        return Ok(JobOutcome::done("step already done; advanced".into()));
    }
    studio::update_step(
        tx.conn(),
        step.id,
        JobStatus::Running,
        None,
        None,
        None,
        true,
    )
    .await?;
    studio::set_job_status(tx.conn(), job_id, JobStatus::Running, None, None).await?;

    let outcome = execute(ctx, &mut tx, &job, step.step, position, payload).await;
    match outcome {
        Ok(StepResult::Done(note)) => {
            studio::update_step(
                tx.conn(),
                step.id,
                JobStatus::Done,
                Some(provider_for(ctx, step.step)),
                None,
                Some(&note),
                false,
            )
            .await?;
            advance(&mut tx, &job, position).await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!("{} · {note}", step.step.as_str())))
        }
        Ok(StepResult::Pending(pending, provider_ref)) => {
            let log = serde_json::to_string(&pending).unwrap_or_default();
            studio::update_step(
                tx.conn(),
                step.id,
                JobStatus::Running,
                Some(provider_for(ctx, step.step)),
                Some(&provider_ref),
                Some(&log),
                false,
            )
            .await?;
            tx.commit().await?;
            Ok(JobOutcome::done(format!(
                "{} · waiting for provider ({provider_ref})",
                step.step.as_str()
            )))
        }
        Err(JobError::Transient(msg)) => {
            // Keep the step running with the error in its log; the queue retries with backoff.
            studio::update_step(
                tx.conn(),
                step.id,
                JobStatus::Running,
                None,
                None,
                Some(&format!("retrying: {msg}")),
                false,
            )
            .await?;
            tx.commit().await?;
            Err(JobError::Transient(msg))
        }
        Err(JobError::Permanent(msg)) => {
            studio::update_step(
                tx.conn(),
                step.id,
                JobStatus::Failed,
                None,
                None,
                Some(&msg),
                false,
            )
            .await?;
            studio::set_job_status(tx.conn(), job_id, JobStatus::Failed, None, Some(&msg)).await?;
            tx.commit().await?;
            Err(JobError::Permanent(msg))
        }
    }
}

enum StepResult {
    Done(String),
    Pending(Pending, String),
}

fn provider_for(ctx: &Ctx, step: GenStep) -> &'static str {
    match step {
        GenStep::Extract
        | GenStep::Script
        | GenStep::Question
        | GenStep::Translate
        | GenStep::FindMoments => ctx.providers.writer.name(),
        GenStep::Render | GenStep::Cut => ctx.providers.video.name(),
        GenStep::Voice => ctx.providers.voice.name(),
        GenStep::Caption => "worker",
        GenStep::Transcribe => ctx.providers.asr.name(),
    }
}

/// Mark progress and enqueue the next step, or finish the job.
async fn advance(
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
    position: i16,
) -> Result<(), JobError> {
    let steps = studio::steps(tx.conn(), job.id).await?;
    let total = steps.len().max(1) as i32;
    let done = steps.iter().filter(|s| s.status == JobStatus::Done).count() as i32;
    let pct = ((done * 100) / total).clamp(0, 100) as i16;
    match steps.iter().find(|s| s.position == position + 1) {
        Some(next) => {
            studio::set_job_status(tx.conn(), job.id, JobStatus::Running, Some(pct), None).await?;
            if !jobs::active_for_generation_position(tx.conn(), job.id, next.position).await? {
                let payload = serde_json::json!({ "generation_job_id": job.id, "position": next.position, "source_id": job.source_id, "reel_id": job.param_str("reel_id") });
                jobs::enqueue(
                    tx.conn(),
                    Some(job.tenant_id),
                    next.step.job_kind(),
                    payload,
                    None,
                )
                .await?;
            }
        }
        None => {
            studio::set_job_status(tx.conn(), job.id, JobStatus::Done, Some(100), None).await?;
        }
    }
    Ok(())
}

async fn execute(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
    step: GenStep,
    _position: i16,
    payload: &serde_json::Value,
) -> Result<StepResult, JobError> {
    match step {
        GenStep::Extract => {
            let words = run_extract(ctx, tx, job.source_id).await?;
            Ok(StepResult::Done(format!("{words} words")))
        }
        GenStep::Script => script_step(ctx, tx, job).await,
        GenStep::Question => question_step(ctx, tx, job).await,
        GenStep::Render => media_step(ctx, tx, job, "render", None).await,
        GenStep::Voice => media_step(ctx, tx, job, "voice", None).await,
        GenStep::Caption => caption_step(tx, job).await,
        GenStep::Translate => translate_step(ctx, tx, job).await,
        GenStep::Cut => {
            let reel_id = payload_uuid(payload, "reel_id").or_else(|_| {
                job.param_uuid("reel_id")
                    .ok_or_else(|| JobError::Permanent("cut without reel_id".into()))
            })?;
            media_step(ctx, tx, job, "cut", Some(reel_id)).await
        }
        GenStep::Transcribe | GenStep::FindMoments => Err(JobError::Permanent(
            "transcription runs as a source job, not a generation step".into(),
        )),
    }
}

fn job_style(job: &studio::GenerationJobRow) -> GenerationStyle {
    serde_json::from_value(job.params["style"].clone()).unwrap_or(GenerationStyle::AiAvatar)
}

fn master_language(job: &studio::GenerationJobRow) -> String {
    job.param_languages()
        .into_iter()
        .next()
        .unwrap_or_else(|| "ar".into())
}

/// The reels this job works on: the one it regenerates, or every reel it created.
async fn job_reels(
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
) -> Result<Vec<reels::ReelRow>, JobError> {
    if let Some(reel_id) = job.param_uuid("reel_id") {
        return Ok(vec![reels::get(tx.conn(), reel_id).await?]);
    }
    Ok(reels::for_job(tx.conn(), job.id).await?)
}

async fn script_step(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
) -> Result<StepResult, JobError> {
    let tid = tx.tenant_id();
    let source = studio::get_source(tx.conn(), job.source_id).await?;
    let extraction = studio::extraction(tx.conn(), job.source_id)
        .await?
        .ok_or_else(|| JobError::Permanent("source has no extraction".into()))?;
    let topic_id = job
        .param_uuid("topic_id")
        .or_else(|| job.param_uuid("reel_id").map(|_| Uuid::nil()));
    let language = master_language(job);
    let presenter = job
        .params
        .get("presenter")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "name": "Reelearn presenter", "kind": "ai" }));
    // The studio sends a voice id ("layla-ar-sa"); the presenter is named after it unless a name is given.
    let presenter_name = presenter["name"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| {
            presenter["voice_id"]
                .as_str()
                .and_then(|v| v.split(['-', '_']).next())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let mut c = s.chars();
                    c.next()
                        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                        .unwrap_or_default()
                })
        })
        .unwrap_or_else(|| "Maha".to_string());
    let key_ideas: Vec<String> = extraction.summary["key_ideas"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Regenerate: rewrite the script of one existing reel.
    if let Some(reel_id) = job.param_uuid("reel_id") {
        let reel = reels::get(tx.conn(), reel_id).await?;
        let topic = topics::get(tx.conn(), reel.topic_id).await?;
        let input = ScriptInput {
            source_title: source.title.clone(),
            source_text: extraction.text.clone(),
            key_ideas,
            topic_name: topic.name,
            language: reel.language.clone(),
            reel_count: 1,
            brief: job.param_str("brief"),
            presenter_name,
        };
        let script = ctx
            .providers
            .writer
            .write_reels(input)
            .await
            .map_err(perr)?
            .into_iter()
            .next()
            .ok_or_else(|| JobError::Permanent("no script returned".into()))?;
        reels::update(
            tx.conn(),
            reel.id,
            reels::ReelUpdate {
                title: Some(script.title),
                script: Some(script.script),
                source_citation: Some(script.source_citation),
                presenter: None,
                compliance_tag: None,
                video_key: None,
                thumbnail_key: None,
                captions_key: None,
                duration_ms: None,
                consent_confirmed: None,
            },
        )
        .await?;
        return Ok(StepResult::Done("1 script rewritten".into()));
    }

    let existing = reels::for_job(tx.conn(), job.id).await?;
    if !existing.is_empty() {
        return Ok(StepResult::Done(format!(
            "{} reels already scripted",
            existing.len()
        )));
    }
    let topic_id = topic_id
        .filter(|t| !t.is_nil())
        .ok_or_else(|| JobError::Permanent("job has no topic_id".into()))?;
    let topic = topics::get(tx.conn(), topic_id).await?;
    let reel_count = job.params["reel_count"].as_u64().unwrap_or(3).clamp(1, 12) as usize;
    let input = ScriptInput {
        source_title: source.title.clone(),
        source_text: extraction.text.clone(),
        key_ideas,
        topic_name: topic.name.clone(),
        language: language.clone(),
        reel_count,
        brief: job.param_str("brief"),
        presenter_name,
    };
    let scripts = ctx
        .providers
        .writer
        .write_reels(input)
        .await
        .map_err(perr)?;
    let kind: ReelKind = job_style(job).reel_kind();
    for s in scripts.iter().take(reel_count) {
        let (tags, difficulty) = s.clean_features();
        let created = reels::create(
            tx.conn(),
            tid,
            reels::NewReel {
                topic_id,
                source_id: Some(job.source_id),
                generation_job_id: Some(job.id),
                kind,
                status: ReelStatus::Draft,
                title: &s.title,
                language: &language,
                script: Some(&s.script),
                source_citation: Some(&s.source_citation),
                presenter: presenter.clone(),
                video_key: None,
                captions_key: None,
                thumbnail_key: None,
                duration_ms: None,
                compliance_tag: topic.compliance_tag.as_deref(),
                consent_confirmed: true,
                created_by: job.requested_by,
            },
        )
        .await?;
        reels::set_features(tx.conn(), created.id, &tags, difficulty).await?;
    }
    Ok(StepResult::Done(format!(
        "{} reels scripted",
        scripts.len().min(reel_count)
    )))
}

async fn question_step(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
) -> Result<StepResult, JobError> {
    let tid = tx.tenant_id();
    let mut written = 0;
    for reel in job_reels(tx, job).await? {
        let script = reel.script.clone().unwrap_or_else(|| reel.title.clone());
        let q = ctx
            .providers
            .writer
            .write_question(&script, &reel.language)
            .await
            .map_err(perr)?;
        reels::upsert_question(
            tx.conn(),
            tid,
            reel.id,
            &reel.language,
            &q.prompt,
            &q.options_json(),
            &q.correct_key,
            Some(&q.explanation),
        )
        .await?;
        written += 1;
    }
    Ok(StepResult::Done(format!("{written} questions")))
}

async fn caption_step(
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
) -> Result<StepResult, JobError> {
    let tid = tx.tenant_id();
    let mut n = 0;
    for reel in job_reels(tx, job).await? {
        let script = reel.script.clone().unwrap_or_default();
        let vtt = vtt_from_script(&script, reel.duration_ms.unwrap_or(45_000));
        let key = reel_media_key(tx.tenant_id(), reel.id, reel.version, &reel.language, "vtt");
        // Captions are tiny: they are written through the same storage path as media when the worker has it;
        // the key is recorded now so the editor can preview immediately, and rewritten on the next render if needed.
        store_text(tx, &key, "text/vtt", vtt).await?;
        reels::update(
            tx.conn(),
            reel.id,
            reels::ReelUpdate {
                title: None,
                script: None,
                source_citation: None,
                presenter: None,
                compliance_tag: None,
                video_key: None,
                thumbnail_key: None,
                captions_key: Some(key.clone()),
                duration_ms: None,
                consent_confirmed: None,
            },
        )
        .await?;
        reels::upsert_variant(
            tx.conn(),
            tid,
            reel.id,
            &reel.language,
            reel.video_key.as_deref(),
            Some(&key),
            None,
            reel.script.as_deref(),
            ExtractionStatus::Done,
        )
        .await?;
        n += 1;
    }
    Ok(StepResult::Done(format!("{n} caption tracks")))
}

/// Storage writes queued on the transaction's context: executed by the caller through `Ctx` once committed.
/// Keeping them here keeps the step logic free of HTTP; `run_step` flushes them.
async fn store_text(
    tx: &mut TenantTx,
    key: &str,
    mime: &str,
    body: String,
) -> Result<(), JobError> {
    PENDING_UPLOADS.with(|p| {
        p.borrow_mut().push((
            tx.tenant_id(),
            key.to_string(),
            mime.to_string(),
            body.into_bytes(),
        ))
    });
    Ok(())
}

/// (tenant, object key, mime, bytes) recorded during a step and uploaded after its transaction commits.
type PendingUpload = (Uuid, String, String, Vec<u8>);

thread_local! {
    static PENDING_UPLOADS: std::cell::RefCell<Vec<PendingUpload>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Upload artefacts recorded during a step. Called by the queue after the step's transaction committed.
pub async fn flush_uploads(ctx: &Ctx) {
    let items: Vec<(Uuid, String, String, Vec<u8>)> =
        PENDING_UPLOADS.with(|p| std::mem::take(&mut *p.borrow_mut()));
    for (_, key, mime, bytes) in items {
        if let Err(e) = ctx
            .state
            .storage
            .put_bytes(&ctx.state.http, &key, &mime, bytes)
            .await
        {
            tracing::warn!(error = %e, key = %key, "artefact upload failed (is object storage running?)");
        }
    }
}

async fn translate_step(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
) -> Result<StepResult, JobError> {
    let tid = tx.tenant_id();
    let languages = job.param_languages();
    let master = master_language(job);
    let mut n = 0;
    for reel in job_reels(tx, job).await? {
        let script = reel.script.clone().unwrap_or_default();
        let question =
            reels::question_for_reel_in(tx.conn(), reel.id, &reel.language, &reel.language)
                .await?
                .map(|q| QuestionDraft {
                    prompt: q.prompt,
                    options: q
                        .options
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|o| {
                                    Some((
                                        o["key"].as_str()?.to_string(),
                                        o["text"].as_str()?.to_string(),
                                    ))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    correct_key: q.correct_key,
                    explanation: q.explanation.unwrap_or_default(),
                });
        for lang in languages
            .iter()
            .filter(|l| **l != master && **l != reel.language)
        {
            if reels::variant(tx.conn(), reel.id, lang)
                .await?
                .map(|v| v.dub_status == ExtractionStatus::Done)
                .unwrap_or(false)
            {
                continue;
            }
            let (script_t, q_t) = ctx
                .providers
                .writer
                .translate(&script, question.as_ref(), &reel.language, lang)
                .await
                .map_err(perr)?;
            let vtt = vtt_from_script(&script_t, reel.duration_ms.unwrap_or(45_000));
            let captions_key = reel_media_key(tx.tenant_id(), reel.id, reel.version, lang, "vtt");
            store_text(tx, &captions_key, "text/vtt", vtt).await?;
            // Dubbing reuses the master video until a dub provider is configured: the captions carry the language.
            reels::upsert_variant(
                tx.conn(),
                tid,
                reel.id,
                lang,
                reel.video_key.as_deref(),
                Some(&captions_key),
                None,
                Some(&script_t),
                ExtractionStatus::Done,
            )
            .await?;
            if let Some(q) = q_t {
                reels::upsert_question(
                    tx.conn(),
                    tid,
                    reel.id,
                    lang,
                    &q.prompt,
                    &q.options_json(),
                    &q.correct_key,
                    Some(&q.explanation),
                )
                .await?;
            }
            n += 1;
        }
    }
    Ok(StepResult::Done(format!("{n} language variants")))
}

/// Render, voice and cut share one shape: per reel, ask the provider, store bytes or park on a provider ref.
async fn media_step(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
    kind: &str,
    only: Option<Uuid>,
) -> Result<StepResult, JobError> {
    let all = match only {
        Some(id) => vec![reels::get(tx.conn(), id).await?],
        None => job_reels(tx, job).await?,
    };
    let mut queue: Vec<Uuid> = all.iter().map(|r| r.id).collect();
    let mut done = 0;
    while let Some(reel_id) = queue.first().copied() {
        let reel = reels::get(tx.conn(), reel_id).await?;
        let language = reel.language.clone();
        match request_media(ctx, tx, job, &reel, kind).await? {
            MediaResult::Ready {
                bytes,
                mime,
                duration_ms,
                poster,
            } => {
                apply_media(
                    ctx,
                    tx,
                    &reel,
                    &language,
                    kind,
                    bytes,
                    &mime,
                    duration_ms,
                    poster,
                )
                .await?;
                done += 1;
                queue.remove(0);
            }
            MediaResult::Pending { provider_ref } => {
                queue.remove(0);
                return Ok(StepResult::Pending(
                    Pending {
                        kind: kind.into(),
                        reel_id,
                        language,
                        remaining: queue,
                    },
                    provider_ref,
                ));
            }
        }
    }
    Ok(StepResult::Done(format!("{done} reels {kind}ed")))
}

async fn request_media(
    ctx: &Ctx,
    tx: &mut TenantTx,
    job: &studio::GenerationJobRow,
    reel: &reels::ReelRow,
    kind: &str,
) -> Result<MediaResult, JobError> {
    let script = reel.script.clone().unwrap_or_else(|| reel.title.clone());
    match kind {
        "render" => {
            let caption_line = script
                .split(['.', '!', '؟', '۔'])
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(70)
                .collect();
            let audio_url = reels::variant(tx.conn(), reel.id, &reel.language)
                .await?
                .and_then(|v| v.audio_key)
                .map(|k| ctx.state.storage.presign_get(&k));
            let req = RenderRequest {
                reel_id: reel.id,
                style: job_style(job).reel_kind().as_str().to_string(),
                script,
                caption_line,
                language: reel.language.clone(),
                presenter: reel.presenter.clone(),
                audio_url,
            };
            ctx.providers.video.render(req).await.map_err(perr)
        }
        "voice" => {
            let voice_id = job.params["presenter"]["voice_id"]
                .as_str()
                .map(|s| s.to_string());
            ctx.providers
                .voice
                .synthesize(&script, &reel.language, voice_id.as_deref())
                .await
                .map_err(perr)
        }
        "cut" => {
            let source = studio::get_source(tx.conn(), job.source_id).await?;
            let asset = match source.media_asset_id {
                Some(id) => media::get(tx.conn(), id).await?,
                None => return Err(JobError::Permanent("podcast source has no media".into())),
            };
            let start = job.params["start_ms"].as_i64().unwrap_or(0) as i32;
            let end = job.params["end_ms"]
                .as_i64()
                .unwrap_or(i64::from(start) + 45_000) as i32;
            let req = CutRequest {
                reel_id: reel.id,
                source_url: ctx.state.storage.presign_get(&asset.storage_key),
                start_ms: start,
                end_ms: end,
            };
            ctx.providers.video.cut(req).await.map_err(perr)
        }
        other => Err(JobError::Permanent(format!("unknown media step {other}"))),
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_media(
    ctx: &Ctx,
    tx: &mut TenantTx,
    reel: &reels::ReelRow,
    language: &str,
    kind: &str,
    bytes: Vec<u8>,
    mime: &str,
    duration_ms: i32,
    poster: Option<Vec<u8>>,
) -> Result<(), JobError> {
    let tid = tx.tenant_id();
    let _ = ctx;
    match kind {
        "render" | "cut" => {
            let key = reel_media_key(tx.tenant_id(), reel.id, reel.version, language, "mp4");
            PENDING_UPLOADS.with(|p| {
                p.borrow_mut()
                    .push((tx.tenant_id(), key.clone(), mime.to_string(), bytes))
            });
            let thumbnail_key = poster.map(|img| {
                let tkey = reel_media_key(tid, reel.id, reel.version, language, "jpg");
                PENDING_UPLOADS.with(|p| {
                    p.borrow_mut()
                        .push((tid, tkey.clone(), "image/jpeg".to_string(), img))
                });
                tkey
            });
            let captions = reels::variant(tx.conn(), reel.id, language)
                .await?
                .and_then(|v| v.captions_key);
            reels::update(
                tx.conn(),
                reel.id,
                reels::ReelUpdate {
                    title: None,
                    script: None,
                    source_citation: None,
                    presenter: None,
                    compliance_tag: None,
                    video_key: Some(key.clone()),
                    thumbnail_key,
                    captions_key: None,
                    duration_ms: Some(duration_ms.clamp(1000, 120_000)),
                    consent_confirmed: None,
                },
            )
            .await?;
            reels::upsert_variant(
                tx.conn(),
                tid,
                reel.id,
                language,
                Some(&key),
                captions.as_deref(),
                None,
                reel.script.as_deref(),
                ExtractionStatus::Done,
            )
            .await?;
        }
        "voice" => {
            let key = reel_media_key(tx.tenant_id(), reel.id, reel.version, language, "mp3");
            PENDING_UPLOADS.with(|p| {
                p.borrow_mut()
                    .push((tx.tenant_id(), key.clone(), mime.to_string(), bytes))
            });
            let existing = reels::variant(tx.conn(), reel.id, language).await?;
            reels::upsert_variant(
                tx.conn(),
                tid,
                reel.id,
                language,
                existing.as_ref().and_then(|v| v.video_key.as_deref()),
                existing.as_ref().and_then(|v| v.captions_key.as_deref()),
                Some(&key),
                reel.script.as_deref(),
                ExtractionStatus::Done,
            )
            .await?;
            if reel.duration_ms.is_none() {
                reels::update(
                    tx.conn(),
                    reel.id,
                    reels::ReelUpdate {
                        title: None,
                        script: None,
                        source_citation: None,
                        presenter: None,
                        compliance_tag: None,
                        video_key: None,
                        thumbnail_key: None,
                        captions_key: None,
                        duration_ms: Some(duration_ms.clamp(1000, 120_000)),
                        consent_confirmed: None,
                    },
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

// ------------------------------------------------------------------ provider callbacks

/// `{ provider, payload: { ref, status: done|failed, url?, duration_ms?, error?, segments? } }`
pub async fn provider_callback(
    ctx: &Ctx,
    payload: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let provider = payload["provider"]
        .as_str()
        .unwrap_or("provider")
        .to_string();
    let body = &payload["payload"];
    let provider_ref = body["ref"]
        .as_str()
        .or_else(|| body["id"].as_str())
        .or_else(|| body["job_id"].as_str())
        .ok_or_else(|| JobError::Permanent("callback without ref".into()))?
        .to_string();
    let status = body["status"].as_str().unwrap_or("done").to_string();

    let mut admin = ctx.state.db.admin_tx().await?;
    let step = studio::step_by_provider_ref(&mut admin, &provider_ref).await?;
    let source = if step.is_none() {
        studio::source_by_pending_ref(&mut admin, &provider_ref).await?
    } else {
        None
    };
    let job_for_step = match &step {
        Some(s) => Some(studio::get_job(&mut admin, s.job_id).await?),
        None => None,
    };
    admin
        .commit()
        .await
        .map_err(|e| JobError::Transient(e.to_string()))?;

    if let (Some(step), Some(job)) = (step, job_for_step) {
        return complete_media_step(ctx, &job, &step, &provider_ref, &status, body).await;
    }
    if let Some((tenant_id, source_id)) = source {
        let mut tx = ctx.state.db.tenant_tx(tenant_id).await?;
        if status != "done" {
            let msg = body["error"]
                .as_str()
                .unwrap_or("transcription failed")
                .to_string();
            studio::set_extraction_status(
                tx.conn(),
                source_id,
                ExtractionStatus::Failed,
                Some(&msg),
            )
            .await?;
            tx.commit().await?;
            return Ok(JobOutcome::done(format!("transcription failed: {msg}")));
        }
        let segments = real::segments_from_callback(&body["segments"]);
        if segments.is_empty() {
            return Err(JobError::Permanent(
                "transcription callback carries no segments".into(),
            ));
        }
        let n = finish_transcript(
            ctx,
            &mut tx,
            source_id,
            &segments,
            body["language"].as_str(),
        )
        .await?;
        tx.commit().await?;
        return Ok(JobOutcome::done(format!(
            "{provider} transcript: {} segments, {n} moments",
            segments.len()
        )));
    }
    tracing::warn!(%provider, %provider_ref, "callback for an unknown provider ref; ignored");
    Ok(JobOutcome::done("unknown ref ignored".into()))
}

async fn complete_media_step(
    ctx: &Ctx,
    job: &studio::GenerationJobRow,
    step: &studio::StepRow,
    provider_ref: &str,
    status: &str,
    body: &serde_json::Value,
) -> Result<JobOutcome, JobError> {
    let pending: Pending = step
        .log
        .as_deref()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or_else(|| JobError::Permanent(format!("step {} has no pending record", step.id)))?;
    let mut tx = ctx.state.db.tenant_tx(job.tenant_id).await?;
    if status != "done" {
        let msg = body["error"]
            .as_str()
            .unwrap_or("provider reported failure")
            .to_string();
        studio::update_step(
            tx.conn(),
            step.id,
            JobStatus::Failed,
            None,
            None,
            Some(&msg),
            false,
        )
        .await?;
        studio::set_job_status(tx.conn(), job.id, JobStatus::Failed, None, Some(&msg)).await?;
        tx.commit().await?;
        return Ok(JobOutcome::done(format!("step failed at provider: {msg}")));
    }
    let url = body["url"]
        .as_str()
        .ok_or_else(|| JobError::Permanent("callback without url".into()))?;
    let resp = ctx
        .state
        .http
        .get(url)
        .send()
        .await
        .map_err(|e| JobError::Transient(format!("fetching provider output: {e}")))?;
    if !resp.status().is_success() {
        return Err(JobError::Transient(format!(
            "provider output not ready: {}",
            resp.status()
        )));
    }
    let mime = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(if pending.kind == "voice" {
            "audio/mpeg"
        } else {
            "video/mp4"
        })
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| JobError::Transient(e.to_string()))?
        .to_vec();
    let duration_ms = body["duration_ms"].as_i64().unwrap_or(45_000) as i32;
    let reel = reels::get(tx.conn(), pending.reel_id).await?;
    apply_media(
        ctx,
        &mut tx,
        &reel,
        &pending.language,
        &pending.kind,
        bytes,
        &mime,
        duration_ms,
        None,
    )
    .await?;

    // Continue with the next reel of the same step, or finish the step.
    let mut remaining = pending.remaining.clone();
    while let Some(next_id) = remaining.first().copied() {
        let next = reels::get(tx.conn(), next_id).await?;
        let language = next.language.clone();
        match request_media(ctx, &mut tx, job, &next, &pending.kind).await? {
            MediaResult::Ready {
                bytes,
                mime,
                duration_ms,
                poster,
            } => {
                apply_media(
                    ctx,
                    &mut tx,
                    &next,
                    &language,
                    &pending.kind,
                    bytes,
                    &mime,
                    duration_ms,
                    poster,
                )
                .await?;
                remaining.remove(0);
            }
            MediaResult::Pending {
                provider_ref: new_ref,
            } => {
                remaining.remove(0);
                let log = serde_json::to_string(&Pending {
                    kind: pending.kind.clone(),
                    reel_id: next_id,
                    language,
                    remaining,
                })
                .unwrap_or_default();
                studio::update_step(
                    tx.conn(),
                    step.id,
                    JobStatus::Running,
                    None,
                    Some(&new_ref),
                    Some(&log),
                    false,
                )
                .await?;
                tx.commit().await?;
                return Ok(JobOutcome::done(format!(
                    "{provider_ref} applied; next reel pending at {new_ref}"
                )));
            }
        }
    }
    studio::update_step(
        tx.conn(),
        step.id,
        JobStatus::Done,
        None,
        None,
        Some("completed via provider callback"),
        false,
    )
    .await?;
    advance(&mut tx, job, step.position).await?;
    tx.commit().await?;
    Ok(JobOutcome::done(format!(
        "{provider_ref} applied; step {} done",
        step.step.as_str()
    )))
}
