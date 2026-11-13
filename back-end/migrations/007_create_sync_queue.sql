CREATE TABLE sync_queue (
    id TEXT PRIMARY KEY,

    tabela TEXT NOT NULL,
    registro_id TEXT NOT NULL,

    operacao TEXT NOT NULL CHECK (
        operacao IN ('INSERT', 'UPDATE', 'DELETE')
    ),

    criado_em TEXT NOT NULL,

    tentativas INTEGER DEFAULT 0,

    ultimo_erro TEXT,

    UNIQUE (tabela, registro_id)
);