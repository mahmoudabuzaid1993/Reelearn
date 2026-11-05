-- Reelearn · initial schema
-- Postgres 17. Multi-tenant by tenant_id, RLS on every tenant table.
-- See docs/db-schema.md for the rationale behind each table and index.

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- UUIDv7: time-ordered ids so B-tree inserts append instead of splitting pages.
CREATE OR REPLACE FUNCTION uuid_generate_v7() RETURNS uuid AS $$
DECLARE
  unix_ts_ms bytea;
  uuid_bytes bytea;
BEGIN
  unix_ts_ms = substring(int8send(floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint) FROM 3);
  uuid_bytes = unix_ts_ms || gen_random_bytes(10);
  uuid_bytes = set_byte(uuid_bytes, 6, (b'0111' || get_byte(uuid_bytes, 6)::bit(4))::bit(8)::int);
  uuid_bytes = set_byte(uuid_bytes, 8, (b'10' || get_byte(uuid_bytes, 8)::bit(6))::bit(8)::int);
  RETURN encode(uuid_bytes, 'hex')::uuid;
END
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END $$ LANGUAGE plpgsql;

-- ---------------------------------------------------------------- enums
CREATE TYPE region_t          AS ENUM ('ksa', 'uae', 'eu');
CREATE TYPE user_role_t       AS ENUM ('admin', 'manager', 'sme', 'learner');
CREATE TYPE user_status_t     AS ENUM ('invited', 'active', 'paused', 'deactivated');
CREATE TYPE channel_t         AS ENUM ('app', 'whatsapp', 'both', 'teams');
CREATE TYPE delivery_slot_t   AS ENUM ('shift_start', 'break', 'shift_end');
CREATE TYPE source_kind_t     AS ENUM ('pdf', 'pptx', 'image', 'url', 'youtube', 'topic_prompt', 'podcast_audio', 'video', 'sme_recording');
CREATE TYPE extraction_status_t AS ENUM ('pending', 'running', 'done', 'failed');
CREATE TYPE job_status_t      AS ENUM ('queued', 'running', 'paused', 'done', 'failed', 'cancelled');
CREATE TYPE gen_step_t        AS ENUM ('extract', 'script', 'render', 'voice', 'caption', 'question', 'translate', 'transcribe', 'find_moments', 'cut');
CREATE TYPE reel_kind_t       AS ENUM ('ai_avatar', 'motion', 'broll', 'podcast_cut', 'sme_recorded');
CREATE TYPE reel_status_t     AS ENUM ('draft', 'in_review', 'approved', 'published', 'archived', 'rejected');
CREATE TYPE approval_role_t   AS ENUM ('ld', 'sme');
CREATE TYPE decision_t        AS ENUM ('approved', 'changes_requested', 'rejected');
CREATE TYPE plan_item_kind_t  AS ENUM ('reel', 'question_only');
CREATE TYPE plan_item_reason_t AS ENUM ('new', 'review');
CREATE TYPE plan_item_state_t AS ENUM ('pending', 'watched', 'answered');
CREATE TYPE delivery_status_t AS ENUM ('scheduled', 'sent', 'delivered', 'read', 'failed', 'cancelled');
CREATE TYPE integration_kind_t AS ENUM ('hris_jisr', 'hris_bayzat', 'sso_entra', 'whatsapp', 'teams', 'ai_claude', 'ai_higgsfield', 'ai_veo', 'voice', 'asr', 'storage');
CREATE TYPE export_kind_t     AS ENUM ('hrdf_evidence', 'personal_data');

-- ---------------------------------------------------------------- tenancy
CREATE TABLE tenants (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  slug          text NOT NULL,
  name          text NOT NULL,
  region        region_t NOT NULL DEFAULT 'ksa',
  default_language text NOT NULL DEFAULT 'ar',
  languages     text[] NOT NULL DEFAULT '{ar,en}',
  timezone      text NOT NULL DEFAULT 'Asia/Riyadh',
  logo_key      text,
  settings      jsonb NOT NULL DEFAULT '{}'::jsonb,
  credits       integer NOT NULL DEFAULT 0 CHECK (credits >= 0),
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT tenants_slug_format CHECK (slug ~ '^[a-z0-9][a-z0-9-]{1,62}$')
);
CREATE UNIQUE INDEX tenants_slug_uq ON tenants (slug);
CREATE TRIGGER tenants_updated BEFORE UPDATE ON tenants FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE users (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  role          user_role_t NOT NULL DEFAULT 'learner',
  status        user_status_t NOT NULL DEFAULT 'invited',
  full_name     text NOT NULL,
  phone_e164    text,
  email         text,
  employee_no   text,
  preferred_language text NOT NULL DEFAULT 'ar',
  ui_language   text NOT NULL DEFAULT 'ar',
  manager_id    uuid REFERENCES users(id),
  location      text,
  job_title     text,
  delivery_channel channel_t NOT NULL DEFAULT 'app',
  delivery_slot delivery_slot_t NOT NULL DEFAULT 'shift_start',
  shift_start   time,
  shift_end     time,
  captions_on   boolean NOT NULL DEFAULT true,
  reminders_on  boolean NOT NULL DEFAULT true,
  extra_reels_on boolean NOT NULL DEFAULT false,
  paused_until  date,
  last_seen_at  timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT users_identity CHECK (phone_e164 IS NOT NULL OR email IS NOT NULL),
  CONSTRAINT users_phone_format CHECK (phone_e164 IS NULL OR phone_e164 ~ '^\+[1-9][0-9]{6,14}$')
);
CREATE UNIQUE INDEX users_tenant_phone_uq ON users (tenant_id, phone_e164) WHERE phone_e164 IS NOT NULL;
CREATE UNIQUE INDEX users_tenant_email_uq ON users (tenant_id, lower(email)) WHERE email IS NOT NULL;
CREATE UNIQUE INDEX users_tenant_employee_no_uq ON users (tenant_id, employee_no) WHERE employee_no IS NOT NULL;
CREATE INDEX users_tenant_status_role_idx ON users (tenant_id, status, role);
CREATE INDEX users_tenant_manager_idx ON users (tenant_id, manager_id) WHERE manager_id IS NOT NULL;
CREATE INDEX users_name_trgm_idx ON users USING gin (full_name gin_trgm_ops);
CREATE TRIGGER users_updated BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE groups (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  name        text NOT NULL,
  kind        text NOT NULL DEFAULT 'team',   -- team | location | role | custom
  external_ref text,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX groups_tenant_name_uq ON groups (tenant_id, name);

CREATE TABLE group_members (
  group_id  uuid NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  user_id   uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id),
  added_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (group_id, user_id)
);
CREATE INDEX group_members_user_idx ON group_members (user_id);

CREATE TABLE devices (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_id   text NOT NULL,
  platform    text NOT NULL,                -- ios | android | web
  push_token  text,
  app_version text,
  last_seen_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (user_id, device_id)
);

CREATE TABLE otp_challenges (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  user_id     uuid REFERENCES users(id) ON DELETE CASCADE,
  phone_e164  text,
  email       text,
  channel     text NOT NULL,                -- whatsapp | sms | email
  code_hash   text NOT NULL,
  attempts    smallint NOT NULL DEFAULT 0,
  expires_at  timestamptz NOT NULL,
  consumed_at timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX otp_challenges_phone_idx ON otp_challenges (phone_e164, created_at DESC) WHERE phone_e164 IS NOT NULL;
CREATE INDEX otp_challenges_email_idx ON otp_challenges (lower(email), created_at DESC) WHERE email IS NOT NULL;

CREATE TABLE refresh_tokens (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_id   text,
  token_hash  text NOT NULL,
  expires_at  timestamptz NOT NULL,
  revoked_at  timestamptz,
  replaced_by uuid,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX refresh_tokens_hash_uq ON refresh_tokens (token_hash);
CREATE INDEX refresh_tokens_user_idx ON refresh_tokens (user_id);

-- ---------------------------------------------------------------- media
CREATE TABLE media_assets (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  storage_key text NOT NULL,
  mime        text NOT NULL,
  bytes       bigint NOT NULL CHECK (bytes >= 0),
  sha256      text,
  width       integer,
  height      integer,
  duration_ms integer,
  created_by  uuid REFERENCES users(id),
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX media_assets_key_uq ON media_assets (storage_key);
CREATE INDEX media_assets_tenant_sha_idx ON media_assets (tenant_id, sha256) WHERE sha256 IS NOT NULL;

-- ---------------------------------------------------------------- content
CREATE TABLE topics (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  slug           text NOT NULL,
  name           text NOT NULL,
  name_ar        text,
  description    text,
  compliance_tag text,
  is_mandatory   boolean NOT NULL DEFAULT false,
  status         text NOT NULL DEFAULT 'active',   -- active | archived
  created_at     timestamptz NOT NULL DEFAULT now(),
  updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX topics_tenant_slug_uq ON topics (tenant_id, slug);
CREATE TRIGGER topics_updated BEFORE UPDATE ON topics FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE topic_assignments (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  topic_id       uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
  group_id       uuid NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  reels_per_day  smallint NOT NULL DEFAULT 2 CHECK (reels_per_day BETWEEN 0 AND 5),
  channel        channel_t NOT NULL DEFAULT 'app',
  is_mandatory   boolean NOT NULL DEFAULT false,
  starts_on      date NOT NULL DEFAULT CURRENT_DATE,
  ends_on        date,
  created_at     timestamptz NOT NULL DEFAULT now(),
  UNIQUE (topic_id, group_id)
);
CREATE INDEX topic_assignments_tenant_group_idx ON topic_assignments (tenant_id, group_id);
CREATE INDEX topic_assignments_tenant_topic_idx ON topic_assignments (tenant_id, topic_id);

CREATE TABLE sources (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  kind           source_kind_t NOT NULL,
  title          text NOT NULL,
  url            text,
  media_asset_id uuid REFERENCES media_assets(id),
  prompt         text,
  language       text,
  extraction_status extraction_status_t NOT NULL DEFAULT 'pending',
  extraction_error text,
  created_by     uuid NOT NULL REFERENCES users(id),
  created_at     timestamptz NOT NULL DEFAULT now(),
  updated_at     timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT sources_payload CHECK (url IS NOT NULL OR media_asset_id IS NOT NULL OR prompt IS NOT NULL)
);
CREATE INDEX sources_tenant_created_idx ON sources (tenant_id, created_at DESC);
CREATE TRIGGER sources_updated BEFORE UPDATE ON sources FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE source_extractions (
  source_id    uuid PRIMARY KEY REFERENCES sources(id) ON DELETE CASCADE,
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  text         text NOT NULL,
  summary      jsonb NOT NULL DEFAULT '{}'::jsonb,   -- key ideas, candidate reels
  language     text,
  word_count   integer,
  extracted_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE source_moments (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  source_id   uuid NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  start_ms    integer NOT NULL CHECK (start_ms >= 0),
  end_ms      integer NOT NULL,
  excerpt     text NOT NULL,
  title       text,
  score       real NOT NULL DEFAULT 0,
  speaker     text,
  selected    boolean NOT NULL DEFAULT false,
  reel_id     uuid,
  CONSTRAINT source_moments_range CHECK (end_ms > start_ms AND end_ms - start_ms <= 90000)
);
CREATE INDEX source_moments_source_score_idx ON source_moments (source_id, score DESC);

CREATE TABLE generation_jobs (
  id            uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  source_id     uuid NOT NULL REFERENCES sources(id),
  requested_by  uuid NOT NULL REFERENCES users(id),
  reviewer_id   uuid REFERENCES users(id),
  params        jsonb NOT NULL,     -- {languages, reel_count, style, voice_id, presenter}
  status        job_status_t NOT NULL DEFAULT 'queued',
  progress_pct  smallint NOT NULL DEFAULT 0 CHECK (progress_pct BETWEEN 0 AND 100),
  cost_credits  integer NOT NULL DEFAULT 0 CHECK (cost_credits >= 0),
  error         text,
  started_at    timestamptz,
  finished_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX generation_jobs_tenant_created_idx ON generation_jobs (tenant_id, created_at DESC);
CREATE INDEX generation_jobs_active_idx ON generation_jobs (status, updated_at) WHERE status IN ('queued', 'running', 'paused');
CREATE TRIGGER generation_jobs_updated BEFORE UPDATE ON generation_jobs FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE generation_steps (
  id             uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id),
  job_id         uuid NOT NULL REFERENCES generation_jobs(id) ON DELETE CASCADE,
  position       smallint NOT NULL,
  step           gen_step_t NOT NULL,
  status         job_status_t NOT NULL DEFAULT 'queued',
  provider       text,
  provider_ref   text,
  attempt        smallint NOT NULL DEFAULT 0,
  log            text,
  started_at     timestamptz,
  finished_at    timestamptz,
  UNIQUE (job_id, position)
);

CREATE TABLE reels (
  id                uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id         uuid NOT NULL REFERENCES tenants(id),
  topic_id          uuid NOT NULL REFERENCES topics(id),
  source_id         uuid REFERENCES sources(id),
  generation_job_id uuid REFERENCES generation_jobs(id),
  kind              reel_kind_t NOT NULL,
  status            reel_status_t NOT NULL DEFAULT 'draft',
  title             text NOT NULL,
  language          text NOT NULL,
  script            text,
  source_citation   text,
  presenter         jsonb NOT NULL DEFAULT '{}'::jsonb,   -- {name, kind: ai|human, provider_id}
  video_key         text,
  thumbnail_key     text,
  captions_key      text,
  duration_ms       integer CHECK (duration_ms IS NULL OR duration_ms BETWEEN 1000 AND 120000),
  version           integer NOT NULL DEFAULT 1,
  compliance_tag    text,
  consent_confirmed boolean NOT NULL DEFAULT false,
  created_by        uuid NOT NULL REFERENCES users(id),
  published_at      timestamptz,
  archived_at       timestamptz,
  created_at        timestamptz NOT NULL DEFAULT now(),
  updated_at        timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX reels_tenant_status_updated_idx ON reels (tenant_id, status, updated_at DESC);
CREATE INDEX reels_tenant_topic_published_idx ON reels (tenant_id, topic_id, published_at DESC) WHERE status = 'published';
CREATE INDEX reels_tenant_job_idx ON reels (tenant_id, generation_job_id) WHERE generation_job_id IS NOT NULL;
CREATE INDEX reels_search_idx ON reels USING gin (to_tsvector('simple', title || ' ' || coalesce(script, '')));
CREATE TRIGGER reels_updated BEFORE UPDATE ON reels FOR EACH ROW EXECUTE FUNCTION set_updated_at();
ALTER TABLE source_moments ADD CONSTRAINT source_moments_reel_fk FOREIGN KEY (reel_id) REFERENCES reels(id);

CREATE TABLE reel_variants (
  id           uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  reel_id      uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  language     text NOT NULL,
  video_key    text,
  captions_key text,
  audio_key    text,
  script       text,
  dub_status   extraction_status_t NOT NULL DEFAULT 'pending',
  created_at   timestamptz NOT NULL DEFAULT now(),
  UNIQUE (reel_id, language)
);

CREATE TABLE questions (
  id           uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  reel_id      uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  language     text NOT NULL,
  prompt       text NOT NULL,
  options      jsonb NOT NULL,          -- [{"key":"A","text":"..."}]
  correct_key  text NOT NULL,
  explanation  text,
  created_at   timestamptz NOT NULL DEFAULT now(),
  updated_at   timestamptz NOT NULL DEFAULT now(),
  UNIQUE (reel_id, language),
  CONSTRAINT questions_options_array CHECK (jsonb_typeof(options) = 'array' AND jsonb_array_length(options) BETWEEN 2 AND 5)
);
CREATE TRIGGER questions_updated BEFORE UPDATE ON questions FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE OR REPLACE FUNCTION questions_check_correct_key() RETURNS trigger AS $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM jsonb_array_elements(NEW.options) o WHERE o->>'key' = NEW.correct_key) THEN
    RAISE EXCEPTION 'correct_key % is not one of the option keys', NEW.correct_key;
  END IF;
  RETURN NEW;
END $$ LANGUAGE plpgsql;
CREATE TRIGGER questions_correct_key BEFORE INSERT OR UPDATE ON questions FOR EACH ROW EXECUTE FUNCTION questions_check_correct_key();

CREATE TABLE approvals (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  reel_id     uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  reviewer_id uuid NOT NULL REFERENCES users(id),
  role        approval_role_t NOT NULL,
  decision    decision_t NOT NULL,
  comment     text,
  reel_version integer NOT NULL,
  decided_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX approvals_reel_idx ON approvals (reel_id, decided_at DESC);

-- A reel may only become published with an approved L&D and an approved SME decision on its current version.
CREATE OR REPLACE FUNCTION reels_require_dual_approval() RETURNS trigger AS $$
BEGIN
  IF NEW.status = 'published' AND (OLD.status IS DISTINCT FROM 'published') THEN
    IF (SELECT count(DISTINCT role) FROM approvals a
        WHERE a.reel_id = NEW.id AND a.decision = 'approved' AND a.reel_version = NEW.version) < 2 THEN
      RAISE EXCEPTION 'reel % needs an approved L&D and SME sign-off on version % before publishing', NEW.id, NEW.version;
    END IF;
    NEW.published_at = coalesce(NEW.published_at, now());
  END IF;
  RETURN NEW;
END $$ LANGUAGE plpgsql;
CREATE TRIGGER reels_dual_approval BEFORE UPDATE ON reels FOR EACH ROW EXECUTE FUNCTION reels_require_dual_approval();

CREATE TABLE reel_audiences (
  reel_id   uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  group_id  uuid NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id),
  starts_on date NOT NULL DEFAULT CURRENT_DATE,
  PRIMARY KEY (reel_id, group_id)
);
CREATE INDEX reel_audiences_group_idx ON reel_audiences (tenant_id, group_id, starts_on);

-- ---------------------------------------------------------------- learning
CREATE TABLE daily_plans (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  plan_date   date NOT NULL,
  status      text NOT NULL DEFAULT 'open',   -- open | done | skipped
  item_count  smallint NOT NULL DEFAULT 0,
  done_count  smallint NOT NULL DEFAULT 0,
  completed_at timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, user_id, plan_date)
);

CREATE TABLE daily_plan_items (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  plan_id     uuid NOT NULL REFERENCES daily_plans(id) ON DELETE CASCADE,
  position    smallint NOT NULL,
  kind        plan_item_kind_t NOT NULL,
  reason      plan_item_reason_t NOT NULL,
  reel_id     uuid REFERENCES reels(id),
  question_id uuid REFERENCES questions(id),
  state       plan_item_state_t NOT NULL DEFAULT 'pending',
  UNIQUE (plan_id, position),
  CONSTRAINT plan_item_refs CHECK (
    (kind = 'reel' AND reel_id IS NOT NULL) OR
    (kind = 'question_only' AND question_id IS NOT NULL)
  )
);
CREATE INDEX daily_plan_items_tenant_reel_idx ON daily_plan_items (tenant_id, reel_id) WHERE reel_id IS NOT NULL;

CREATE TABLE memory_states (
  user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  question_id   uuid NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  due_on        date NOT NULL,
  interval_days smallint NOT NULL DEFAULT 1 CHECK (interval_days >= 0),
  ease          real NOT NULL DEFAULT 2.5 CHECK (ease BETWEEN 1.3 AND 3.0),
  reps          smallint NOT NULL DEFAULT 0,
  lapses        smallint NOT NULL DEFAULT 0,
  last_result   boolean,
  last_answered_at timestamptz,
  PRIMARY KEY (user_id, question_id)
);
CREATE INDEX memory_states_tenant_user_due_idx ON memory_states (tenant_id, user_id, due_on);
CREATE INDEX memory_states_tenant_due_idx ON memory_states (tenant_id, due_on);

CREATE TABLE watch_events (
  id           uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL,
  user_id      uuid NOT NULL,
  reel_id      uuid NOT NULL,
  plan_item_id uuid,
  channel      channel_t NOT NULL DEFAULT 'app',
  device_id    text,
  watched_ms   integer NOT NULL DEFAULT 0 CHECK (watched_ms >= 0),
  completed    boolean NOT NULL DEFAULT false,
  started_at   timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (id, started_at)
) PARTITION BY RANGE (started_at);
CREATE INDEX watch_events_tenant_user_idx ON watch_events (tenant_id, user_id, started_at DESC);
CREATE INDEX watch_events_tenant_reel_idx ON watch_events (tenant_id, reel_id, started_at);
CREATE INDEX watch_events_started_brin ON watch_events USING brin (started_at);
CREATE TABLE watch_events_default PARTITION OF watch_events DEFAULT;

CREATE TABLE answers (
  id           uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL,
  user_id      uuid NOT NULL,
  question_id  uuid NOT NULL,
  reel_id      uuid NOT NULL,
  plan_item_id uuid,
  chosen_key   text NOT NULL,
  is_correct   boolean NOT NULL,
  response_ms  integer CHECK (response_ms IS NULL OR response_ms >= 0),
  attempt_no   smallint NOT NULL DEFAULT 1,
  channel      channel_t NOT NULL DEFAULT 'app',
  answered_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (id, answered_at)
) PARTITION BY RANGE (answered_at);
CREATE INDEX answers_tenant_user_idx ON answers (tenant_id, user_id, answered_at DESC);
CREATE INDEX answers_tenant_question_idx ON answers (tenant_id, question_id, answered_at);
CREATE INDEX answers_answered_brin ON answers USING brin (answered_at);
CREATE TABLE answers_default PARTITION OF answers DEFAULT;

CREATE TABLE saved_reels (
  user_id   uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  reel_id   uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id),
  saved_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, reel_id)
);
CREATE INDEX saved_reels_reel_idx ON saved_reels (reel_id);

CREATE TABLE reel_requests (
  id         uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id),
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  text       text NOT NULL CHECK (char_length(text) BETWEEN 3 AND 500),
  status     text NOT NULL DEFAULT 'open',   -- open | planned | done | dismissed
  votes      integer NOT NULL DEFAULT 1,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX reel_requests_tenant_status_idx ON reel_requests (tenant_id, status, votes DESC);

CREATE TABLE user_stats (
  user_id            uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  tenant_id          uuid NOT NULL REFERENCES tenants(id),
  streak_current     integer NOT NULL DEFAULT 0,
  streak_longest     integer NOT NULL DEFAULT 0,
  last_active_on     date,
  freezes_available  smallint NOT NULL DEFAULT 1,
  reels_completed    integer NOT NULL DEFAULT 0,
  answers_total      integer NOT NULL DEFAULT 0,
  answers_correct    integer NOT NULL DEFAULT 0,
  remembered_count   integer NOT NULL DEFAULT 0,   -- questions with reps >= 2 and last_result true
  minutes_on_mandatory numeric(10,2) NOT NULL DEFAULT 0,
  updated_at         timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX user_stats_tenant_last_active_idx ON user_stats (tenant_id, last_active_on);

CREATE TABLE user_topic_readiness (
  user_id     uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  topic_id    uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  ready       boolean NOT NULL DEFAULT false,
  recall_pct  smallint NOT NULL DEFAULT 0 CHECK (recall_pct BETWEEN 0 AND 100),
  reels_seen  integer NOT NULL DEFAULT 0,
  reels_total integer NOT NULL DEFAULT 0,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, topic_id)
);
CREATE INDEX user_topic_readiness_tenant_topic_idx ON user_topic_readiness (tenant_id, topic_id, ready);

CREATE TABLE daily_topic_stats (
  tenant_id        uuid NOT NULL REFERENCES tenants(id),
  topic_id         uuid NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
  group_id         uuid NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  day              date NOT NULL,
  learners_active  integer NOT NULL DEFAULT 0,
  reels_completed  integer NOT NULL DEFAULT 0,
  answers_total    integer NOT NULL DEFAULT 0,
  answers_correct  integer NOT NULL DEFAULT 0,
  minutes_total    numeric(12,2) NOT NULL DEFAULT 0,
  PRIMARY KEY (tenant_id, topic_id, group_id, day)
);
CREATE INDEX daily_topic_stats_tenant_day_idx ON daily_topic_stats (tenant_id, day);

-- ---------------------------------------------------------------- delivery and ops
CREATE TABLE deliveries (
  id             uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id      uuid NOT NULL,
  user_id        uuid NOT NULL,
  plan_id        uuid,
  channel        channel_t NOT NULL,
  kind           text NOT NULL,            -- daily_minute | reminder | nudge | otp
  status         delivery_status_t NOT NULL DEFAULT 'scheduled',
  scheduled_for  timestamptz NOT NULL,
  sent_at        timestamptz,
  provider_ref   text,
  error          text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (id, scheduled_for)
) PARTITION BY RANGE (scheduled_for);
CREATE INDEX deliveries_due_idx ON deliveries (status, scheduled_for) WHERE status = 'scheduled';
CREATE INDEX deliveries_tenant_user_idx ON deliveries (tenant_id, user_id, scheduled_for DESC);
CREATE TABLE deliveries_default PARTITION OF deliveries DEFAULT;

CREATE TABLE whatsapp_messages (
  id           uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  user_id      uuid REFERENCES users(id),
  phone_e164   text NOT NULL,
  direction    text NOT NULL,            -- in | out
  body         text,
  payload      jsonb,
  provider_ref text,
  created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX whatsapp_messages_tenant_phone_idx ON whatsapp_messages (tenant_id, phone_e164, created_at DESC);

CREATE TABLE jobs (
  id          uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id   uuid REFERENCES tenants(id),
  kind        text NOT NULL,
  payload     jsonb NOT NULL DEFAULT '{}'::jsonb,
  status      job_status_t NOT NULL DEFAULT 'queued',
  run_at      timestamptz NOT NULL DEFAULT now(),
  attempts    smallint NOT NULL DEFAULT 0,
  max_attempts smallint NOT NULL DEFAULT 8,
  locked_by   text,
  locked_at   timestamptz,
  last_error  text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  finished_at timestamptz
);
CREATE INDEX jobs_queue_idx ON jobs (run_at) WHERE status = 'queued';
CREATE INDEX jobs_kind_status_idx ON jobs (kind, status);

CREATE TABLE integrations (
  id         uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id),
  kind       integration_kind_t NOT NULL,
  status     text NOT NULL DEFAULT 'disconnected',   -- disconnected | connected | error
  config_enc bytea,                                   -- AES-GCM encrypted JSON
  last_sync_at timestamptz,
  last_error text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, kind)
);
CREATE TRIGGER integrations_updated BEFORE UPDATE ON integrations FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE exports (
  id           uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
  tenant_id    uuid NOT NULL REFERENCES tenants(id),
  kind         export_kind_t NOT NULL,
  requested_by uuid NOT NULL REFERENCES users(id),
  subject_user_id uuid REFERENCES users(id),
  period_start date,
  period_end   date,
  status       job_status_t NOT NULL DEFAULT 'queued',
  file_key     text,
  error        text,
  created_at   timestamptz NOT NULL DEFAULT now(),
  finished_at  timestamptz
);
CREATE INDEX exports_tenant_created_idx ON exports (tenant_id, created_at DESC);

CREATE TABLE audit_log (
  id         uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id  uuid NOT NULL,
  actor_id   uuid,
  action     text NOT NULL,
  entity     text NOT NULL,
  entity_id  uuid,
  diff       jsonb,
  ip         inet,
  at         timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (id, at)
) PARTITION BY RANGE (at);
CREATE INDEX audit_log_tenant_at_idx ON audit_log (tenant_id, at DESC);
CREATE INDEX audit_log_entity_idx ON audit_log (tenant_id, entity, entity_id);
CREATE TABLE audit_log_default PARTITION OF audit_log DEFAULT;

-- ---------------------------------------------------------------- partition maintenance
CREATE OR REPLACE FUNCTION ensure_month_partition(parent text, month date) RETURNS void AS $$
DECLARE
  start_d date := date_trunc('month', month)::date;
  end_d   date := (date_trunc('month', month) + interval '1 month')::date;
  pname   text := format('%s_%s', parent, to_char(start_d, 'YYYYMM'));
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_class WHERE relname = pname) THEN
    EXECUTE format('CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)', pname, parent, start_d, end_d);
  END IF;
END $$ LANGUAGE plpgsql;

SELECT ensure_month_partition(t, m)
FROM unnest(ARRAY['watch_events', 'answers', 'deliveries', 'audit_log']) t,
     unnest(ARRAY[CURRENT_DATE, (CURRENT_DATE + interval '1 month')::date]) m;

-- ---------------------------------------------------------------- row-level security
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY[
    'users','groups','group_members','devices','otp_challenges','refresh_tokens','media_assets',
    'topics','topic_assignments','sources','source_extractions','source_moments','generation_jobs','generation_steps',
    'reels','reel_variants','questions','approvals','reel_audiences',
    'daily_plans','daily_plan_items','memory_states','watch_events','answers','saved_reels','reel_requests',
    'user_stats','user_topic_readiness','daily_topic_stats','deliveries','whatsapp_messages','integrations','exports','audit_log'
  ] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format(
      'CREATE POLICY %I ON %I USING (tenant_id = current_setting(''app.tenant_id'', true)::uuid) WITH CHECK (tenant_id = current_setting(''app.tenant_id'', true)::uuid)',
      t || '_tenant_isolation', t);
  END LOOP;
END $$;

-- tenants and jobs are not tenant-scoped rows: readable by the app role, written by the app in explicit code paths.
