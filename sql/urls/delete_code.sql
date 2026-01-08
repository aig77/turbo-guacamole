DELETE FROM urls where code = $1 RETURNING url;
