CREATE OR REPLACE FUNCTION public.convert_km(n FLOAT4, unit text)
RETURNS numeric(6,2)
LANGUAGE 'sql'
COST 100
VOLATILE
AS $BODY$
  SELECT
  CASE $2 WHEN 'km' THEN $1
          WHEN 'mi' THEN $1 / 1.60934
  END;
$BODY$;

CREATE OR REPLACE FUNCTION public.convert_celsius(n FLOAT4, unit text)
RETURNS numeric(4,1)
LANGUAGE 'sql'
COST 100
VOLATILE
AS $BODY$
  SELECT
  CASE $2 WHEN 'C' THEN $1
          WHEN 'F' THEN ($1 * 9 / 5) + 32
  END;
$BODY$;

CREATE OR REPLACE FUNCTION public.convert_tire_pressure(n FLOAT4, character varying)
RETURNS numeric(6,2)
LANGUAGE 'sql'
COST 100
VOLATILE
AS $BODY$
SELECT
CASE $2 WHEN 'bar' THEN $1
    WHEN 'psi' THEN $1 * 14.503773773
END;
$BODY$;
