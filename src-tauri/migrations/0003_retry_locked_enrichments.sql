UPDATE tweets
SET enrichment_status = 'pending',
    enrichment_error = NULL,
    retry_count = 0
WHERE enrichment_status = 'failed'
  AND enrichment_error LIKE '%database is locked%';
