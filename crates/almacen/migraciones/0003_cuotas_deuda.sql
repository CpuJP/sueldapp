-- Número total de cuotas de una deuda, si se conoce; opcional.
ALTER TABLE deudas ADD COLUMN cuotas INTEGER CHECK (cuotas > 0);
