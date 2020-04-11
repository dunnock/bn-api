ALTER TABLE events
    -- the additional_json is a json object of {
	--   cover_image_url,
    --   video_url,
    --   top_line_info,
    --   additional_info,
    --   promo_image_url,
	-- }
    ADD COLUMN additional_json JSONB NOT NULL default '{}'::jsonb;

UPDATE events
SET additional_json = row_to_json((SELECT jsn FROM (SELECT cover_image_url, video_url, top_line_info, additional_info, promo_image_url) jsn))

-- keep old columns in place for some time in production
