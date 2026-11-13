CREATE TABLE missoes (
    id TEXT PRIMARY KEY,
    nome_missao TEXT NOT NULL,
    data_lancamento TEXT,
    data_retorno TEXT,

    updated_at TEXT,
    deleted_at TEXT
);

CREATE TABLE participacao_missao (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    missao_id TEXT NOT NULL,

    funcao TEXT,

    updated_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (astronauta_id, missao_id)
);