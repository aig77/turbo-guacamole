SELECT
  (SELECT COUNT(*) FROM urls) as total_urls,
  (SELECT COUNT(*) FROM clicks) as total_clicks;
