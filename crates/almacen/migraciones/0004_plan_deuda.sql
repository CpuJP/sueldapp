-- Plan de una deuda: interés (porcentaje, mensual o efectivo anual) y mes de la primera cuota (AAAA-MM).
ALTER TABLE deudas ADD COLUMN interes TEXT;
ALTER TABLE deudas ADD COLUMN interes_anual INTEGER NOT NULL DEFAULT 0;
ALTER TABLE deudas ADD COLUMN primera TEXT;
