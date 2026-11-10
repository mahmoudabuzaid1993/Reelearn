# The learning engine

The engine decides which reel each person sees next, how long practice can run, and when to remind people. It follows the published structure of short-video recommenders (Meta Reels, TikTok, YouTube, Kuaishou). Its value model rewards learning as well as attention. Each company tunes how hard it pushes, from **Gentle** to **Maximum**.

The research behind it, with a label on every number (official, paper, reported or our choice), is in [research/short-video-recommenders.md](research/short-video-recommenders.md).

## 1. The pipeline

| Stage | Large platforms | Reelearn |
|---|---|---|
| Event logging | Every impression, view, skip, like, share, "not interested" | `engagement_events`, partitioned by month; counters in `reel_engagement_stats` updated in the same transaction |
| Retrieval | Several candidate sources mixed (two-tower nearest neighbours, follows, trends) | One SQL query per request. Candidates are the reels assigned to the person, the open catalogue when the company allows it, reels a colleague sent, and due reviews. A company catalogue is hundreds to a few thousand reels, so nothing needs an approximate index |
| Early ranking | A light model distilled from the late ranker | Not needed at this size |
| Late ranking | A multi-task network; predictions combined in a weighted sum, negatives subtracted | Smoothed predictions per signal, combined by the value model below, with weights the company sets |
| Re-ranking | Integrity, diversity, exploration | Only approved, published reels. Colleague shares lead, the run of one topic is capped, a share of slots explores under-seen reels, reviews are interleaved |

Code: pure ranking in `api/crates/domain/src/engine/`, loading in `api/crates/db/src/engine.rs` and `repos/engine.rs`, HTTP in `api/crates/api/src/services/engine.rs` and `routes/engine.rs`, upkeep in `api/crates/worker/src/jobs/engine.rs`.

## 2. Signals

| Signal | Source | Interest change (topic; tags get half) |
|---|---|---|
| impression | the ranker, with score and reason | 0 |
| start | `/me/watch` with 0 ms, or `/me/events` | +0.05 |
| skip (left before 3 s) | `/me/events`, or a short `/me/watch` | −0.6 |
| complete (past 90 %) | `/me/watch` | +1 |
| rewatch | `/me/events` | +1.5 |
| save / unsave | `/me/saved/{id}` | +2 / −1 |
| share | `/me/shares` | +3 |
| not_relevant ("not for me") | `/me/events` | −4 |
| more_like_this | `/me/events` | +3 |
| correct / wrong answer | `/me/answers` | +0.4 / +0.15 |

Interest is a time-decayed sum that is **not normalised**, so ten completions count ten times. This follows Meta's note on HSTU that the intensity of a preference is itself a signal. Each event decays the stored score to now, then adds:

```text
score(now) = clamp(score(t) · 0.5^((now − t) / half_life) + Δ, −20, 50)
```

Readers decay to the current time the same way. The half-life is a company setting, 14 days by default.

## 3. Predictions and the value model

Each prediction is a smoothed rate, pulled toward a prior until there is evidence (`(k + prior·m) / (n + m)`), then moved by the person's interest:

- **P(complete)** is the reel's completion rate, smoothed toward the company's rate for reels **of the same length band**. The value model uses the *lift*, P(complete) divided by the band rate, so a 70-second reel finished by 60 % beats a 15-second reel finished by 75 % when their bands average 40 % and 90 %. This is Meta's "watch more than 95 % of users for reels of the same length" and Kuaishou's duration debiasing, reduced to five bands.
- **P(skip under 3 s)**, **P(save)**, **P(share)** and **P(not for me)**. A muted topic makes P(not for me) 1.
- **P(correct)** is the question's answer rate, or a prior from the studio's difficulty rating (1 to 5) until 8 answers exist. It is shifted in logit space by the person's own answer rate.

The score:

```text
score = w_completion · lift/2                         (lift capped at 2)
      + w_learning   · max(0, 1 − ((P(correct) − target)/0.3)²)
      + w_save       · min(10·P(save), 1.5)
      + w_share      · min(20·P(share), 1.5)
      + w_interest   · (tanh(interest/5) + 0.5 if followed)
      + w_urgency    · (1 + days-to-deadline boost if mandatory, 0.3 if assigned)
      + w_freshness  · e^(−age/10 days)
      + w_social     · cohort popularity (location, job title, team; 0..1)
      − w_skip       · min(2·P(skip), 2)
      − w_not_relevant · min(10·P(not for me), 3)
      − w_repeat     · [finished before]
      + 3 if a colleague sent it
```

The scale factors put every term on roughly 0 to 1, so the weights read as relative importance. The learning term peaks at the target answer rate, 85 % by default, following the "85 % rule" for learning.

## 4. Re-ranking

1. **Colleague shares first.** At most two per batch, then the share is marked opened when the reel starts.
2. **Exploration.** In each slot, with probability `exploration_rate`, the best reel with fewer than 30 impressions is chosen by an upper-confidence bonus `√(ln N / (n + 1))`. A person with no interest history gets 2.5 times the rate for their first 20 items. The draw is seeded by person, session and position, so a request ranks the same way twice.
3. **Diversity.** No more than `max_same_topic_in_row` reels of one topic in a row.
4. **Reviews.** Every `review_every` reels, a spaced-repetition question the person got wrong or that is due within three days.
5. **Muted topics** are dropped unless mandatory.

The daily plan uses the same value model to order its new reels after mandatory ones. Its caps and review rules are unchanged.

## 5. Practice sessions and the budget

`GET /me/next` opens or resumes a session. Each batch is logged as impressions. The phone reports practice time with `POST /me/events {active_ms}` every 30 seconds while the screen is in front.

Practice closes, with `budget.stop` saying why, when any of these holds:

- the company switched practice off
- the person is paused
- it is outside the delivery windows and the company does not allow practice outside work hours
- the daily or session minutes are spent
- nothing is left to show

The phone shows a pause card every `break_reminder_minutes`. Reels finished in the last three days are left out of practice.

## 6. What a company tunes

Presets are our choices; editing any value makes the settings **Custom**.

| Setting | Gentle | Balanced (default) | Intense | Maximum | Range |
|---|---|---|---|---|---|
| Practice after the minute | off | on | on | on | |
| Practice minutes a day | n/a | 15 | 45 | no cap | 0 to 240 (0 = no cap) |
| Minutes a session | n/a | 10 | 20 | no cap | 0 to 240 |
| Pause suggestion every | n/a | 10 min | 20 min | never | 0 to 120 |
| Practice outside work hours | no | no | yes | yes | |
| Reminders a day | 1 | 1 | 2 | 3 | 0 to 6 |
| Remind after hours away | never | 24 | 8 | 4 | 0 to 72 |
| Exploration | 5 % | 10 % | 12 % | 15 % | 0 to 50 % |
| Interest half-life | 21 d | 14 d | 10 d | 7 d | 1 to 90 |
| Same topic in a row | 1 | 2 | 2 | 3 | 1 to 5 |
| Review every N reels | never | 4 | 5 | 6 | 0 to 10 |
| Open catalogue, colleague activity | off | on | on | on | |
| Completion / learning weight | 1.0 / 1.5 | 1.2 / 1.2 | 1.6 / 0.9 | 2.0 / 0.6 | 0 to 5 |
| Share / interest weight | 0.5 / 0.8 | 1.0 / 1.0 | 1.4 / 1.4 | 1.8 / 1.8 | 0 to 5 |

Admins use **Learning engine** in the console, or the API directly:

- `GET /admin/engine` returns the settings and the four presets.
- `PUT /admin/engine {"intensity":"intense"}` switches preset.
- `PUT /admin/engine {"daily_minutes_cap":30,"weights":{"share":2}}` changes fields; the result is re-labelled Custom.

Every change is in the audit log with the before and after values.

## 7. Guardrails that are not tunable

- **"Not for me" and quick skips always count.** Their weights cannot go below 1, because they are the person's own signals.
- **Mandatory topics are never hidden.** Mute is refused for them.
- **Each person can turn personalisation off** (`PUT /me/personalization`). Practice is then ordered without interest, colleague activity or exploration. This follows EU DSA Articles 27 and 38 as best practice.
- **Purpose limitation (Saudi PDPL).** Engagement and interest data choose reels and nothing else. They never appear in readiness, manager views or performance reviews. The personal-data export includes all of it: interest per topic and tag, signal counts, practice sessions and colleague shares.
- **Group thresholds.** Insights leave out locations with fewer than three people practising.
- **Reminders only inside delivery windows**, only for people with reminders on, never more than the company's daily number, and never within the "remind after" interval of the last one.
- **No emotion inference, no variable rewards** (EU AI Act Article 5).

## 8. Insights

`GET /admin/engine/insights?range=30d` reports:

- practice minutes, sessions and reels per session
- completion and quick-skip rates
- saves, "not for me" and sharing, with the opened rate
- daily practice
- interest by topic, with follows and mutes, and by tag
- practice by location
- completion by length band
- why reels were shown
- per-reel performance, to fix or retire weak reels

## 9. Upkeep

The scheduler enqueues `engine_hourly` per tenant once an hour. It:

- recomputes the duration-band baselines from the running reel statistics
- rebuilds the 7-day colleague activity table (`cohort_reel_stats`) from the event log
- schedules re-engagement reminders, which `send_delivery` sends like any other reminder

Partition maintenance covers `engagement_events`.

## 10. Tests

| Where | What |
|---|---|
| `domain::engine` (24 unit tests) | presets and validation, interest decay, duration normalisation, skips, interest and mute, shares first, topic runs, reproducible exploration, the learning term's peak, zero weights, urgency, the budget |
| `api/tests/engine.rs` (7 HTTP tests on Postgres) | practice batches and impressions, the budget closing by settings and by session time, signals moving interest and statistics, follow and mute, sharing and its access rules, admin tuning with validation and audit, insights, personalisation off |
| `web` | settings logic (vitest), and Playwright for switching preset, tuning and saving as Custom, and the insights tab |
| `mobile` | the practice controller (batches, the end card, practice off, pause cards and the session cap, signals), the practice screen (reason line, send to a colleague, "not for me"), "Your topics", "Keep going" |
