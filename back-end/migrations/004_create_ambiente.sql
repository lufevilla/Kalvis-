CREATE TABLE modulos_nave (
    id TEXT PRIMARY KEY,

    missao_id TEXT NOT NULL,
    nome TEXT NOT NULL,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (missao_id, nome)
);

CREATE TABLE ambiente_nave (
    id TEXT PRIMARY KEY,

    modulo_id TEXT NOT NULL,
    timestamp_utc TEXT NOT NULL,

    co2_ppm REAL,
    radiacao_msv_hora REAL,

    temperatura_cabine_c REAL,
    pressao_kpa REAL,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (modulo_id)
        REFERENCES modulos_nave(id)
);

CREATE INDEX idx_ambiente_modulo_tempo
ON ambiente_nave(
    modulo_id,
    timestamp_utc
);

CREATE TABLE presenca_modulo (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    modulo_id TEXT NOT NULL,

    inicio_utc TEXT NOT NULL,
    fim_utc TEXT,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (modulo_id)
        REFERENCES modulos_nave(id),

    CHECK (
        fim_utc IS NULL
        OR fim_utc >= inicio_utc
    )
);

CREATE INDEX idx_presenca_astro_inicio
ON presenca_modulo(
    astronauta_id,
    inicio_utc
);