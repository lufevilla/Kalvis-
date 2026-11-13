CREATE TABLE astronautas (
    id TEXT PRIMARY KEY,
    nome TEXT NOT NULL,
    data_nascimento TEXT NOT NULL,
    tipo_sanguineo TEXT,
    peso_baseline_kg REAL,
    altura_cm REAL,

    updated_at TEXT,
    deleted_at TEXT
);