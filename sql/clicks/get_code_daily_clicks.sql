SELECT DATE(clicked_at) AS date, COUNT(*)  count
FROM clicks
WHERE code = ($1)
GROUP BY DATE(clicked_at)
ORDER BY DATE(clicked_at);
