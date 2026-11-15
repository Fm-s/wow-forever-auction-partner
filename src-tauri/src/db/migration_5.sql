-- Houses are now region + realm qualified (key = "{region}-{auction house id}").
-- Everything collected so far was PvP US: Alliance 110, Horde 113, Blackwater 116.
UPDATE feed_snapshots SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE daily_rollups SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE daily_coverage SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE source_status SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
-- Installs that already collected were set up before setup existed: keep them collecting (PvP US)
-- instead of idling until someone opens the window. New installs still see setup.
INSERT INTO settings(key, value) SELECT 'app', '{}'
    WHERE NOT EXISTS (SELECT 1 FROM settings WHERE key = 'app') AND EXISTS (SELECT 1 FROM feed_snapshots);
UPDATE settings SET value = json_set(value, '$.setupDone', json('true'))
    WHERE key = 'app' AND json_valid(value) AND EXISTS (SELECT 1 FROM feed_snapshots);
