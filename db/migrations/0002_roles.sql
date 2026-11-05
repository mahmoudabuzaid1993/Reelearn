-- Reelearn · application roles
--
-- reelearn_admin: the connection role for migrations, the scheduler and cross-tenant jobs (BYPASSRLS).
-- reelearn_app:   the role every tenant-scoped transaction drops into with `SET LOCAL ROLE`
--                 (NOBYPASSRLS, not a table owner), so the FORCE'd row-level security policies apply.
--
-- Roles are cluster-wide, so creation is guarded: this migration may run against many databases
-- (every `#[sqlx::test]` database runs it) and concurrently.

DO $$
BEGIN
  BEGIN
    CREATE ROLE reelearn_app LOGIN NOBYPASSRLS NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD 'reelearn_app';
  EXCEPTION WHEN duplicate_object OR unique_violation THEN
    NULL;
  END;
  BEGIN
    CREATE ROLE reelearn_admin LOGIN BYPASSRLS NOSUPERUSER CREATEDB NOCREATEROLE PASSWORD 'reelearn_admin';
  EXCEPTION WHEN duplicate_object OR unique_violation THEN
    NULL;
  END;
END $$;

-- The admin role may drop into the app role inside a transaction.
GRANT reelearn_app TO reelearn_admin;

GRANT USAGE ON SCHEMA public TO reelearn_app, reelearn_admin;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO reelearn_app, reelearn_admin;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO reelearn_app, reelearn_admin;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO reelearn_app, reelearn_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO reelearn_app, reelearn_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO reelearn_app, reelearn_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT EXECUTE ON FUNCTIONS TO reelearn_app, reelearn_admin;

-- Partition maintenance creates new partitions at runtime; they inherit the parent's grants
-- through the default privileges above (the creating role is the migration/admin role).
