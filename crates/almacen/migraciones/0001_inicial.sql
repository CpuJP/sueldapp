-- Esquema inicial. Montos como TEXT decimal (nunca REAL).

CREATE TABLE meses (
    id      INTEGER PRIMARY KEY,
    anio    INTEGER NOT NULL CHECK (anio BETWEEN 2000 AND 2100),
    mes     INTEGER NOT NULL CHECK (mes BETWEEN 1 AND 12),
    cerrado INTEGER NOT NULL DEFAULT 0,
    UNIQUE (anio, mes)
);

CREATE TABLE estados (
    id            INTEGER PRIMARY KEY,
    nombre        TEXT NOT NULL UNIQUE,
    cuenta_pagado INTEGER NOT NULL DEFAULT 0,
    orden         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE categorias (
    id     INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE
);

CREATE TABLE fuentes_ingreso (
    id     INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE
);

CREATE TABLE deudas (
    id      INTEGER PRIMARY KEY,
    nombre  TEXT NOT NULL,
    total   TEXT,
    ajuste  TEXT NOT NULL DEFAULT '0',
    saldada INTEGER NOT NULL DEFAULT 0,
    notas   TEXT NOT NULL DEFAULT '',
    origen  TEXT UNIQUE
);

CREATE TABLE tarjetas (
    id        INTEGER PRIMARY KEY,
    nombre    TEXT NOT NULL,
    cupo      TEXT,
    saldo     TEXT,
    dia_corte INTEGER CHECK (dia_corte BETWEEN 1 AND 31),
    dia_pago  INTEGER CHECK (dia_pago BETWEEN 1 AND 31),
    origen    TEXT UNIQUE
);

CREATE TABLE gastos (
    id              INTEGER PRIMARY KEY,
    mes_id          INTEGER NOT NULL REFERENCES meses (id) ON DELETE CASCADE,
    concepto        TEXT NOT NULL,
    monto           TEXT NOT NULL,
    estado_id       INTEGER NOT NULL REFERENCES estados (id),
    categoria_id    INTEGER REFERENCES categorias (id) ON DELETE SET NULL,
    deuda_id        INTEGER REFERENCES deudas (id) ON DELETE SET NULL,
    tarjeta_id      INTEGER REFERENCES tarjetas (id) ON DELETE SET NULL,
    desglose        TEXT,
    nota            TEXT NOT NULL DEFAULT '',
    fuera_de_bloque INTEGER NOT NULL DEFAULT 0,
    orden           INTEGER NOT NULL DEFAULT 0,
    origen          TEXT UNIQUE
);
CREATE INDEX gastos_mes ON gastos (mes_id);
CREATE INDEX gastos_deuda ON gastos (deuda_id);
CREATE INDEX gastos_tarjeta ON gastos (tarjeta_id);

CREATE TABLE ingresos (
    id        INTEGER PRIMARY KEY,
    mes_id    INTEGER NOT NULL REFERENCES meses (id) ON DELETE CASCADE,
    fuente_id INTEGER NOT NULL REFERENCES fuentes_ingreso (id),
    monto     TEXT NOT NULL,
    desglose  TEXT,
    nota      TEXT NOT NULL DEFAULT '',
    origen    TEXT UNIQUE
);
CREATE INDEX ingresos_mes ON ingresos (mes_id);

CREATE TABLE importaciones (
    id      INTEGER PRIMARY KEY,
    fecha   TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    archivo TEXT NOT NULL,
    huella  TEXT NOT NULL,
    ok      INTEGER NOT NULL,
    resumen TEXT NOT NULL,
    log     TEXT NOT NULL
);

INSERT INTO estados (nombre, cuenta_pagado, orden) VALUES
    ('Pagado', 1, 1),
    ('Pendiente', 0, 2),
    ('No', 0, 3),
    ('Aplazado', 0, 4),
    ('Sin estado', 0, 5);

INSERT INTO categorias (nombre) VALUES ('Tarjetas'), ('Préstamos'), ('Servicios'), ('Personal');

INSERT INTO fuentes_ingreso (nombre) VALUES ('Sueldo'), ('Prima'), ('Otro');
