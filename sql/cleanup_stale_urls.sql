DELETE FROM urls u WHERE NOT EXISTS (
  SELECT 1 FROM clicks c
  WHERE c.code = u.code
  AND c.clicked_at > NOW() - make_interval(days => $1) 
);

