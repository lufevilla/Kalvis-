CREATE TABLE exames_clinicos (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    missao_id TEXT,

    fase_missao TEXT CHECK (
        fase_missao IN (
            'PRE_VOO',
            'EM_VOO',
            'POS_VOO'
        )
    ),

    data_coleta TEXT NOT NULL,

    categoria TEXT NOT NULL,

    resultados_json TEXT NOT NULL,

    observacoes_medicas TEXT,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    CHECK (json_valid(resultados_json) = 1)
);  