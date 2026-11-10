-- Reelearn · learning engine
--
-- The ranking engine behind the daily minute and "keep going" practice: engagement signals,
-- per-person interest scores (topics and tags), reel statistics, colleague sharing, practice
-- sessions and the hourly aggregates the ranker reads. Design notes: docs/engine.md.

-- ---------------------------------------------------------------- reel features
-- Topic tags and a 1..5 difficulty, written by the studio pipeline (Muse Spark / Claude / fake).
ALTER TABLE reels ADD COLUMN tags text[] NOT NULL DEFAULT '{}';
ALTER TABLE reels ADD COLUMN difficulty smallint CHECK (difficulty IS NULL OR difficulty BETWEEN 1 AND 5);
CREATE INDEX reels_tags_gin ON reels USING gin (tags);

-- A person may turn personalisation off (EU DSA Art. 27/38 as best practice): their practice is
-- then ordered without interest, colleague activity or exploration, by assignment and recency.
ALTER TABLE users ADD COLUMN personalized boolean NOT NULL DEFAULT true;

-- ---------------------------------------------------------------- engagement events
CREATE TYPE engagement_kind_t AS ENUM (
  'impression',      -- the ranker served the reel (server side, carries score and reason)
  'start',           -- the reel became the active card on the phone
  'skip',            -- left within the first three seconds
  'complete',        -- played past 90 %
  'rewatch',         -- played again after completing
  'save', 'unsave',
  'share',           -- sent to a colleague
  'not_relevant',    -- "not for my job"
  'more_like_this'
);

CREATE TABLE engagement_events (
  id           uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL,
  user_id      uuid NOT NULL,
  reel_id      uuid NOT NULL,
  session_id   uuid,
  kind         engagement_kind_t NOT NULL,
  watched_ms   integer CHECK (watched_ms IS NULL OR watched_ms >= 0),
  position     smallint,
  score        real,
  reason       text,
  channel      channel_t NOT NULL DEFAULT 'app',
  occurred_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);
CREATE TABLE engagement_events_default PARTITION OF engagement_events DEFAULT;
CREATE INDEX engagement_events_user_idx ON engagement_events (tenant_id, user_id, occurred_at DESC);
CREATE INDEX engagement_events_reel_idx ON engagement_events (tenant_id, reel_id, occurred_at);
CREATE INDEX engagement_events_kind_idx ON engagement_events (tenant_id, kind, occurred_at);
CREATE INDEX engagement_events_session_idx ON engagement_events (session_id) WHERE session_id IS NOT NULL;
CREATE INDEX engagement_events_brin ON engagement_events USING brin (occurred_at);

SELECT ensure_month_partition('engagement_events', m)
FROM unnest(ARRAY[CURRENT_DATE, (CURRENT_DATE + interval '1 month')::date]) m;

-- ---------------------------------------------------------------- running reel statistics
-- Incremented in the same transaction as each event, so the ranker sees signals in real time.
CREATE TABLE reel_engagement_stats (
  reel_id         uuid PRIMARY KEY REFERENCES reels(id) ON DELETE CASCADE,
  tenant_id       uuid NOT NULL REFERENCES tenants(id),
  impressions     integer NOT NULL DEFAULT 0,
  starts          integer NOT NULL DEFAULT 0,
  skips           integer NOT NULL DEFAULT 0,
  completions     integer NOT NULL DEFAULT 0,
  rewatches       integer NOT NULL DEFAULT 0,
  saves           integer NOT NULL DEFAULT 0,
  shares          integer NOT NULL DEFAULT 0,
  not_relevant    integer NOT NULL DEFAULT 0,
  more_like_this  integer NOT NULL DEFAULT 0,
  answers         integer NOT NULL DEFAULT 0,
  answers_correct integer NOT NULL DEFAULT 0,
  watched_ms      bigint  NOT NULL DEFAULT 0,
  updated_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX reel_engagement_stats_tenant_idx ON reel_engagement_stats (tenant_id);

-- Completion baseline per duration band, so a 20 s reel and a 70 s reel are judged on their own scale.
CREATE TABLE duration_band_stats (
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  band         smallint NOT NULL CHECK (band BETWEEN 0 AND 4),
  starts       integer NOT NULL DEFAULT 0,
  completions  integer NOT NULL DEFAULT 0,
  skips        integer NOT NULL DEFAULT 0,
  refreshed_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, band)
);

-- What colleagues in the same location, job title or team finished, saved and shared in the last 7 days.
CREATE TABLE cohort_reel_stats (
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  cohort_kind   text NOT NULL CHECK (cohort_kind IN ('location', 'job_title', 'team')),
  cohort_key    text NOT NULL,
  reel_id       uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  completions_7d integer NOT NULL DEFAULT 0,
  saves_7d      integer NOT NULL DEFAULT 0,
  shares_7d     integer NOT NULL DEFAULT 0,
  learners_7d   integer NOT NULL DEFAULT 0,
  refreshed_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, cohort_kind, cohort_key, reel_id)
);
CREATE INDEX cohort_reel_stats_lookup_idx ON cohort_reel_stats (tenant_id, cohort_kind, cohort_key, completions_7d DESC);

-- ---------------------------------------------------------------- interest
-- Time-decayed interest per person and topic. `score` is stored as of `last_event_at`; readers decay
-- it to now with the tenant's half-life. `preference` is the person's own follow / mute choice.
CREATE TABLE user_topic_affinity (
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  user_id        uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  topic_id       uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
  score          real NOT NULL DEFAULT 0,
  events         integer NOT NULL DEFAULT 0,
  last_event_at  timestamptz NOT NULL DEFAULT now(),
  preference     text CHECK (preference IS NULL OR preference IN ('follow', 'mute')),
  updated_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, topic_id)
);
CREATE INDEX user_topic_affinity_tenant_topic_idx ON user_topic_affinity (tenant_id, topic_id);

CREATE TABLE user_tag_affinity (
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  user_id        uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tag            text NOT NULL,
  score          real NOT NULL DEFAULT 0,
  events         integer NOT NULL DEFAULT 0,
  last_event_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, tag)
);
CREATE INDEX user_tag_affinity_tenant_tag_idx ON user_tag_affinity (tenant_id, tag);

-- ---------------------------------------------------------------- sharing between colleagues
CREATE TABLE reel_shares (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  reel_id       uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  sender_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  recipient_id  uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  note          text CHECK (note IS NULL OR char_length(note) <= 280),
  created_at    timestamptz NOT NULL DEFAULT now(),
  opened_at     timestamptz,
  CONSTRAINT reel_shares_not_self CHECK (sender_id <> recipient_id)
);
CREATE UNIQUE INDEX reel_shares_once_uq ON reel_shares (reel_id, sender_id, recipient_id);
CREATE INDEX reel_shares_inbox_idx ON reel_shares (recipient_id, created_at DESC) WHERE opened_at IS NULL;
CREATE INDEX reel_shares_sender_idx ON reel_shares (sender_id, created_at DESC);
CREATE INDEX reel_shares_tenant_idx ON reel_shares (tenant_id, created_at);

-- ---------------------------------------------------------------- practice sessions
CREATE TABLE engine_sessions (
  id              uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id       uuid NOT NULL REFERENCES tenants(id),
  user_id         uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  started_at      timestamptz NOT NULL DEFAULT now(),
  last_active_at  timestamptz NOT NULL DEFAULT now(),
  ended_at        timestamptz,
  items_served    integer NOT NULL DEFAULT 0,
  items_completed integer NOT NULL DEFAULT 0,
  active_ms       bigint  NOT NULL DEFAULT 0,
  stop_reason     text
);
CREATE INDEX engine_sessions_user_idx ON engine_sessions (tenant_id, user_id, started_at DESC);
CREATE INDEX engine_sessions_tenant_idx ON engine_sessions (tenant_id, started_at);

-- ---------------------------------------------------------------- row-level security
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY[
    'engagement_events','reel_engagement_stats','duration_band_stats','cohort_reel_stats',
    'user_topic_affinity','user_tag_affinity','reel_shares','engine_sessions'
  ] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format(
      'CREATE POLICY %I ON %I USING (tenant_id = current_setting(''app.tenant_id'', true)::uuid) WITH CHECK (tenant_id = current_setting(''app.tenant_id'', true)::uuid)',
      t || '_tenant_isolation', t);
  END LOOP;
END $$;

-- New tables inherit grants through 0002's default privileges; the partitions created above too.
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO reelearn_app, reelearn_admin;
