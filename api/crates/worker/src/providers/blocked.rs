//! A provider slot that is switched off: either not set up (the judge is optional) or kept off
//! by the company's AI plan or budget. Every call answers with the reason, which the studio shows
//! the admin, instead of producing placeholder content.

use super::*;

pub struct Blocked {
    message: String,
    /// Optional slots (the judge) are skipped quietly; required ones fail the step.
    optional: bool,
}

impl Blocked {
    pub fn new(message: impl Into<String>) -> Blocked {
        Blocked {
            message: message.into(),
            optional: false,
        }
    }

    pub fn off(message: impl Into<String>) -> Blocked {
        Blocked {
            message: message.into(),
            optional: true,
        }
    }

    fn err<T>(&self) -> Result<T, ProviderError> {
        Err(ProviderError::Config(self.message.clone()))
    }
}

#[async_trait]
impl ScriptWriter for Blocked {
    fn name(&self) -> &'static str {
        "blocked"
    }
    async fn extract(&self, _: ExtractInput) -> Result<Extraction, ProviderError> {
        self.err()
    }
    async fn write_reels(&self, _: ScriptInput) -> Result<Vec<ReelScript>, ProviderError> {
        self.err()
    }
    async fn write_question(&self, _: &str, _: &str) -> Result<QuestionDraft, ProviderError> {
        self.err()
    }
    async fn translate(
        &self,
        _: &str,
        _: Option<&QuestionDraft>,
        _: &str,
        _: &str,
    ) -> Result<(String, Option<QuestionDraft>), ProviderError> {
        self.err()
    }
    async fn find_moments(
        &self,
        _: &[TranscriptSegment],
    ) -> Result<Vec<MomentDraft>, ProviderError> {
        self.err()
    }
}

#[async_trait]
impl Judge for Blocked {
    fn name(&self) -> &'static str {
        if self.optional {
            "off"
        } else {
            "blocked"
        }
    }
    async fn review_draft(
        &self,
        _: &DraftInput,
    ) -> Result<reelearn_domain::ai::DraftAnswers, ProviderError> {
        self.err()
    }
    async fn role_fit(&self, _: &str, _: &[RoleReel]) -> Result<Vec<f32>, ProviderError> {
        self.err()
    }
}
