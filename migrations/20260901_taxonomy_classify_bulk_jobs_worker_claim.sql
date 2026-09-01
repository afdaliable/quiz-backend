-- Both the NAS and PC backend instances run the same
-- resume_orphaned_bulk_classify_jobs startup check against the same
-- production DB. Confirmed live: NAS and PC redeployed at different
-- times, both saw the same orphaned job (status='running') and both
-- resumed it concurrently -- doubled AI/9router load and processed
-- counts overshooting total (two workers each re-resolving "still
-- uncategorized" and incrementing the same job row).
--
-- worker_hostname + heartbeat_at are a claim token with a liveness check
-- (not a one-shot flag -- a job legitimately gets resumed many times over
-- its life on a single instance too, every time that instance itself
-- restarts, so "already claimed once" can't mean "never claimable
-- again"). The active worker touches heartbeat_at on every processed item
-- (see run_bulk_classify_job); resume only claims a job whose heartbeat
-- is stale (>2 min), which is true both for "my own process died" and
-- for "the other instance's process died", but false while any instance
-- is genuinely still working it. InnoDB row locking serializes concurrent
-- UPDATEs to the same row, so only one of two simultaneous claim attempts
-- can win.
ALTER TABLE dbquizapp.taxonomy_classify_bulk_jobs
    ADD COLUMN worker_hostname VARCHAR(255) NULL,
    ADD COLUMN heartbeat_at TIMESTAMP NULL;
