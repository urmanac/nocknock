-- The 20241010 migration declared snapshot_id VARCHAR(26), but uuid::Uuid
-- stringifies to 36 characters, so every snapshot insert failed with
-- "value too long for type character varying(26)" on databases provisioned
-- from migrations. The docker-compose init-db.sql path uses UUID and was
-- unaffected — classic schema drift between the two provisioning paths.
ALTER TABLE snapshots ALTER COLUMN snapshot_id TYPE UUID USING snapshot_id::UUID;
