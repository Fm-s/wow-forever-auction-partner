-- Houses are now region + realm qualified (key = "{region}-{auction house id}").
-- Everything collected so far was PvP US: Alliance 110, Horde 113, Blackwater 116.
UPDATE feed_snapshots SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE daily_rollups SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE daily_coverage SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
UPDATE source_status SET house = CASE house WHEN 'alliance' THEN 'us-110' WHEN 'horde' THEN 'us-113' WHEN 'blackwater' THEN 'us-116' ELSE house END;
