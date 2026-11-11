-- AI providers configurable from the console, with a platform layer for commercials.
--
-- Resolution for each AI capability, per company: the company's own key (integrations),
-- else the platform's key (platform_integrations, then the server environment) when the
-- company's AI plan allows it and its monthly budget has room, else the development fake.
-- Every billable call lands in ai_usage with whose key paid for it.

ALTER TYPE integration_kind_t ADD VALUE IF NOT EXISTS 'ai_typesafe';

-- ---------------------------------------------------------------- platform layer
-- People who run the platform (not a company role): they manage platform keys and AI plans.
CREATE TABLE platform_operators (
  user_id    uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  granted_at timestamptz NOT NULL DEFAULT now()
);

-- One row per provider kind: the platform's own keys and prices. Not tenant-scoped, so no RLS;
-- only reached through admin transactions in the platform routes and the resolver.
CREATE TABLE platform_integrations (
  kind         text PRIMARY KEY,                -- an integration_kind_t value
  status       text NOT NULL DEFAULT 'disconnected',
  config_enc   bytea,                           -- AES-GCM encrypted JSON (keys, models, prices)
  last_error   text,
  updated_by   uuid REFERENCES users(id) ON DELETE SET NULL,
  updated_at   timestamptz NOT NULL DEFAULT now()
);

-- The company's AI plan, set by the platform operator:
-- { "platform_keys": true, "monthly_budget_usd": null | number, "markup_pct": 0..500,
--   "allowed": ["ai_claude","ai_muse","ai_typesafe", ...] }
ALTER TABLE tenants ADD COLUMN ai_plan jsonb NOT NULL DEFAULT '{}'::jsonb;

-- ---------------------------------------------------------------- metering
CREATE TABLE ai_usage (
  id            uuid NOT NULL DEFAULT uuid_generate_v7(),
  tenant_id     uuid NOT NULL REFERENCES tenants(id),
  at            timestamptz NOT NULL DEFAULT now(),
  provider      text NOT NULL,        -- integration kind: ai_claude, ai_muse, ai_typesafe, ai_higgsfield, voice, asr
  capability    text NOT NULL,        -- write | judge | render | voice | transcribe
  key_source    text NOT NULL CHECK (key_source IN ('company', 'platform')),
  model         text,
  input_tokens  bigint NOT NULL DEFAULT 0 CHECK (input_tokens >= 0),
  output_tokens bigint NOT NULL DEFAULT 0 CHECK (output_tokens >= 0),
  units         integer NOT NULL DEFAULT 0 CHECK (units >= 0),
  cost_usd      numeric(14, 6),       -- null when no price is configured for the provider
  ok            boolean NOT NULL DEFAULT true,
  job_id        uuid,
  PRIMARY KEY (id, at)
) PARTITION BY RANGE (at);
CREATE INDEX ai_usage_tenant_at_idx ON ai_usage (tenant_id, at DESC);
CREATE INDEX ai_usage_source_idx ON ai_usage (tenant_id, key_source, at);
CREATE INDEX ai_usage_at_brin ON ai_usage USING brin (at);

-- This month and next; the scheduler adds the following months like the other partitioned tables.
SELECT ensure_month_partition('ai_usage', m)
FROM unnest(ARRAY[CURRENT_DATE, (CURRENT_DATE + interval '1 month')::date]) m;

-- ---------------------------------------------------------------- AI judgements (TypeSafe Jev)
-- The quality check of a draft: the raw calibrated answers, the score code derived from them
-- and the flags reviewers see. Advisory: a person still approves every reel.
ALTER TABLE reels ADD COLUMN ai_review jsonb;
ALTER TABLE reels ADD COLUMN quality_score smallint CHECK (quality_score BETWEEN 0 AND 100);
CREATE INDEX reels_quality_idx ON reels (tenant_id, status, quality_score);

-- How relevant a published reel is to a job title, as Jev judged it. A prior for people the
-- engine knows little about yet; it fades as their own signals arrive.
CREATE TABLE reel_role_fit (
  tenant_id  uuid NOT NULL REFERENCES tenants(id),
  reel_id    uuid NOT NULL REFERENCES reels(id) ON DELETE CASCADE,
  role_key   text NOT NULL,           -- lower-cased, trimmed job title
  fit        real NOT NULL CHECK (fit BETWEEN 0 AND 1),
  model      text,
  scored_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, role_key, reel_id)
);
CREATE INDEX reel_role_fit_reel_idx ON reel_role_fit (reel_id);

-- ---------------------------------------------------------------- row-level security
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['ai_usage', 'reel_role_fit'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format(
      'CREATE POLICY %I ON %I USING (tenant_id = current_setting(''app.tenant_id'', true)::uuid) WITH CHECK (tenant_id = current_setting(''app.tenant_id'', true)::uuid)',
      t || '_tenant_isolation', t);
  END LOOP;
END $$;

-- Platform tables stay out of reach of the tenant-scoped app role.
REVOKE ALL ON platform_operators, platform_integrations FROM reelearn_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO reelearn_admin;
GRANT SELECT, INSERT, UPDATE, DELETE ON ai_usage, reel_role_fit TO reelearn_app;
