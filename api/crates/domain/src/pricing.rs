//! Credit pricing for generation quotes. Prices are per step so the storyboard can
//! show the admin where credits go before any render spend.

use crate::enums::{GenStep, GenerationStyle};
use serde::{Deserialize, Serialize};

pub const EXTRACT_FLAT: i32 = 2;
pub const SCRIPT_PER_REEL: i32 = 3;
pub const QUESTION_PER_REEL: i32 = 1;
pub const VOICE_PER_REEL_LANGUAGE: i32 = 4;
pub const CAPTION_PER_REEL_LANGUAGE: i32 = 1;
pub const TRANSLATE_PER_REEL_LANGUAGE: i32 = 3;
pub const TRANSCRIBE_PER_MINUTE: i32 = 1;
pub const CUT_PER_MOMENT: i32 = 5;

pub fn render_per_reel(style: GenerationStyle) -> i32 {
    match style {
        GenerationStyle::AiAvatar => 20,
        GenerationStyle::Motion => 30,
        GenerationStyle::Broll => 12,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteLine {
    pub step: String,
    pub credits: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub cost_credits: i32,
    pub estimated_minutes: i32,
    pub breakdown: Vec<QuoteLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationQuoteInput {
    pub reel_count: i32,
    /// Number of requested languages including the master language (>= 1).
    pub language_count: i32,
    pub style: GenerationStyle,
    /// False when the source already has an extraction on file.
    pub needs_extraction: bool,
}

pub fn quote_generation(input: &GenerationQuoteInput) -> Quote {
    let reels = input.reel_count.max(0);
    let langs = input.language_count.max(1);
    let extra_langs = langs - 1;
    let mut lines = Vec::new();
    if input.needs_extraction {
        lines.push(QuoteLine {
            step: GenStep::Extract.to_string(),
            credits: EXTRACT_FLAT,
        });
    }
    lines.push(QuoteLine {
        step: GenStep::Script.to_string(),
        credits: SCRIPT_PER_REEL * reels,
    });
    lines.push(QuoteLine {
        step: GenStep::Question.to_string(),
        credits: QUESTION_PER_REEL * reels,
    });
    lines.push(QuoteLine {
        step: GenStep::Render.to_string(),
        credits: render_per_reel(input.style) * reels,
    });
    lines.push(QuoteLine {
        step: GenStep::Voice.to_string(),
        credits: VOICE_PER_REEL_LANGUAGE * reels * langs,
    });
    lines.push(QuoteLine {
        step: GenStep::Caption.to_string(),
        credits: CAPTION_PER_REEL_LANGUAGE * reels * langs,
    });
    if extra_langs > 0 {
        lines.push(QuoteLine {
            step: GenStep::Translate.to_string(),
            credits: TRANSLATE_PER_REEL_LANGUAGE * reels * extra_langs,
        });
    }
    let cost = lines.iter().map(|l| l.credits).sum();
    Quote {
        cost_credits: cost,
        estimated_minutes: estimate_minutes(reels, langs),
        breakdown: lines,
    }
}

fn estimate_minutes(reels: i32, langs: i32) -> i32 {
    // Roughly two minutes per reel render plus half a minute per extra language per reel.
    (2 * reels + (langs - 1) * reels / 2).max(1)
}

/// Regenerating parts of one reel.
pub fn quote_regenerate(what: &[String], style: GenerationStyle, language_count: i32) -> Quote {
    let langs = language_count.max(1);
    let mut lines = Vec::new();
    for w in what {
        match w.as_str() {
            "visual" => lines.push(QuoteLine {
                step: GenStep::Render.to_string(),
                credits: render_per_reel(style),
            }),
            "voice" => lines.push(QuoteLine {
                step: GenStep::Voice.to_string(),
                credits: VOICE_PER_REEL_LANGUAGE * langs,
            }),
            "script" => lines.push(QuoteLine {
                step: GenStep::Script.to_string(),
                credits: SCRIPT_PER_REEL,
            }),
            "question" => lines.push(QuoteLine {
                step: GenStep::Question.to_string(),
                credits: QUESTION_PER_REEL,
            }),
            _ => {}
        }
    }
    let cost = lines.iter().map(|l| l.credits).sum();
    Quote {
        cost_credits: cost,
        estimated_minutes: (lines.len() as i32).max(1),
        breakdown: lines,
    }
}

/// Adding language variants (dub + captions) to an existing reel.
pub fn quote_variants(new_language_count: i32) -> Quote {
    let n = new_language_count.max(0);
    let lines = vec![
        QuoteLine {
            step: GenStep::Translate.to_string(),
            credits: TRANSLATE_PER_REEL_LANGUAGE * n,
        },
        QuoteLine {
            step: GenStep::Voice.to_string(),
            credits: VOICE_PER_REEL_LANGUAGE * n,
        },
        QuoteLine {
            step: GenStep::Caption.to_string(),
            credits: CAPTION_PER_REEL_LANGUAGE * n,
        },
    ];
    let cost = lines.iter().map(|l| l.credits).sum();
    Quote {
        cost_credits: cost,
        estimated_minutes: n.max(1),
        breakdown: lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_language_avatar_quote() {
        let q = quote_generation(&GenerationQuoteInput {
            reel_count: 3,
            language_count: 1,
            style: GenerationStyle::AiAvatar,
            needs_extraction: true,
        });
        // extract 2 + script 9 + question 3 + render 60 + voice 12 + caption 3 = 89
        assert_eq!(q.cost_credits, 89);
        assert!(q.breakdown.iter().all(|l| l.step != "translate"));
        assert_eq!(q.estimated_minutes, 6);
    }

    #[test]
    fn extra_languages_add_translation_voice_and_captions() {
        let q = quote_generation(&GenerationQuoteInput {
            reel_count: 2,
            language_count: 3,
            style: GenerationStyle::Broll,
            needs_extraction: false,
        });
        // script 6 + question 2 + render 24 + voice 4*2*3=24 + caption 1*2*3=6 + translate 3*2*2=12 = 74
        assert_eq!(q.cost_credits, 74);
        assert!(q
            .breakdown
            .iter()
            .any(|l| l.step == "translate" && l.credits == 12));
        assert!(q.breakdown.iter().all(|l| l.step != "extract"));
    }

    #[test]
    fn motion_costs_more_than_broll() {
        let m = quote_generation(&GenerationQuoteInput {
            reel_count: 1,
            language_count: 1,
            style: GenerationStyle::Motion,
            needs_extraction: false,
        });
        let b = quote_generation(&GenerationQuoteInput {
            reel_count: 1,
            language_count: 1,
            style: GenerationStyle::Broll,
            needs_extraction: false,
        });
        assert!(m.cost_credits > b.cost_credits);
    }

    #[test]
    fn regenerate_and_variant_quotes() {
        let r = quote_regenerate(
            &["visual".into(), "voice".into(), "bogus".into()],
            GenerationStyle::AiAvatar,
            2,
        );
        assert_eq!(r.cost_credits, 20 + 8);
        assert_eq!(r.breakdown.len(), 2);
        let v = quote_variants(2);
        assert_eq!(v.cost_credits, (3 + 4 + 1) * 2);
        assert_eq!(quote_variants(0).cost_credits, 0);
    }

    #[test]
    fn negative_inputs_are_clamped() {
        let q = quote_generation(&GenerationQuoteInput {
            reel_count: -4,
            language_count: 0,
            style: GenerationStyle::Broll,
            needs_extraction: false,
        });
        assert_eq!(q.cost_credits, 0);
        assert_eq!(q.estimated_minutes, 1);
    }
}
