CREATE TABLE avaliacoes_saude (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    missao_id TEXT,

    timestamp_utc TEXT NOT NULL,

    nivel_risco TEXT NOT NULL CHECK (
        nivel_risco IN (
            'NORMAL',
            'ATENCAO',
            'ALERTA',
            'CRITICO'
        )
    ),

    resumo TEXT NOT NULL,

    indicadores_analisados_json TEXT NOT NULL,

    fatores_contextuais_json TEXT,

    explicacao TEXT,
    orientacao TEXT,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    CHECK (json_valid(indicadores_analisados_json) = 1),

    CHECK (
        fatores_contextuais_json IS NULL
        OR json_valid(fatores_contextuais_json) = 1
    )
);

CREATE INDEX idx_avaliacoes_astro_tempo
ON avaliacoes_saude(
    astronauta_id,
    timestamp_utc
);