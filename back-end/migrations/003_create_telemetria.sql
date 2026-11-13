CREATE TABLE telemetria_fisiologica (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    missao_id TEXT,

    timestamp_utc TEXT NOT NULL,

    frequencia_cardiaca_bpm REAL,
    spo2_percentual REAL,

    temperatura_corporal_c REAL,
    temperatura_pele_c REAL,

    taxa_respiratoria_rpm REAL,
    hrv_ms REAL,

    pressao_sistolica_mmhg REAL,
    pressao_diastolica_mmhg REAL,

    glicose_mg_dl REAL,

    origem_medicao TEXT CHECK (
        origem_medicao IN (
            'WEARABLE',
            'MANUAL',
            'IMPORTADO'
        )
    ),

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id)
);

CREATE INDEX idx_telemetria_astro_tempo
ON telemetria_fisiologica(
    astronauta_id,
    timestamp_utc
);

CREATE INDEX idx_telemetria_missao_tempo
ON telemetria_fisiologica(
    missao_id,
    timestamp_utc
);

CREATE TABLE registros_diarios (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    data_registro TEXT NOT NULL,

    horas_sono_total REAL,
    qualidade_sono_score INTEGER,

    exercicio_resistencia_min INTEGER,
    exercicio_aerobico_min INTEGER,

    ingestao_agua_ml INTEGER,
    calorias_consumidas INTEGER,

    score_teste_cognitivo REAL,

    nivel_estresse_subjetivo INTEGER,
    humor_score INTEGER,
    fadiga_score INTEGER,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    UNIQUE (astronauta_id, data_registro),

    CHECK (
        qualidade_sono_score IS NULL
        OR qualidade_sono_score BETWEEN 1 AND 10
    ),

    CHECK (
        nivel_estresse_subjetivo IS NULL
        OR nivel_estresse_subjetivo BETWEEN 1 AND 10
    ),

    CHECK (
        humor_score IS NULL
        OR humor_score BETWEEN 1 AND 10
    ),

    CHECK (
        fadiga_score IS NULL
        OR fadiga_score BETWEEN 1 AND 10
    )
);

CREATE INDEX idx_registros_diarios_astro_data
ON registros_diarios(
    astronauta_id,
    data_registro
);

CREATE TABLE baseline_fisiologico (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,

    indicador TEXT NOT NULL,

    fase_missao TEXT NOT NULL CHECK (
        fase_missao IN (
            'PRE_VOO',
            'EM_VOO',
            'POS_VOO'
        )
    ),

    missao_id TEXT,

    valor_referencia REAL NOT NULL,
    desvio_padrao REAL,

    periodo_inicio TEXT,
    periodo_fim TEXT,

    quantidade_amostras INTEGER,

    atualizado_em TEXT NOT NULL,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (
        astronauta_id,
        indicador,
        fase_missao,
        missao_id
    )
);