/*
ALERTAS OPERACIONAIS.

O histórico de avaliações vive em `avaliacoes_saude` (migration 006), que
registra TODA avaliação — inclusive as normais — e não possui campos de
ciclo de vida. O gerenciamento de alertas (criar/atualizar/resolver,
evitar duplicação de uma mesma condição persistente) precisa distinguir
"alerta ativo" de "avaliação do ciclo", o que não pode ser feito naquela
tabela sem corromper o histórico (a resolução usaria soft-delete em
registros que são também avaliações).

Esta migration cria, então, a tabela de alertas com ciclo de vida próprio:
um alerta é UM registro por condição (astronauta + indicadores alterados),
evolui enquanto a condição persiste (UPDATE) e é marcado RESOLVIDO quando
a condição termina. O índice único parcial garante no máximo um alerta
ATIVO por (astronauta, condicao) — deduplicação no próprio banco.
*/

CREATE TABLE alertas (
    id TEXT PRIMARY KEY,

    avaliacao_id TEXT NOT NULL,
    astronauta_id TEXT NOT NULL,
    missao_id TEXT,

    condicao TEXT NOT NULL,

    timestamp_utc TEXT NOT NULL,
    nivel_risco TEXT NOT NULL CHECK (
        nivel_risco IN ('ATENCAO', 'ALERTA', 'CRITICO')
    ),

    resumo TEXT NOT NULL,
    explicacao TEXT,
    orientacao TEXT,

    estado TEXT NOT NULL DEFAULT 'ATIVO' CHECK (
        estado IN ('ATIVO', 'RESOLVIDO')
    ),

    atualizado_em TEXT NOT NULL,
    resolvido_em TEXT,

    updated_at TEXT,
    deleted_at TEXT,

    FOREIGN KEY (avaliacao_id) REFERENCES avaliacoes_saude(id),
    FOREIGN KEY (astronauta_id) REFERENCES astronautas(id),
    FOREIGN KEY (missao_id) REFERENCES missoes(id)
);

CREATE INDEX idx_alertas_astro_estado
ON alertas(astronauta_id, estado);

CREATE UNIQUE INDEX idx_alertas_condicao_ativa
ON alertas(astronauta_id, condicao)
WHERE estado = 'ATIVO' AND deleted_at IS NULL;
