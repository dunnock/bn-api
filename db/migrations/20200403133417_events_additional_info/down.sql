UPDATE events 
SET 
	cover_image_url = additional_json::json#>>'{cover_image_url}',
	video_url = additional_json::json#>>'{video_url}',
	top_line_info = additional_json::json#>>'{top_line_info}',
	additional_info = additional_json::json#>>'{additional_info}',
	promo_image_url = additional_json::json#>>'{promo_image_url}'

ALTER TABLE events
    DROP COLUMN additional_json;
