# Short-video recommenders: published designs and a ranking engine for Reelearn

Research date: 2026-11-09. Audience: the engineer building the Reelearn feed ranker (Rust/axum API, one Postgres 17 database, per-company catalogs of hundreds to a few thousand 30-60 s learning reels).

**Evidence labels.** Every number is tagged:

- **[official]**: a company's own transparency page, newsroom, help centre or engineering blog.
- **[paper]**: a peer-reviewed or arXiv paper by the operator's own researchers.
- **[reported]**: journalism about leaked or unofficial material.
- **[unverified]**: agency or marketing blogs.
- **[our choice]**: no published value exists, so this document proposes a default.

The Medium article "Reverse-Engineering the Reels Algorithm" was the starting point. It is not summarised again here.

---

## (a) Synthesis: what a production reels engine consists of

Every operator that publishes its design uses the same funnel. Only the names differ.

| Stage | What it does | Published evidence | What Reelearn needs |
|---|---|---|---|
| 0. Event logging and real-time features | Logs impressions, watch time, skips, likes, shares and "not interested", then updates counters and embeddings within minutes. | ByteDance Monolith syncs sparse parameters at minute-level intervals [paper]. Instagram Explore fine-tunes models every hour [official]. | One append-only `feed_events` table, plus counters updated in the same transaction. At this scale, real-time comes free. |
| 1. Candidate retrieval | Several sources are unioned: two-tower ANN, items similar to past positives, followed accounts and heuristics. | Explore retrieves hundreds of items from billions [official]. YouTube candidate generation narrows millions of videos to hundreds [paper]. | The catalog is ≤ 5k items, so every eligible item can be scored. Sources still matter as quotas and as "why you're seeing this" reasons. |
| 2. Early ranking | A cheap model, distilled from the late ranker, prunes to about 100 items. | Explore's first-stage two-tower model is trained to predict the second-stage output [official]. Instagram 2019: 500 → 150 → 50 → 25 [official]. | Not needed at our size. |
| 3. Late ranking (multi-task) | Predicts P(action) for 5-15 actions, then combines them with a value model. | Meta: `W_click·P(click) + W_like·P(like) − W_see_less·P(see less)` [official]. YouTube: MMoE with engagement and satisfaction heads, combined by weighted multiplication [paper]. | Smoothed per-item rates plus user-topic affinity, combined in a weighted additive value model that has a learning term. |
| 4. Re-ranking | Integrity filters, diversity (no same author back to back) and freshness or exploration slots. | Explore applies an author penalty that grows with repeats [official]. TikTok avoids same creator or same sound back to back [official]. YouTube uses DPP [paper]. | Greedy MMR plus hard sliding-window rules, with reserved exploration slots. |
| 5. Long-term objective | Optimises retention and satisfaction instead of immediate clicks. | YouTube surveys feed "valued watch time" [official]. Kuaishou's RL rewards short return time [paper]. Meta's UTIS interest survey [official]. | Treat knowledge retention (spaced repetition) and assigned-path completion as first-class value terms. Add a 1-question survey later. |
| 6. Guardrails and controls | "Not interested", topic controls, break reminders, daily limits and a non-personalised feed. | YouTube break reminders are on by default for 13-17 [official]. TikTok sets a 60 min default for under-18s [reported]. The DSA requires a non-profiling option [official, law]. | Admin-configurable caps, a "done for today" stopping cue, an explanation for every item and a non-personalised mode. |

Three lessons apply directly:

1. **Raw watch time is biased by duration.** Every short-video operator that publishes on this normalises completion within duration groups: Kuaishou (D2Q, TPM), the Tsinghua/Kuaishou WTG work, and Meta's "watch more than 95% of users who watched reels of the same length".
2. **Negative signals need large weights**, because their probabilities are small. Twitter's published weights put negative feedback at −74 against +0.5 for a like [official].
3. **Exploration looks neutral or negative in short A/B tests but pays off long term.** Google's short-video exploration paper is the evidence (+~50% discoverable corpus over 3 months) [paper].

---

## (b) Per-source notes

### 1. Meta

#### Instagram Reels Chaining system card [official]

Source: [transparency.meta.com/features/explaining-ranking/ig-reels-chaining](https://transparency.meta.com/features/explaining-ranking/ig-reels-chaining/)

The pipeline is: gather inventory → leverage signals → make predictions → rank by score.

Predictions, each with the signals Meta lists for it:

- **P(use the audio)**: "viewing reels for more than 15 seconds"; audio-link clicks; "Use audio" clicks after an audio-link click, and that count relative to reels seen.
- **P(watch < 3 s)**: how often users watched ≥3 s anywhere and in Explore; dismiss count; views with sound on; "skipped within two seconds of being opened".
- **P(click "Interested")** on the Interested/Not interested prompt: the authors of reels you watched < 3 s; like count; your saves; your reshares and their authors; profile clicks.
- **P(comment)**: your past commented media and authors, including "authors you've commented on in the past hour"; time viewers from your country spent on the reel; your total comments.
- **P(watch more than 95% of users who watched reels of the same length)**: the reel's length; total time people spent viewing it. This is a duration-normalised completion label.
- **P(tap a tall reel in Explore to open full screen)**.
- **P(reshare)**, **P(share off Instagram)** and **P(follow author)**. Follow uses "followers the author received through the reel".

User controls are Not interested, Report, Save, Share, Remix, "See newest reels first" (a non-personalised order) and the sensitive-content setting.

Takeaways for Reelearn:

- **Fast skip has an explicit definition**: < 3 s watched, or skipped within 2 s.
- **Short-window counters** such as "past hour" are first-class features.

#### Facebook Reels system card [official]

Source: [transparency.meta.com/features/explaining-ranking/fb-reels](https://transparency.meta.com/features/explaining-ranking/fb-reels/)

Inventory comes from followed accounts, reels similar to ones you engaged with, and accounts similar to those. A lightweight model keeps about 10-100 reels, then integrity demotions are applied.

The card lists three recommenders. Their predictions include:

- **Watch-time predictions**: expected watch time; P(complete); "how much of a reel you're likely to watch".
- **Navigation predictions**: P(keep watching after opening full screen); P(start a viewing session rather than leaving right away); P(move on to the next reel).
- **Positive engagement**: P(like); P(share to WhatsApp or Messenger) and the number of people shared with.
- **Negative feedback**: P(skip); P(skip or hide a reel *and leave*); P(hide); P(ask to see less); P(ask to see more).

Recurring signals:

- **Item-level**: completions by others; views; "how often the reel has been skipped by others"; reel length.
- **User-level**: your recent likes, watches, shares and comments; "how much the reel's topic matches topics you tend to watch"; authors you watched.

Takeaway: the session-level labels ("skip and leave" and "start a session") are the operator-scale version of the retention objective.

#### Instagram Explore

**2019, "Powered by AI: Instagram's Explore recommender system"** [official] ([ai.meta.com](https://ai.meta.com/blog/powered-by-ai-instagrams-explore-recommender-system/)):

- **Seed accounts.** Candidates come from accounts the user liked or saved, expanded by ig2vec. ig2vec is word2vec over sequences of account IDs, with FAISS KNN.
- **Funnel.** 500 candidates are sampled per request. A distilled model keeps 150, a light NN keeps 50 and a deep NN picks 25.
- **Value model.** `w_like·P(Like) + w_save·P(Save) − w_negative_action·P(Negative Action)`.
- **Diversity.** A heuristic penalty "downranks posts from the same author or the same seed account". The penalty grows with each repeat further down the batch.
- **Example config values.** `liked(max_num_to_retrieve=30)`, `posted_media(max_media_per_account=10)`, `non_recommendable_model_threshold=0.2` and round-robin diversification.

**2023, "Scaling the Instagram Explore recommendations system"** [official] ([engineering.fb.com](https://engineering.fb.com/2023/08/09/ml-applications/scaling-instagram-explore-recommendations-system/)):

- **Retrieval** uses two-tower models (item embeddings precomputed daily, user tower online, ANN) plus "user interaction history" retrieval. That second source finds items similar to what the user liked, saved or shared, after rule-based quality filtering.
- **First-stage ranking** is a two-tower model distilled on the second stage's top-K. **Second-stage ranking** is an MTML network over the top 100.
- **Value model**: `Expected Value = W_click·P(click) + W_like·P(like) − W_see_less·P(see less) + …`.
- **Re-ranking** applies integrity filters and "not showing items from the same author in a row".
- **Training cadence**: models are fine-tuned hourly. Bayesian optimisation of the value-model weights can take more than a month online, and offline tuning cuts that to hours.

#### Facebook Reels "User True Interest Survey" (UTIS), 2026 [official]

Source: [engineering.fb.com, 2026-01-14](https://engineering.fb.com/2026/01/14/ml-applications/adapting-the-facebook-reels-recsys-ai-model-based-on-user-feedback/)

- **Survey.** The question is "To what extent does this video match your interests?" on a 1-5 scale. It is shown to a random subset of sessions daily, and responses are weighted for sampling and nonresponse bias, then binarised.
- **Model.** UTIS runs alongside the late ranker and feeds the value formula as an input. It gives a modest boost for high predicted interest and a demotion for low.
- **Why it was needed.** Heuristic "interest" signals had 48.3% precision. UTIS reached 63.2% precision, with recall rising from 45.4% to 66.1%.
- **Online A/B (>10M users).** High ratings +5.4%, low ratings −6.84%, engagement +5.2% and integrity violations −0.34%.

Takeaway: engagement-derived interest is about half wrong. A tiny randomised survey is a cheap correction.

### 2. TikTok / ByteDance

#### "How TikTok recommends videos #ForYou" (2020) [official]

Source: [newsroom.tiktok.com](https://newsroom.tiktok.com/en-us/how-tiktok-recommends-videos-for-you)

Signals:

- **User interactions**: likes, shares, follows, comments, replays, content created and **finishing videos**.
- **Video information**: captions, sounds and hashtags.
- **Device and account settings**: language, country and device. These carry **lower weight**.
- **Explicit negatives**: "Not interested", hiding a creator or sound, and reports.

Weighting and eligibility:

- **Strong vs weak indicators.** Strong indicators outweigh weak ones. TikTok's example: finishing a longer video is strong, while sharing a country with the creator is weak.
- **Follower count is not a direct factor.** Neither is an account's past high-performing videos.
- **No numeric weights are published.**
- **Not shown**: duplicates, already-seen content and spam.
- **Ineligible**: just-uploaded content and content under review.

Diversity:

- The feed generally won't show **two videos in a row from the same creator or with the same sound**.
- It intentionally inserts videos outside the user's expressed interests.

New users pick interest categories. Users who skip that step get a generalised feed of popular videos.

#### "An update on our work to safeguard and diversify recommendations" (Dec 2021) [official]

Source: [newsroom.tiktok.com](https://newsroom.tiktok.com/en-us/an-update-on-our-work-to-safeguard-and-diversify-recommendations)

- **Clusters can harm.** TikTok tests ways to avoid "a series of similar content", such as extreme dieting, sadness or breakups. A single video can be fine on its own but harmful in a cluster.
- **"Not interested"** suppresses future videos from that creator or with that audio.
- **Keyword filters** let users block words or hashtags.

Takeaway: diversity is enforced over a sliding window, not only on adjacent pairs.

#### Monolith (ByteDance, ORSUM@RecSys 2022) [paper]

Source: [arXiv 2209.07663](https://arxiv.org/abs/2209.07663)

**Collisionless embedding table.** IDs are stored in a Cuckoo hashmap, which gives worst-case O(1) lookup, and two policies keep the table small:

- **Admission filter.** An ID is inserted only after a minimum occurrence count, which is a tunable per-model threshold. A probabilistic filter is added on top.
- **Expiry.** Embeddings expire after a tunable per-table inactivity period.

**Online training data flow.** Two Kafka queues carry user actions and features. A Flink online joiner joins them, using an on-disk KV store for late actions, and negative sampling is corrected by **log-odds correction** at serving time.

**Parameter sync.**

- The serving parameter server receives **only touched sparse keys at minute-level intervals**. Dense parameters sync less often, with "no conspicuous loss".
- On the Criteo dataset, average AUC was 79.66 with a 5 h sync interval, 79.78 with 1 h and 79.80 with 30 min. Batch training stayed around 79.42-79.44.
- A live A/B test on an ads model reported a **14.0-18.1% daily AUC improvement** from online training over batch training (Table 3).

**Fault tolerance.** Parameter servers are snapshotted daily. Losing a day of updates on one shard was tolerable at a 0.01% daily failure rate.

Takeaway: freshness beats completeness. Our equivalent is to update counters and affinities synchronously on event ingest.

#### "TikTok Algo 101" (NYT, 5 Dec 2021) [reported]

Source: [nytimes.com](https://www.nytimes.com/2021/12/05/business/media/tiktok-algorithm.html)

The NYT described an internal document that gives the scoring as `Plike×Vlike + Pcomment×Vcomment + Eplaytime×Vplaytime + Pplay×Vplay`. The P terms are predicted probabilities, E is expected playtime and the V terms are value weights. The report also says the document named retention and time spent as the ultimate goals.

This is **not official**. Its structure matches Meta's published value model, a weighted sum of predicted actions.

### 3. YouTube / Google

#### Covington et al., "Deep Neural Networks for YouTube Recommendations" (RecSys 2016) [paper]

Source: [research.google PDF](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45530.pdf)

**Candidate generation.**

- Generation is extreme multiclass classification over millions of videos. Several thousand negatives are sampled, with importance weighting.
- Inputs are averaged embeddings of the **50 most recent watches and 50 recent searches**, plus an **"example age"** feature that removes the past bias of training data.
- Serving uses ANN to return hundreds of candidates.
- Training uses a **fixed number of examples per user**, so heavy users don't dominate.

**Ranking.**

- The ranker uses hundreds of features. Impression-frequency features create "churn", so successive requests differ.
- **Expected watch time** comes from weighted logistic regression. Positive (clicked) impressions are weighted by watch time T_i and negatives get unit weight. The learned odds ≈ ΣT_i/(N−k) ≈ E[T](1+P) ≈ E[T], and serving uses `e^(Wx+b)`.
- The best tested network (1024→512→256 ReLU) reached a 34.6% "weighted per-user loss".

#### Zhao et al., "Recommending What Video to Watch Next: A Multitask Ranking System" (RecSys 2019) [paper]

Source: [ACM DOI 10.1145/3298689.3346997](https://dl.acm.org/doi/10.1145/3298689.3346997)

- **Objectives come in two groups.** Engagement covers clicks and time watched. Satisfaction covers likes, dismissals and ratings.
- **Loss and architecture.** Binary tasks use cross-entropy and time or rating tasks use squared loss. An MMoE layer is used, tested with 4 or 8 experts.
- **Combination.** Predictions are combined "using a combination function in the form of **weighted multiplication**", with weights **manually tuned**.
- **Position bias.** A **shallow tower** takes position and device as inputs, with **10% feature dropout** on position, and its logit is added to the main logit. At serving, the position feature is dropped.

#### "On YouTube's recommendation system" (Goodrow, 2021) [official]

Source: [blog.youtube](https://blog.youtube/inside-youtube/on-youtubes-recommendation-system/)

- **Signals.** Clicks, watch time (added in 2012), survey responses, shares, likes and dislikes. The system draws on more than 80 billion signals.
- **Surveys.** Viewers rate videos 1-5 stars. Only videos rated 4-5 count toward **"valued watchtime"**. A model predicts survey responses for everyone, and some responses are held out to test it.
- **Numbers.** Adding watch time first cut views by 20%. Demoting tabloid-style content raised watch time 0.5% over 2.5 months. Demoting borderline content gave −70% watch time on that content, and borderline content is now "well under 1%" of recommendations.

#### Wilhelm et al., "Practical Diversified Recommendations on YouTube with DPPs" (CIKM 2018) [paper]

Source: [ACM DOI 10.1145/3269206.3272018](https://dl.acm.org/doi/10.1145/3269206.3272018)

- **Method.** The YouTube homepage feed is re-ranked with a determinantal point process.
- **Kernel.** `L_ii = q_i²` and `L_ij = α·q_i·q_j·exp(−D_ij / 2σ²)`. Here q is the ranker's quality score, D_ij is an embedding distance, and α and σ are tuned. Selection is greedy over a sliding window of items.
- **Result.** Live tests showed short- and long-term engagement gains.
- **Fast greedy inference.** Chen et al. (Hulu, NeurIPS 2018) give a fast greedy MAP algorithm for this kernel: [arXiv 1709.05135](https://arxiv.org/abs/1709.05135) [paper].

The kernel form is as published in the paper. No α or σ values are published.

#### Su et al., "Long-Term Value of Exploration" (WSDM 2024) [paper]

Source: [arXiv 2305.07764](https://arxiv.org/abs/2305.07764)

The study ran on "one of the largest short-form video platforms". Its treatment **reserved dedicated slots for fresh and tail content**. Fresh means less than X days old, and tail means fewer than Y lifetime positive interactions.

- Discoverable Corpus @X, 3-month rose by about **+50%** across buckets (+48% to +119%).
- Regular user-level A/B tests of exploration "often show neutral or even negative" engagement.
- A Neural Linear Bandit gave +1.25% unique topics a user engages with, and +5.33% / +5.66% Discoverable Corpus @100 / @1000.

### 4. Kuaishou and duration debiasing

#### D2Q (Zhan et al., KDD 2022) [paper]

Source: [arXiv 2206.06003](https://arxiv.org/abs/2206.06003)

- **The bias.** Duration is a confounder: it raises exposure, which is bias, and it raises watch time, which is genuine signal.
- **Method.** Split data into **M equal-frequency duration groups**, compute each group's empirical watch-time CDF, and regress the **watch-time quantile within the group**. One shared model is used, with duration as an input. At serving, the predicted quantile is mapped back to watch time through the group's CDF.
- **Group count.** Performance peaked at **30 groups**, out of 1, 10, 20, 30, 50 and 100 tested.
- **Live result.** On 5% of Kuaishou users for 24 h, Res-D2Q gained **+0.746% watch time** over value regression. The paper notes "the average watch-time improvement from production algorithms is around 0.1%".

#### TPM (Lin et al., KDD 2023) [paper]

Source: [arXiv 2306.03392](https://arxiv.org/abs/2306.03392)

- **Method.** Watch time is treated as ordinal and split into intervals. A tree of conditional binary classifiers is built over those intervals, and the variance of the prediction is added to the loss. Backdoor adjustment removes duration bias.
- **Result.** More than 0.2% watch-time improvement at Kuaishou (over 300M DAU).

#### WTG / DVR (Zheng et al., ACM MM 2022) [paper]

Source: [arXiv 2208.05190](https://arxiv.org/abs/2208.05190)

**Watch Time Gain** is defined as `WTG = (WT − μ_B(v)) / σ_B(v)`. Here μ_B and σ_B are the mean and standard deviation of watch time over all users in video v's duration bin, and both can be maintained recursively.

Ranking by raw watch time gave short-video creators less than 20% of traffic.

#### D2Co (Zhao et al., RecSys 2023) [paper]

Source: [arXiv 2308.08120](https://arxiv.org/abs/2308.08120)

- **Model.** Per duration d, a two-component GMM splits watch time into a "duration-biased" level w⁺_d and a "noisy watching" level w⁻_d, smoothed with a frequency-weighted moving average.
- **Label.** `r = (exp(αw) − exp(αw⁻_d)) / (exp(αw⁺_d) − exp(αw⁻_d))`.
- **Baselines.** These include Play Completion Rate (PCR = watch time / duration). PCR is unbiased only under strong assumptions.

Takeaway: a fast skip on a 60 s reel and one on a 30 s reel are both "noise", and the label should treat them so.

#### RLUR, "Reinforcing User Retention in a Billion Scale Short Video Recommender System" (Cai et al., WWW 2023) [paper]

Source: [arXiv 2302.01724](https://arxiv.org/abs/2302.01724)

- **Formulation.** The final score is linear, `f(a, x_j) = Σ_k a_k·x_jk`, over n model predictions, and the top 6 items are shown per request. An RL policy outputs the weight vector a per request.
- **Rewards.** The immediate reward is watch time plus interaction count. The delayed reward is **returning time**, the gap between sessions, which the policy minimises with discount γ.
- **Result vs CEM.** App-open frequency rose +0.450%, DAU +0.2%, day-1 retention +0.053% and day-7 retention +0.063%. The authors note that "+0.01% retention and +0.1% DAU are statistically significant" at their scale.

Takeaway: even the most sophisticated operators keep a **linear value model** and only learn its weights. We can tune weights per company by hand or A/B test, and RL is out of scope at our traffic.

### 5. Exploration and cold start

- **Yahoo LinUCB** (Li et al., WWW 2010, [arXiv 1003.0146](https://arxiv.org/abs/1003.0146)) [paper]: a contextual UCB bandit in production for news recommendation, scoring `θᵀx + α·sqrt(xᵀA⁻¹x)`.
- **Thompson sampling** (Chapelle & Li, NeurIPS 2011, [paper](https://papers.nips.cc/paper/2011/hash/e53a0a2978c28872a4505bdb51db06dc-Abstract.html)) [paper]: Beta-Bernoulli TS matched or beat UCB on Yahoo ad and news data, and is robust to delayed feedback.
- **Dedicated exploration slots for fresh and tail items**: Google (above) [paper].
- **New-user priors**: TikTok's interest picker, with a popular-feed fallback [official].
- **TikTok's freshness exclusion**: just-uploaded or under-review content is ineligible [official], so freshness boosts apply only after review.
- **ε-greedy in production**: Spotify "Explore, Exploit, Explain" (McInerney et al., RecSys 2018, [ACM](https://dl.acm.org/doi/10.1145/3240323.3240354)) [paper]. Spotify uses ε-greedy exploration on Home shelves to gather unbiased training data, and pairs it with explanations.
- **No operator publishes its ε, slot share or new-item impression guarantee.**

### 6. Diversity re-ranking

- **MMR** (Carbonell & Goldstein, SIGIR 1998, [ACM](https://dl.acm.org/doi/10.1145/290941.291025)) [paper]: `argmax_i [λ·rel(i) − (1−λ)·max_{j∈S} sim(i,j)]`. It is simple, greedy, and the right default for under 5k items.
- **DPP**: YouTube 2018 (above) [paper].
- **Rule-based windows**: no same author in a row (Instagram 2023, TikTok) [official]. An escalating same-author penalty (Instagram 2019) [official]. Avoiding clusters of similar sensitive content (TikTok 2021) [official].

### 7. Negative feedback

**Explicit signals.**

- Meta: "not interested", "see less", hide, P(skip or hide and leave) [official].
- TikTok: "Not interested", hide creator or sound, report [official].
- YouTube: dismissals and dislikes [paper, official].

**Implicit signals.**

- Meta's fast-skip definitions are watched < 3 s and skipped within 2 s [official].
- Facebook Reels uses "how often the reel has been skipped by others" as an item feature [official].
- D2Co treats short watches as noisy watching [paper].

**Weighting.** The only operator to publish real value-model weights is **Twitter/X** (2023-04-05, [the-algorithm-ml README](https://github.com/twitter/the-algorithm-ml/blob/main/projects/home/recap/README.md)) [official]:

| Action | Weight |
|---|---|
| fav | 0.5 |
| retweet | 1.0 |
| reply | 13.5 |
| good profile click | 12.0 |
| good click | 11.0 |
| reply engaged by author | 75.0 |
| video playback 50% | 0.005 |
| **negative feedback** | **−74.0** |
| **report** | **−369.0** |

Negative feedback is therefore about 148× a like, and a report about 738× a like. The weights multiply *probabilities*, so rare negatives need big weights to matter.

### 8. Real-time interest and drift

- **Short-window counters as features.** Meta uses "authors you've commented on in the past hour" [official].
- **Hourly model fine-tuning** at Instagram Explore [official].
- **Minute-level sparse sync and embedding expiry** in Monolith [paper].
- **Example age** in YouTube 2016 removes staleness bias [paper].
- **No published decay half-life** exists for user-interest counters at any operator, so ours below is **[our choice]**.

### 9. Learning-specific: spacing, difficulty and knowledge tracing

**Half-Life Regression** (Settles & Meeder, ACL 2016, [P16-1174](https://aclanthology.org/P16-1174.pdf)) [paper]:

- **Model.** Recall probability is `p = 2^(−Δ/h)` and half-life is `ĥ = 2^(Θ·x)`. Δ is the time since last practice, and x holds √-counts of total, correct and incorrect exposures plus lexeme tags.
- **Loss.** `(p − p̂)² + α(h − ĥ)² + λ‖Θ‖²`, with λ = 0.1, α = 0.01 and learning rate 0.001.
- **Relation to Leitner.** Leitner and Pimsleur are special cases with fixed weights, so Leitner box-doubling is a valid cold-start for HLR.
- **Results.** Mean absolute error fell ≥45% vs Leitner. Daily retention for lesson practice rose +12.0% (HLR without lexeme features vs HLR), and HLR was deployed to all students.

**Duolingo Birdbrain** ([blog](https://blog.duolingo.com/learning-how-to-help-you-learn-introducing-birdbrain/)) [official]:

- It jointly estimates learner ability and exercise difficulty to predict P(correct). The Session Generator then picks exercises at "just the right difficulty level".
- It went from about 5% of lessons to over 20% in 2020.
- **No target success rate or update rule is published.**

**Desirable difficulty.** Wilson et al., "The Eighty Five Percent Rule for optimal learning" (Nature Communications 2019, [link](https://www.nature.com/articles/s41467-019-12552-4)) [paper]. For gradient-learning in binary classification, the optimal training error is about **15.87%**, which is about 85% accuracy. This is a theoretical result for learning algorithms; transferring it to humans is an assumption.

**Knowledge tracing.**

- **BKT** (Corbett & Anderson 1995, [DOI](https://doi.org/10.1007/BF01099821)) [paper]: a hidden mastery state per skill with parameters P(L0), P(T) (learn), P(G) (guess) and P(S) (slip). The update is Bayesian on each quiz answer.
- **DKT** (Piech et al., NeurIPS 2015, [arXiv 1506.05908](https://arxiv.org/abs/1506.05908)) [paper]: an RNN over answer sequences. It needs far more data than a single company will have.

**Choice for Reelearn.** Use BKT per skill or topic and HLR per reel or concept. Both fit in a few Postgres columns.

### 10. Responsible design and regulation

**EU AI Act, Art. 5** ([artificialintelligenceact.eu/article/5](https://artificialintelligenceact.eu/article/5/)) [official, law; applies since 2 Feb 2025]. It prohibits AI that:

- "deploys subliminal techniques … or purposefully manipulative or deceptive techniques" causing significant harm (5(1)(a));
- exploits vulnerabilities due to age, disability or social or economic situation (5(1)(b));
- infers **emotions in the workplace** or education (5(1)(f)), except for medical or safety reasons.

Implications for Reelearn: never infer emotion from camera, voice or facial data, and do not design dark patterns such as fake urgency or variable-ratio streak traps.

**EU DSA** ([Regulation 2022/2065](https://eur-lex.europa.eu/eli/reg/2022/2065/oj)) [official, law]:

- **Art. 27** requires online platforms to state the "main parameters" of their recommender systems in their terms, including the most significant criteria and why they matter. Where several options exist, users must be able to select and modify their option at any time.
- **Art. 38** requires very large platforms to offer at least one option "not based on profiling".

An internal, employer-provided learning app is likely not a DSA "online platform". Treat these as **best practice**: an explanation per item, plus a "standard order" (non-personalised) mode.

**Saudi PDPL** (Royal Decree M/19, 2021, amended 2023; in force 14 Sept 2023; SDAIA regulator; [SDAIA](https://sdaia.gov.sa/en/SDAIA/about/Pages/RegulationsAndPolicies.aspx)) [official; English summaries [unverified], e.g. [Clyde & Co](https://www.clydeco.com/en/insights/2021/09/saudi-arabia-issues-personal-data-protection-law)]:

- **Purpose limitation.** Collection must be "directly related" to the controller's purposes. Data must not be reused for a new, unrelated purpose, and collection must be minimised.

Implications for Reelearn:

- Feed telemetry collected "to personalise learning" must **not** be repurposed for performance evaluation or discipline without a separate lawful basis and notice.
- Admin views should show aggregates. A minimum group size before team-level reports is **[our choice]**: 5.
- Check the Arabic text on laws.boe.gov.sa for exact wording.

**Wellbeing features shipped by operators:**

- **YouTube**: a "take a break" reminder that pauses playback. It is **on by default for ages 13-17** and off for adults ([help](https://support.google.com/youtube/answer/9012523)) [official].
- **TikTok**: a 60-minute default daily limit for under-18 accounts, with a passcode to continue, announced 1 Mar 2023 [reported, e.g. [WABE/AP](https://www.wabe.org/tiktok-sets-new-default-time-limits-for-minors/)]. Teens over 100 min/day are prompted to set a limit. User-selectable break reminders at 10, 20 or 30 min [reported].
- **Instagram**: "Take a Break" reminders [reported, [Bloomberg 2021](https://www.bloomberg.com/news/articles/2021-11-10/instagram-tests-take-a-break-tool-to-let-users-self-regulate)].

---

## (c) Recommended design for a small-catalog learning feed

### Design principles

1. **Score everything.** With at most about 5k items per company, the Rust API loads the company's eligible items into memory and caches them in Redis. It scores all of them per request; no ANN or early ranker is needed. Candidate sources become **features and quotas**, not recall gates.
2. **Use a linear, explainable value model** with named terms, as Meta, TikTok (reported), Twitter and Kuaishou do. Every term maps to a "why you're seeing this" string.
3. **Debias completion by duration** (D2Q/WTG style).
4. **Treat learning value as a first-class term.** That means due reviews (HLR), next-in-path items and difficulty near 85% expected success.
5. **Give negatives large weights and permanent effects**, and reserve exploration slots.
6. **Ship guardrails on by default.**

### Candidate sources (all are tags on the item for this request)

| Source tag | Rule | Quota per 10-item page [our choice] |
|---|---|---|
| `assigned` | Admin-assigned and not yet completed; urgency rises as the due date nears | ≤ 3 |
| `review_due` | Previously completed reel whose HLR recall p ≤ `review_threshold` | ≤ 2 |
| `next_in_path` | Next unwatched reel in a learning path or series the user started | ≤ 2 |
| `affinity` | Topic in the user's top-k affinities | unbounded |
| `similar_positive` | Embedding cosine ≥ 0.75 to a reel liked, saved or completed in the last 7 days (pgvector) | unbounded |
| `fresh` | Published ≤ 14 days ago and passed review | via exploration slots |
| `popular_org` | Top decayed completion rate in the company or department | fallback |
| `explore` | Topics with low user exposure, or items with < `min_explore_impressions` | `explore_rate` |

### Signals and labels (per impression, logged in `feed_events`)

| Signal | Field | Note |
|---|---|---|
| Watch time and duration | `watch_ms`, `duration_ms` | `wr = min(watch_ms/duration_ms, 2.0)`; loops count up to 2× |
| Fast skip | `watch_ms < 2000` or `wr < 0.10` | Meta uses < 3 s or a skip within 2 s for general reels [official]. For 30-60 s reels, 2 s / 10% is [our choice] |
| Completion | `wr ≥ 0.9` | [our choice] |
| Like, save, share | booleans | |
| Not interested, report | booleans | Explicit negatives |
| Quiz answered and correct | booleans | Learning signal; also drives HLR and BKT |
| Session exit after item | derived | "Skip and leave" (FB Reels card) [official] |

### Duration-normalised completion (Ĉ)

Duration buckets are per company, at 5 s width from 30 to 60 s (6 buckets). This is **[our choice]**: D2Q's 30 equal-frequency groups [paper] assume Kuaishou-scale data. Fall back to global buckets when a bucket has fewer than 200 events [our choice].

For each bucket B, keep the empirical CDF of `wr` as a 21-point histogram that is refreshed nightly.

- **Training label** (D2Q-style): `c = CDF_B(wr)`, which lies in [0,1].
- **Item feature**: `Ĉ_i = (Σ c + m·0.5) / (n + m)`, a Bayesian-smoothed mean with prior strength m = 20 [our choice].
- **User-item estimate**: `Ĉ(u,i) = σ( logit(Ĉ_i) + β_c·aff(u, topic_i) )`, with β_c = 1.0 [our choice].
- **Alternative**: WTG z-score [paper] if you prefer unbounded values. Squash it with σ before use in the value model.

### Action probabilities

For each action a in {like, save, share, fastskip, notinterested, quiz_correct}:

```
r̄_a(i)   = (k_a(i) + m·μ_a(org)) / (n(i) + m)                         # Beta-smoothed item rate, m = 20 [our choice]
P̂_a(u,i) = σ( logit(r̄_a(i)) + β_a·aff(u,topic_i) + γ_a·prop_a(u) )   # prop_a = user's own smoothed rate, logit-centred
```

Start with β_a = 1 and γ_a = 1 (for negatives use β = −1 on affinity) [our choice]. Once a company has at least about 50k impressions [our choice], fit β and γ nightly by logistic regression, in the spirit of YouTube's weighted LR [paper].

### Learning terms

**Review urgency (HLR).** Per (user, reel):

```
p = 2^(−Δ/h)
R = clip((τ − p)/τ, 0, 1)
```

The review threshold is τ = 0.9 [our choice]. HLR is cold-started as Leitner [paper]:

- start h = 1 day;
- correct quiz → h ← 2h;
- wrong quiz → h ← max(h/2, 1 day);
- completion without a quiz → h ← 1.25h [our choice].

Fit HLR's Θ once there are more than 10k quiz events.

**Difficulty fit.** Use a Rasch/Elo estimate `P_ok = σ(θ_u − b_i)`, with K = 0.1 updates per answer [our choice; Birdbrain publishes no rule]. Then `D = exp(−(P_ok − 0.85)² / (2·0.12²))`. The 0.85 target comes from Wilson et al. [paper]; the width is [our choice].

**Mastery gate (BKT).** Use topic mastery `P(L)` with P(L0) = 0.2, P(T) = 0.15, P(G) = 0.2 and P(S) = 0.1 [our choice; standard textbook-range starting values, not from a production source]. When P(L) > 0.95, new items in that topic get `M = 1` (already mastered) unless they are review items.

**Path progress.** `N = 1` if the item is `next_in_path`, else 0.

**Assignment urgency.** `A = 1` if the item is assigned and due within 3 days, `0.6` if due within 14 days, `0.3` otherwise; `A = 0` if not assigned [our choice].

### The value model

```
V(u,i) =  w_comp  · Ĉ(u,i)
        + w_like  · P̂_like + w_save · P̂_save + w_share · P̂_share
        + w_learn · ( R + 0.5·D + 0.5·N )
        + w_assign· A
        + w_fresh · F                       # F = 1 for fresh items, decaying linearly to 0 by day 14
        − w_skip  · P̂_fastskip
        − w_ni    · P̂_notinterested
        − w_mast  · M
        − w_seen  · S                       # S = 1 if seen in the last 7 days and not a due review; 0.5 if 7-30 days
```

Use the additive form, as Meta does [official]. YouTube's weighted multiplication [paper] is an alternative once the weights are learned. All terms are in [0,1], so weights are directly comparable.

Hard filters (not scored):

- reported or "not interested" items for this user;
- items under review or unpublished;
- items outside the user's audience rules.

Default weights [our choice]. The negative-to-positive ratio follows Twitter's lesson [official]: rare negatives get weights an order of magnitude above a like.

| Weight | Default | | Weight | Default |
|---|---|---|---|---|
| w_comp | 1.0 | | w_skip | 1.5 |
| w_like | 0.5 | | w_ni | 8.0 |
| w_save | 1.0 | | w_mast | 1.0 |
| w_share | 1.0 | | w_seen | 2.0 |
| w_learn | 1.5 | | w_fresh | 0.3 |
| w_assign | 2.0 | | | |

### Affinity update rule (long and short term)

Keep a per (user, topic) and per (user, creator) score with **lazy exponential decay**. Store `(score, updated_at)` and decay on read or write:

```
score ← score · 2^(−(now − updated_at)/H) + r_event
aff_long(u,t)  : H = 14 days      [our choice, no published value]
aff_sess(u,t)  : H = 30 minutes   [our choice]  (reset if session gap > 30 min)
aff(u,t) = tanh( 0.7·aff_long/ s_long + 0.3·aff_sess / s_sess )   # s = per-user 90th-percentile score, for scale
```

Event rewards `r_event` [our choice]:

| Event | Reward |
|---|---|
| completion (wr ≥ 0.9) | +1.0 |
| like | +1.0 |
| save | +2.0 |
| share | +2.0 |
| quiz correct | +1.0 |
| quiz wrong | +0.5 (still interest; HLR handles the difficulty) |
| loop (wr > 1.2) | +0.5 |
| fast skip | −0.5 |
| not interested | −5.0 on the topic and −5.0 on the creator |
| report | −10.0 |

Update the row in the same transaction that inserts the event. This is our Monolith-style freshness: a feed request made 1 s later already sees it.

### Exploration

- **Slots.** Reserve `explore_rate` = 10% of slots (1 per 10-item page; position chosen uniformly from 3-9) for exploration [our choice]. Google reserves dedicated slots but publishes no share [paper].
- **Choosing the slot item.** Use Thompson sampling: for each fresh, tail or low-exposure-topic item, draw from `Beta(1 + completions_c, 1 + (n − completions_c))`, where `completions_c` counts duration-normalised c ≥ 0.5. Show the highest draw [paper: Chapelle & Li].
- **New-item guarantee.** Keep an item in the exploration pool until it has `min_explore_impressions` = 30 impressions across the company [our choice].
- **New users.** Use an onboarding topic picker, as TikTok does [official], plus a department/role prior. Seed `aff_long` with +2.0 for each picked topic and +1.0 for the department's top 3 topics [our choice]. For the first 20 impressions, raise `explore_rate` to 25% [our choice].
- **Measurement.** Measure exploration with long-horizon metrics such as distinct topics completed per user over 30 days and the share of catalog with ≥ 10 completions. Do not use 1-week engagement, because Google found short A/B tests understate exploration [paper].

### Diversity re-rank

Apply a greedy fill over the V-sorted list:

```
pick next i maximising  λ·V̂(i) − (1−λ)·max_{j in last W picks} sim(i,j)
   V̂ = V min-max normalised over the candidate set
   sim = cosine of reel embeddings (fallback: Jaccard of topic tags)
   λ = 0.7, W = 5   [our choice]
```

Hard constraints apply before scoring. An item that violates one is skipped for this position:

- no same creator in adjacent positions (TikTok, Instagram) [official];
- at most 2 items from the same topic in any window of 5 [our choice, after TikTok's cluster concern];
- at most 1 part of the same series per page, unless the user is in that series' `next_in_path`.

DPP [paper] is a drop-in upgrade if MMR proves too greedy.

### Negative-feedback handling summary

| Signal | Immediate effect | Lasting effect |
|---|---|---|
| Not interested | Hide the item for this user permanently | Topic −5 and creator −5 (decaying); P̂_ni rises for similar items |
| Report | Hide the item; queue it for admin review | Topic −10; the item is excluded company-wide if reports ≥ 3 [our choice] |
| Fast skip | Session affinity −0.5 | Item `r̄_fastskip` rises |
| 3 fast skips in a row | Next page draws 2 extra items from the user's top-affinity topics and skips the exploration slot [our choice] | none |

### Admin knobs (per company)

| Knob | Min | Default | Max | Source of default |
|---|---|---|---|---|
| `w_assign` (assigned-content priority) | 0 | 2.0 | 5.0 | our choice |
| `w_learn` (review/learning priority) | 0 | 1.5 | 4.0 | our choice |
| `w_comp` | 0.2 | 1.0 | 2.0 | our choice |
| `w_ni` (not-interested penalty) | 2.0 | 8.0 | 20.0 | our choice (Twitter ratio as guide) |
| `explore_rate` | 0.0 | 0.10 | 0.30 | our choice |
| `min_explore_impressions` | 10 | 30 | 100 | our choice |
| `affinity_half_life_days` | 3 | 14 | 60 | our choice |
| `review_threshold` τ (HLR) | 0.7 | 0.9 | 0.95 | our choice |
| `target_success` (difficulty) | 0.70 | 0.85 | 0.95 | Wilson et al. [paper] |
| `diversity_lambda` λ | 0.5 | 0.7 | 1.0 | our choice |
| `max_same_topic_in_5` | 1 | 2 | 5 | our choice |
| `fresh_boost_days` | 0 | 14 | 30 | our choice |
| `daily_reel_cap` (soft stop, "done for today") | 5 | 20 | 100 | our choice |
| `break_reminder_minutes` | off | 20 | 60 | our choice (operators offer 10-30) [reported] |
| `quiet_hours` (no push or feed nudges) | – | 20:00-07:00 local | – | our choice |
| `personalisation_enabled` (else "standard order") | – | on, user may turn off | – | DSA Art. 27/38 best practice |
| `min_group_size_for_reports` | 3 | 5 | 20 | our choice (PDPL minimisation) |

Admins must **not** be able to set negative weights to zero or turn off "not interested" or "report". Those are user rights, not tuning knobs.

### Guardrails (non-tunable)

- **No streak loss-aversion mechanics.** No variable-reward animations or emotion inference (AI Act Art. 5(1)(a) and (f)).
- **A stopping cue.** The feed ends after `daily_reel_cap` with "You're done for today", plus an optional "keep learning" link, instead of infinite scroll.
- **"Why am I seeing this"** on every item, built from the top 2 contributing value terms and the source tag.
- **Purpose-bound telemetry.** Feed events are used for personalisation only. Completion of *assigned* content is the only individual-level data surfaced to managers, and that must be stated in the employee notice (PDPL purpose limitation).

### Postgres and Rust sketch

- **Tables.**
  - `feed_events` (partitioned monthly, append-only).
  - `user_topic_affinity` and `user_creator_affinity`: `(user_id, key_id, score_long, score_sess, updated_at, sess_updated_at)`.
  - `item_stats`: per item, counts by action and `sum_c`.
  - `duration_bucket_cdf`: per company and bucket, a 21-float array.
  - `user_item_memory`: `h_days`, `last_seen_at`, `last_quiz_correct`.
  - `user_skill_bkt`: `p_mastery`.
  - `ranking_config`: per company, the knobs as JSONB.
- **Request path.**
  1. Load the company item cache.
  2. Load the user's affinity rows (≤ a few hundred), memory rows and BKT rows.
  3. Score all items.
  4. Run the MMR fill and insert exploration slots.
  5. Return 10 items, each with `reason`.
  6. Log impressions with the full score breakdown, for offline weight tuning (counterfactual replay).
- **Nightly job (the existing worker).** Rebuild duration CDFs. Refit β and γ when data is sufficient. Recompute company priors μ_a.

---

## (d) Sources

Official (operator):

- Meta, Instagram Reels Chaining system card: https://transparency.meta.com/features/explaining-ranking/ig-reels-chaining/
- Meta, Facebook Reels system card: https://transparency.meta.com/features/explaining-ranking/fb-reels/
- Meta AI, "Powered by AI: Instagram's Explore recommender system" (2019): https://ai.meta.com/blog/powered-by-ai-instagrams-explore-recommender-system/
- Meta Engineering, "Scaling the Instagram Explore recommendations system" (2023): https://engineering.fb.com/2023/08/09/ml-applications/scaling-instagram-explore-recommendations-system/
- Meta Engineering, "Adapting the Facebook Reels RecSys AI model based on user feedback" (2026): https://engineering.fb.com/2026/01/14/ml-applications/adapting-the-facebook-reels-recsys-ai-model-based-on-user-feedback/
- TikTok, "How TikTok recommends videos #ForYou": https://newsroom.tiktok.com/en-us/how-tiktok-recommends-videos-for-you
- TikTok, "An update on our work to safeguard and diversify recommendations": https://newsroom.tiktok.com/en-us/an-update-on-our-work-to-safeguard-and-diversify-recommendations
- YouTube, "On YouTube's recommendation system": https://blog.youtube/inside-youtube/on-youtubes-recommendation-system/
- YouTube Help, take a break reminder: https://support.google.com/youtube/answer/9012523
- Twitter/X, the-algorithm-ml heavy ranker weights: https://github.com/twitter/the-algorithm-ml/blob/main/projects/home/recap/README.md
- Duolingo, "Introducing Birdbrain": https://blog.duolingo.com/learning-how-to-help-you-learn-introducing-birdbrain/

Papers:

- Liu et al., Monolith (2022): https://arxiv.org/abs/2209.07663
- Zhai et al., HSTU / Generative Recommenders (2024): https://arxiv.org/abs/2402.17152
- Covington et al., Deep Neural Networks for YouTube Recommendations (2016): https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45530.pdf
- Zhao et al., Recommending What Video to Watch Next (2019): https://dl.acm.org/doi/10.1145/3298689.3346997
- Wilhelm et al., Practical Diversified Recommendations on YouTube with DPPs (2018): https://dl.acm.org/doi/10.1145/3269206.3272018
- Chen et al., Fast Greedy MAP Inference for DPP (2018): https://arxiv.org/abs/1709.05135
- Su et al., Long-Term Value of Exploration (2024): https://arxiv.org/abs/2305.07764
- Zhan et al., D2Q (2022): https://arxiv.org/abs/2206.06003
- Lin et al., TPM (2023): https://arxiv.org/abs/2306.03392
- Zheng et al., DVR / WTG (2022): https://arxiv.org/abs/2208.05190
- Zhao et al., D2Co (2023): https://arxiv.org/abs/2308.08120
- Cai et al., RLUR (2023): https://arxiv.org/abs/2302.01724
- Li et al., LinUCB (2010): https://arxiv.org/abs/1003.0146
- Chapelle & Li, Thompson Sampling (2011): https://papers.nips.cc/paper/2011/hash/e53a0a2978c28872a4505bdb51db06dc-Abstract.html
- McInerney et al., Explore, Exploit, Explain (2018): https://dl.acm.org/doi/10.1145/3240323.3240354
- Carbonell & Goldstein, MMR (1998): https://dl.acm.org/doi/10.1145/290941.291025
- Settles & Meeder, Half-Life Regression (2016): https://aclanthology.org/P16-1174.pdf
- Wilson et al., The Eighty Five Percent Rule (2019): https://www.nature.com/articles/s41467-019-12552-4
- Corbett & Anderson, Knowledge Tracing (1995): https://doi.org/10.1007/BF01099821
- Piech et al., Deep Knowledge Tracing (2015): https://arxiv.org/abs/1506.05908

Law and regulation:

- EU AI Act Art. 5: https://artificialintelligenceact.eu/article/5/
- EU Digital Services Act (Reg. 2022/2065), Art. 27 and 38: https://eur-lex.europa.eu/eli/reg/2022/2065/oj
- Saudi PDPL, SDAIA: https://sdaia.gov.sa/en/SDAIA/about/Pages/RegulationsAndPolicies.aspx (official Arabic text: laws.boe.gov.sa)

Reported or unverified:

- NYT, "How TikTok Reads Your Mind" (2021-12-05) [reported]: https://www.nytimes.com/2021/12/05/business/media/tiktok-algorithm.html
- AP via WABE, TikTok 60-minute teen default (2023-03-01) [reported]: https://www.wabe.org/tiktok-sets-new-default-time-limits-for-minors/
- Bloomberg, Instagram "Take a Break" (2021-11-10) [reported]: https://www.bloomberg.com/news/articles/2021-11-10/instagram-tests-take-a-break-tool-to-let-users-self-regulate
- Clyde & Co, PDPL summary [unverified, law-firm summary]: https://www.clydeco.com/en/insights/2021/09/saudi-arabia-issues-personal-data-protection-law
