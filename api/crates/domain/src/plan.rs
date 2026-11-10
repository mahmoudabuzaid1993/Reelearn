//! Daily plan selection: the "minute" is computed, not browsed.
//!
//! 1. Due reviews (`memory_states.due_on <= today`), oldest due first, capped by the
//!    tenant's `max_reviews_per_day`.
//! 2. New published reels from the learner's group assignments that they have not
//!    seen, each assignment contributing at most its `reels_per_day`, the whole plan
//!    capped by the tenant's `reels_per_day` (plus one when the learner opted into
//!    extra reels). Mandatory assignments go first, then older reels first so a
//!    topic is learned in publication order.
//!
//! The selection is deterministic for a given input so plan building is idempotent.

use crate::enums::{Channel, PlanItemKind, PlanItemReason};
use chrono::NaiveDate;
use std::collections::HashMap;
use uuid::Uuid;

/// A review that is due: one memory state joined to its question and reel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueReview {
    pub question_id: Uuid,
    pub reel_id: Uuid,
    pub due_on: NaiveDate,
    pub interval_days: i16,
}

/// A published, unseen reel reachable through one of the learner's assignments.
#[derive(Debug, Clone, PartialEq)]
pub struct NewReelCandidate {
    pub reel_id: Uuid,
    /// The question in the learner's language (or the master language as a fallback).
    pub question_id: Option<Uuid>,
    pub topic_id: Uuid,
    pub assignment_id: Uuid,
    pub assignment_reels_per_day: i16,
    pub assignment_channel: Channel,
    pub is_mandatory: bool,
    pub published_at: chrono::DateTime<chrono::Utc>,
    /// The learning engine's value-model score; higher is chosen first after mandatory reels.
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanInput {
    pub today: NaiveDate,
    pub max_reviews_per_day: usize,
    pub reels_per_day: usize,
    pub extra_reels_on: bool,
    pub due_reviews: Vec<DueReview>,
    pub candidates: Vec<NewReelCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItemSpec {
    pub position: i16,
    pub kind: PlanItemKind,
    pub reason: PlanItemReason,
    pub reel_id: Option<Uuid>,
    pub question_id: Option<Uuid>,
    /// For reviews: the current interval, used for the "Asked again · day N" line.
    pub interval_days: Option<i16>,
}

/// Rough seconds the plan takes: 30 s per reel, 10 s per question-only review.
pub fn estimated_seconds(items: &[PlanItemSpec]) -> i32 {
    items
        .iter()
        .map(|i| match i.kind {
            PlanItemKind::Reel => 30,
            PlanItemKind::QuestionOnly => 10,
        })
        .sum()
}

pub fn build_plan(input: &PlanInput) -> Vec<PlanItemSpec> {
    let mut items = Vec::new();

    // 1. Reviews, oldest due first; ties broken by question id for determinism.
    let mut reviews: Vec<&DueReview> = input
        .due_reviews
        .iter()
        .filter(|r| r.due_on <= input.today)
        .collect();
    reviews.sort_by(|a, b| {
        a.due_on
            .cmp(&b.due_on)
            .then(a.question_id.cmp(&b.question_id))
    });
    for r in reviews.into_iter().take(input.max_reviews_per_day) {
        items.push(PlanItemSpec {
            position: 0,
            kind: PlanItemKind::QuestionOnly,
            reason: PlanItemReason::Review,
            reel_id: Some(r.reel_id),
            question_id: Some(r.question_id),
            interval_days: Some(r.interval_days),
        });
    }

    // 2. New reels: mandatory first, then by publication order.
    let total_cap = input.reels_per_day + usize::from(input.extra_reels_on);
    let mut candidates: Vec<&NewReelCandidate> = input.candidates.iter().collect();
    candidates.sort_by(|a, b| {
        b.is_mandatory
            .cmp(&a.is_mandatory)
            .then(
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.published_at.cmp(&b.published_at))
            .then(a.reel_id.cmp(&b.reel_id))
    });

    let mut per_assignment: HashMap<Uuid, usize> = HashMap::new();
    let mut chosen: Vec<Uuid> = Vec::new();
    for c in candidates {
        if chosen.len() >= total_cap {
            break;
        }
        if chosen.contains(&c.reel_id) {
            continue; // the same reel reachable through two assignments
        }
        let cap = usize::try_from(c.assignment_reels_per_day.max(0)).unwrap_or(0);
        let used = per_assignment.entry(c.assignment_id).or_insert(0);
        if *used >= cap {
            continue;
        }
        *used += 1;
        chosen.push(c.reel_id);
        items.push(PlanItemSpec {
            position: 0,
            kind: PlanItemKind::Reel,
            reason: PlanItemReason::New,
            reel_id: Some(c.reel_id),
            question_id: c.question_id,
            interval_days: None,
        });
    }

    for (i, item) in items.iter_mut().enumerate() {
        item.position = i16::try_from(i).unwrap_or(i16::MAX);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).expect("date")
    }

    fn cand(
        reel: u128,
        assignment: u128,
        per_day: i16,
        mandatory: bool,
        published_day: u32,
    ) -> NewReelCandidate {
        NewReelCandidate {
            reel_id: Uuid::from_u128(reel),
            question_id: Some(Uuid::from_u128(reel + 1000)),
            topic_id: Uuid::from_u128(assignment + 500),
            assignment_id: Uuid::from_u128(assignment),
            assignment_reels_per_day: per_day,
            assignment_channel: Channel::App,
            is_mandatory: mandatory,
            published_at: Utc
                .with_ymd_and_hms(2026, 10, published_day, 8, 0, 0)
                .single()
                .expect("ts"),
            score: 0.0,
        }
    }

    fn base_input() -> PlanInput {
        PlanInput {
            today: d(20),
            max_reviews_per_day: 1,
            reels_per_day: 2,
            extra_reels_on: false,
            due_reviews: vec![],
            candidates: vec![],
        }
    }

    #[test]
    fn reviews_come_first_and_are_capped() {
        let mut input = base_input();
        input.due_reviews = vec![
            DueReview {
                question_id: Uuid::from_u128(2),
                reel_id: Uuid::from_u128(12),
                due_on: d(19),
                interval_days: 3,
            },
            DueReview {
                question_id: Uuid::from_u128(1),
                reel_id: Uuid::from_u128(11),
                due_on: d(10),
                interval_days: 10,
            },
            DueReview {
                question_id: Uuid::from_u128(3),
                reel_id: Uuid::from_u128(13),
                due_on: d(25),
                interval_days: 30,
            },
        ];
        input.candidates = vec![cand(100, 1, 2, false, 1)];
        let items = build_plan(&input);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, PlanItemKind::QuestionOnly);
        assert_eq!(items[0].reason, PlanItemReason::Review);
        assert_eq!(
            items[0].question_id,
            Some(Uuid::from_u128(1)),
            "oldest due first"
        );
        assert_eq!(items[0].interval_days, Some(10));
        assert_eq!(items[1].kind, PlanItemKind::Reel);
        assert_eq!(items[1].position, 1);
    }

    #[test]
    fn future_reviews_are_not_due() {
        let mut input = base_input();
        input.due_reviews = vec![DueReview {
            question_id: Uuid::from_u128(3),
            reel_id: Uuid::from_u128(13),
            due_on: d(25),
            interval_days: 30,
        }];
        assert!(build_plan(&input).is_empty());
    }

    #[test]
    fn new_reels_respect_assignment_and_tenant_caps() {
        let mut input = base_input();
        // Assignment 1 allows 2/day, assignment 2 allows 1/day; tenant cap is 2.
        input.candidates = vec![
            cand(101, 1, 2, false, 3),
            cand(102, 1, 2, false, 4),
            cand(103, 1, 2, false, 5),
            cand(201, 2, 1, false, 1),
            cand(202, 2, 1, false, 2),
        ];
        let items = build_plan(&input);
        let reels: Vec<Uuid> = items.iter().filter_map(|i| i.reel_id).collect();
        assert_eq!(reels, vec![Uuid::from_u128(201), Uuid::from_u128(101)]);
    }

    #[test]
    fn mandatory_assignments_go_first() {
        let mut input = base_input();
        input.candidates = vec![cand(101, 1, 2, false, 1), cand(301, 3, 2, true, 9)];
        let items = build_plan(&input);
        assert_eq!(items[0].reel_id, Some(Uuid::from_u128(301)));
        assert_eq!(items[1].reel_id, Some(Uuid::from_u128(101)));
    }

    #[test]
    fn extra_reels_add_one_to_the_cap() {
        let mut input = base_input();
        input.extra_reels_on = true;
        input.candidates = vec![
            cand(101, 1, 5, false, 1),
            cand(102, 1, 5, false, 2),
            cand(103, 1, 5, false, 3),
            cand(104, 1, 5, false, 4),
        ];
        assert_eq!(build_plan(&input).len(), 3);
    }

    #[test]
    fn the_same_reel_through_two_assignments_is_planned_once() {
        let mut input = base_input();
        let mut via_other = cand(101, 2, 2, false, 1);
        via_other.assignment_id = Uuid::from_u128(2);
        input.candidates = vec![cand(101, 1, 2, false, 1), via_other];
        assert_eq!(build_plan(&input).len(), 1);
    }

    #[test]
    fn zero_per_day_assignment_contributes_nothing() {
        let mut input = base_input();
        input.candidates = vec![cand(101, 1, 0, false, 1)];
        assert!(build_plan(&input).is_empty());
    }

    #[test]
    fn plan_is_deterministic() {
        let mut input = base_input();
        input.candidates = vec![cand(102, 1, 2, false, 2), cand(101, 1, 2, false, 2)];
        let a = build_plan(&input);
        input.candidates.reverse();
        let b = build_plan(&input);
        assert_eq!(a, b);
        assert_eq!(estimated_seconds(&a), 60);
    }

    #[test]
    fn the_engine_score_orders_new_reels_after_mandatory_ones() {
        let mut input = base_input();
        input.reels_per_day = 3;
        let mut low = cand(101, 1, 5, false, 1);
        low.score = 0.2;
        let mut high = cand(102, 1, 5, false, 9);
        high.score = 2.5;
        let mandatory = cand(301, 3, 5, true, 5);
        input.candidates = vec![low, high, mandatory];
        let reels: Vec<Uuid> = build_plan(&input)
            .iter()
            .filter_map(|i| i.reel_id)
            .collect();
        assert_eq!(
            reels,
            vec![
                Uuid::from_u128(301),
                Uuid::from_u128(102),
                Uuid::from_u128(101)
            ]
        );
    }
}
