-- NASA SPACE APPS — CONSULTAS SQLITE
-- Baseado em "Documentação do Banco de Dados" e "Queries".
-- Parâmetros no formato :nome_parametro.
-- Consultas somente de leitura (SELECT). Ajuste os parâmetros na aplicação.

-- =========================================================
-- 1. CONSULTAS BÁSICAS POR ENTIDADE
-- =========================================================

-- 1.1 Astronauta por ID
SELECT *
FROM astronautas
WHERE id = :astronauta_id
  AND deleted_at IS NULL;

-- 1.2 Listar astronautas
SELECT *
FROM astronautas
WHERE deleted_at IS NULL
ORDER BY nome ASC
LIMIT :limite OFFSET :deslocamento;

-- 1.3 Missão por ID
SELECT *
FROM missoes
WHERE id = :missao_id
  AND deleted_at IS NULL;

-- 1.4 Listar missões
SELECT *
FROM missoes
WHERE deleted_at IS NULL
ORDER BY data_lancamento DESC
LIMIT :limite OFFSET :deslocamento;

-- 1.5 Participações do astronauta
SELECT
    pm.id,
    pm.astronauta_id,
    pm.missao_id,
    m.nome_missao,
    pm.funcao
FROM participacao_missao pm
INNER JOIN missoes m ON m.id = pm.missao_id
WHERE pm.astronauta_id = :astronauta_id
  AND pm.deleted_at IS NULL
  AND m.deleted_at IS NULL
ORDER BY m.data_lancamento DESC;

-- 1.6 Telemetria fisiológica por ID
SELECT *
FROM telemetria_fisiologica
WHERE id = :registro_id
  AND astronauta_id = :astronauta_id
  AND deleted_at IS NULL;

-- 1.7 Registros diários por ID
SELECT *
FROM registros_diarios
WHERE id = :registro_id
  AND astronauta_id = :astronauta_id
  AND deleted_at IS NULL;

-- 1.8 Módulos de uma missão
SELECT
    mn.id,
    mn.nome,
    mn.missao_id,
    m.nome_missao
FROM modulos_nave mn
INNER JOIN missoes m ON m.id = mn.missao_id
WHERE mn.missao_id = :missao_id
  AND mn.deleted_at IS NULL
  AND m.deleted_at IS NULL
ORDER BY mn.nome ASC;

-- 1.9 Leituras ambientais por ID
SELECT
    an.*,
    mn.nome AS nome_modulo,
    mn.missao_id
FROM ambiente_nave an
INNER JOIN modulos_nave mn ON mn.id = an.modulo_id
WHERE an.id = :registro_id
  AND mn.missao_id = :missao_id
  AND an.deleted_at IS NULL
  AND mn.deleted_at IS NULL;

-- 1.10 Presenças do astronauta
SELECT
    pm.id,
    pm.inicio_utc,
    pm.fim_utc,
    mn.id AS modulo_id,
    mn.nome AS nome_modulo
FROM presenca_modulo pm
INNER JOIN modulos_nave mn ON mn.id = pm.modulo_id
WHERE pm.astronauta_id = :astronauta_id
  AND mn.missao_id = :missao_id
  AND pm.deleted_at IS NULL
  AND mn.deleted_at IS NULL
ORDER BY pm.inicio_utc DESC;

-- 1.11 Exames clínicos do astronauta
SELECT *
FROM exames_clinicos
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND deleted_at IS NULL
ORDER BY data_coleta DESC;

-- 1.12 Baselines do astronauta
SELECT *
FROM baseline_fisiologico
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND deleted_at IS NULL
ORDER BY indicador, fase_missao;

-- 1.13 Avaliações do astronauta
SELECT *
FROM avaliacoes_saude
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND deleted_at IS NULL
ORDER BY timestamp_utc DESC;


-- =========================================================
-- 2. COMPARAÇÃO: TELEMETRIA ATUAL x BASELINE
-- =========================================================
-- Retorna cada indicador fisiológico com horário, valor atual,
-- valor de referência e variação percentual.
-- :fase_missao deve ser PRE_VOO, EM_VOO ou POS_VOO.
-- O INNER JOIN mantém somente indicadores com baseline correspondente.
-- Os nomes dos indicadores devem coincidir com os cadastrados no baseline.

SELECT
    t.id AS telemetria_id,
    t.timestamp_utc,
    t.astronauta_id,
    t.missao_id,
    dados.indicador,
    dados.valor_atual,
    b.valor_referencia,
    b.desvio_padrao,
    dados.valor_atual - b.valor_referencia AS diferenca,
    CASE
        WHEN b.valor_referencia = 0 THEN NULL
        ELSE ((dados.valor_atual - b.valor_referencia) / b.valor_referencia) * 100
    END AS variacao_percentual
FROM telemetria_fisiologica t
INNER JOIN (
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'FREQUENCIA_CARDIACA' AS indicador,
           frequencia_cardiaca_bpm AS valor_atual
    FROM telemetria_fisiologica
    WHERE frequencia_cardiaca_bpm IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'SPO2', spo2_percentual
    FROM telemetria_fisiologica
    WHERE spo2_percentual IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'TEMPERATURA_CORPORAL', temperatura_corporal_c
    FROM telemetria_fisiologica
    WHERE temperatura_corporal_c IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'TEMPERATURA_PELE', temperatura_pele_c
    FROM telemetria_fisiologica
    WHERE temperatura_pele_c IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'TAXA_RESPIRATORIA', taxa_respiratoria_rpm
    FROM telemetria_fisiologica
    WHERE taxa_respiratoria_rpm IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'HRV', hrv_ms
    FROM telemetria_fisiologica
    WHERE hrv_ms IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'PRESSAO_SISTOLICA', pressao_sistolica_mmhg
    FROM telemetria_fisiologica
    WHERE pressao_sistolica_mmhg IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'PRESSAO_DIASTOLICA', pressao_diastolica_mmhg
    FROM telemetria_fisiologica
    WHERE pressao_diastolica_mmhg IS NOT NULL

    UNION ALL
    SELECT id, astronauta_id, missao_id, timestamp_utc,
           'GLICOSE', glicose_mg_dl
    FROM telemetria_fisiologica
    WHERE glicose_mg_dl IS NOT NULL
) dados
    ON dados.id = t.id
INNER JOIN baseline_fisiologico b
    ON b.astronauta_id = t.astronauta_id
   AND b.indicador = dados.indicador
   AND b.fase_missao = :fase_missao
   AND (
        b.missao_id = t.missao_id
        OR (b.missao_id IS NULL AND t.missao_id IS NULL)
   )
WHERE t.astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR t.missao_id = :missao_id)
  AND t.deleted_at IS NULL
  AND b.deleted_at IS NULL
  AND b.astronauta_id = :astronauta_id
ORDER BY t.timestamp_utc ASC, dados.indicador ASC;


-- =========================================================
-- 3. CONTEXTO DA MEDIÇÃO
-- =========================================================

-- 3.1 Registro diário correspondente à data da medição
SELECT
    t.id AS telemetria_id,
    t.timestamp_utc,
    rd.id AS registro_diario_id,
    rd.data_registro,
    rd.horas_sono_total,
    rd.qualidade_sono_score,
    rd.exercicio_resistencia_min,
    rd.exercicio_aerobico_min,
    rd.ingestao_agua_ml,
    rd.calorias_consumidas,
    rd.score_teste_cognitivo,
    rd.nivel_estresse_subjetivo,
    rd.humor_score,
    rd.fadiga_score
FROM telemetria_fisiologica t
LEFT JOIN registros_diarios rd
    ON rd.astronauta_id = t.astronauta_id
   AND rd.data_registro = substr(t.timestamp_utc, 1, 10)
   AND rd.deleted_at IS NULL
WHERE t.id = :telemetria_id
  AND t.astronauta_id = :astronauta_id
  AND t.deleted_at IS NULL;

-- 3.2 Módulo ocupado no momento da medição
SELECT
    t.id AS telemetria_id,
    t.timestamp_utc,
    pm.id AS presenca_id,
    pm.inicio_utc,
    pm.fim_utc,
    mn.id AS modulo_id,
    mn.nome AS nome_modulo
FROM telemetria_fisiologica t
LEFT JOIN presenca_modulo pm
    ON pm.astronauta_id = t.astronauta_id
   AND pm.inicio_utc <= t.timestamp_utc
   AND (pm.fim_utc IS NULL OR pm.fim_utc >= t.timestamp_utc)
   AND pm.deleted_at IS NULL
LEFT JOIN modulos_nave mn
    ON mn.id = pm.modulo_id
   AND mn.deleted_at IS NULL
WHERE t.id = :telemetria_id
  AND t.astronauta_id = :astronauta_id
  AND t.deleted_at IS NULL;

-- 3.3 Leitura ambiental mais próxima da medição fisiológica
-- A diferença temporal é apresentada em segundos.
SELECT
    t.id AS telemetria_id,
    t.timestamp_utc AS horario_fisiologico,
    mn.id AS modulo_id,
    mn.nome AS nome_modulo,
    an.id AS ambiente_id,
    an.timestamp_utc AS horario_ambiental,
    an.co2_ppm,
    an.radiacao_msv_hora,
    an.temperatura_cabine_c,
    an.pressao_kpa,
    ABS(
        (julianday(an.timestamp_utc) - julianday(t.timestamp_utc)) * 86400
    ) AS diferenca_segundos
FROM telemetria_fisiologica t
INNER JOIN presenca_modulo pm
    ON pm.astronauta_id = t.astronauta_id
   AND pm.inicio_utc <= t.timestamp_utc
   AND (pm.fim_utc IS NULL OR pm.fim_utc >= t.timestamp_utc)
   AND pm.deleted_at IS NULL
INNER JOIN modulos_nave mn
    ON mn.id = pm.modulo_id
   AND mn.deleted_at IS NULL
LEFT JOIN ambiente_nave an
    ON an.modulo_id = mn.id
   AND an.deleted_at IS NULL
WHERE t.id = :telemetria_id
  AND t.astronauta_id = :astronauta_id
  AND t.deleted_at IS NULL
ORDER BY ABS(julianday(an.timestamp_utc) - julianday(t.timestamp_utc)) ASC
LIMIT 1;

-- 3.4 Presenças e leituras ambientais em um intervalo
SELECT
    pm.id AS presenca_id,
    pm.astronauta_id,
    pm.inicio_utc,
    pm.fim_utc,
    mn.id AS modulo_id,
    mn.nome AS nome_modulo,
    an.id AS ambiente_id,
    an.timestamp_utc AS horario_ambiental,
    an.co2_ppm,
    an.radiacao_msv_hora,
    an.temperatura_cabine_c,
    an.pressao_kpa
FROM presenca_modulo pm
INNER JOIN modulos_nave mn ON mn.id = pm.modulo_id
LEFT JOIN ambiente_nave an
    ON an.modulo_id = mn.id
   AND an.timestamp_utc BETWEEN :inicio_utc AND :fim_utc
   AND an.deleted_at IS NULL
WHERE pm.astronauta_id = :astronauta_id
  AND mn.missao_id = :missao_id
  AND pm.inicio_utc <= :fim_utc
  AND (pm.fim_utc IS NULL OR pm.fim_utc >= :inicio_utc)
  AND pm.deleted_at IS NULL
  AND mn.deleted_at IS NULL
ORDER BY an.timestamp_utc ASC;

-- 3.5 Contexto consolidado de uma medição
-- Reúne telemetria, rotina diária, presença e leitura ambiental mais próxima.
SELECT
    t.id AS telemetria_id,
    t.timestamp_utc,
    t.frequencia_cardiaca_bpm,
    t.spo2_percentual,
    t.temperatura_corporal_c,
    t.temperatura_pele_c,
    t.taxa_respiratoria_rpm,
    t.hrv_ms,
    t.pressao_sistolica_mmhg,
    t.pressao_diastolica_mmhg,
    t.glicose_mg_dl,
    rd.horas_sono_total,
    rd.qualidade_sono_score,
    rd.exercicio_resistencia_min,
    rd.exercicio_aerobico_min,
    rd.ingestao_agua_ml,
    rd.nivel_estresse_subjetivo,
    rd.humor_score,
    rd.fadiga_score,
    mn.nome AS nome_modulo,
    an.co2_ppm,
    an.radiacao_msv_hora,
    an.temperatura_cabine_c,
    an.pressao_kpa,
    an.timestamp_utc AS horario_ambiental,
    ABS((julianday(an.timestamp_utc) - julianday(t.timestamp_utc)) * 86400)
        AS diferenca_ambiental_segundos
FROM telemetria_fisiologica t
LEFT JOIN registros_diarios rd
    ON rd.astronauta_id = t.astronauta_id
   AND rd.data_registro = substr(t.timestamp_utc, 1, 10)
   AND rd.deleted_at IS NULL
LEFT JOIN presenca_modulo pm
    ON pm.astronauta_id = t.astronauta_id
   AND pm.inicio_utc <= t.timestamp_utc
   AND (pm.fim_utc IS NULL OR pm.fim_utc >= t.timestamp_utc)
   AND pm.deleted_at IS NULL
LEFT JOIN modulos_nave mn
    ON mn.id = pm.modulo_id
   AND mn.deleted_at IS NULL
LEFT JOIN ambiente_nave an
    ON an.id = (
        SELECT an2.id
        FROM ambiente_nave an2
        WHERE an2.modulo_id = mn.id
          AND an2.deleted_at IS NULL
        ORDER BY ABS(julianday(an2.timestamp_utc) - julianday(t.timestamp_utc))
        LIMIT 1
    )
WHERE t.id = :telemetria_id
  AND t.astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR t.missao_id = :missao_id)
  AND t.deleted_at IS NULL;


-- =========================================================
-- 4. SÉRIES PARA MONITORAMENTO E GRÁFICOS
-- =========================================================

-- 4.1 Série de BPM
SELECT id, timestamp_utc, frequencia_cardiaca_bpm
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND frequencia_cardiaca_bpm IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.2 Série de SpO2
SELECT id, timestamp_utc, spo2_percentual
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND spo2_percentual IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.3 Série de temperatura corporal
SELECT id, timestamp_utc, temperatura_corporal_c
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND temperatura_corporal_c IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.4 Série de temperatura da pele
SELECT id, timestamp_utc, temperatura_pele_c
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND temperatura_pele_c IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.5 Série respiratória
SELECT id, timestamp_utc, taxa_respiratoria_rpm
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND taxa_respiratoria_rpm IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.6 Série de HRV
SELECT id, timestamp_utc, hrv_ms
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND hrv_ms IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.7 Série de pressão arterial
SELECT id, timestamp_utc, pressao_sistolica_mmhg, pressao_diastolica_mmhg
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND (pressao_sistolica_mmhg IS NOT NULL OR pressao_diastolica_mmhg IS NOT NULL)
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.8 Série de glicose
SELECT id, timestamp_utc, glicose_mg_dl
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND glicose_mg_dl IS NOT NULL
  AND deleted_at IS NULL
ORDER BY timestamp_utc ASC;

-- 4.9 Série ambiental por módulo
SELECT
    an.id,
    an.timestamp_utc,
    mn.id AS modulo_id,
    mn.nome AS nome_modulo,
    an.co2_ppm,
    an.radiacao_msv_hora,
    an.temperatura_cabine_c,
    an.pressao_kpa
FROM ambiente_nave an
INNER JOIN modulos_nave mn ON mn.id = an.modulo_id
WHERE mn.missao_id = :missao_id
  AND an.timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND an.deleted_at IS NULL
  AND mn.deleted_at IS NULL
ORDER BY mn.nome ASC, an.timestamp_utc ASC;


-- =========================================================
-- 5. RESUMOS DO PERÍODO
-- =========================================================
-- Cada indicador é resumido separadamente para não misturar unidades.

-- 5.1 Resumo de BPM
SELECT
    MIN(frequencia_cardiaca_bpm) AS minimo,
    MAX(frequencia_cardiaca_bpm) AS maximo,
    AVG(frequencia_cardiaca_bpm) AS media,
    COUNT(frequencia_cardiaca_bpm) AS quantidade_amostras,
    MIN(timestamp_utc) AS primeira_medicao,
    MAX(timestamp_utc) AS ultima_medicao
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND deleted_at IS NULL;

-- 5.2 Resumo de SpO2
SELECT
    MIN(spo2_percentual) AS minimo,
    MAX(spo2_percentual) AS maximo,
    AVG(spo2_percentual) AS media,
    COUNT(spo2_percentual) AS quantidade_amostras,
    MIN(timestamp_utc) AS primeira_medicao,
    MAX(timestamp_utc) AS ultima_medicao
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND deleted_at IS NULL;

-- 5.3 Resumo dos demais indicadores fisiológicos
SELECT
    MIN(temperatura_corporal_c) AS temperatura_corporal_min,
    MAX(temperatura_corporal_c) AS temperatura_corporal_max,
    AVG(temperatura_corporal_c) AS temperatura_corporal_media,
    COUNT(temperatura_corporal_c) AS temperatura_corporal_amostras,
    MIN(temperatura_pele_c) AS temperatura_pele_min,
    MAX(temperatura_pele_c) AS temperatura_pele_max,
    AVG(temperatura_pele_c) AS temperatura_pele_media,
    COUNT(temperatura_pele_c) AS temperatura_pele_amostras,
    MIN(taxa_respiratoria_rpm) AS respiracao_min,
    MAX(taxa_respiratoria_rpm) AS respiracao_max,
    AVG(taxa_respiratoria_rpm) AS respiracao_media,
    COUNT(taxa_respiratoria_rpm) AS respiracao_amostras,
    MIN(hrv_ms) AS hrv_min,
    MAX(hrv_ms) AS hrv_max,
    AVG(hrv_ms) AS hrv_media,
    COUNT(hrv_ms) AS hrv_amostras,
    MIN(pressao_sistolica_mmhg) AS sistolica_min,
    MAX(pressao_sistolica_mmhg) AS sistolica_max,
    AVG(pressao_sistolica_mmhg) AS sistolica_media,
    COUNT(pressao_sistolica_mmhg) AS sistolica_amostras,
    MIN(pressao_diastolica_mmhg) AS diastolica_min,
    MAX(pressao_diastolica_mmhg) AS diastolica_max,
    AVG(pressao_diastolica_mmhg) AS diastolica_media,
    COUNT(pressao_diastolica_mmhg) AS diastolica_amostras,
    MIN(glicose_mg_dl) AS glicose_min,
    MAX(glicose_mg_dl) AS glicose_max,
    AVG(glicose_mg_dl) AS glicose_media,
    COUNT(glicose_mg_dl) AS glicose_amostras
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND deleted_at IS NULL;

-- 5.4 Resumo diário da telemetria (média por dia)
SELECT
    substr(timestamp_utc, 1, 10) AS dia,
    AVG(frequencia_cardiaca_bpm) AS bpm_medio,
    AVG(spo2_percentual) AS spo2_media,
    AVG(temperatura_corporal_c) AS temperatura_corporal_media,
    AVG(temperatura_pele_c) AS temperatura_pele_media,
    AVG(taxa_respiratoria_rpm) AS respiracao_media,
    AVG(hrv_ms) AS hrv_media,
    AVG(pressao_sistolica_mmhg) AS sistolica_media,
    AVG(pressao_diastolica_mmhg) AS diastolica_media,
    AVG(glicose_mg_dl) AS glicose_media,
    COUNT(*) AS total_registros
FROM telemetria_fisiologica
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND deleted_at IS NULL
GROUP BY substr(timestamp_utc, 1, 10)
ORDER BY dia ASC;

-- 5.5 Resumo diário dos registros de rotina
SELECT
    data_registro,
    horas_sono_total,
    qualidade_sono_score,
    exercicio_resistencia_min,
    exercicio_aerobico_min,
    ingestao_agua_ml,
    calorias_consumidas,
    score_teste_cognitivo,
    nivel_estresse_subjetivo,
    humor_score,
    fadiga_score
FROM registros_diarios
WHERE astronauta_id = :astronauta_id
  AND data_registro BETWEEN :data_inicio AND :data_fim
  AND deleted_at IS NULL
ORDER BY data_registro ASC;

-- 5.6 Resumo ambiental por módulo no período
SELECT
    mn.id AS modulo_id,
    mn.nome AS nome_modulo,
    MIN(an.co2_ppm) AS co2_min,
    MAX(an.co2_ppm) AS co2_max,
    AVG(an.co2_ppm) AS co2_media,
    COUNT(an.co2_ppm) AS co2_amostras,
    MIN(an.radiacao_msv_hora) AS radiacao_min,
    MAX(an.radiacao_msv_hora) AS radiacao_max,
    AVG(an.radiacao_msv_hora) AS radiacao_media,
    MIN(an.temperatura_cabine_c) AS temperatura_min,
    MAX(an.temperatura_cabine_c) AS temperatura_max,
    AVG(an.temperatura_cabine_c) AS temperatura_media,
    MIN(an.pressao_kpa) AS pressao_min,
    MAX(an.pressao_kpa) AS pressao_max,
    AVG(an.pressao_kpa) AS pressao_media,
    COUNT(*) AS total_leituras
FROM ambiente_nave an
INNER JOIN modulos_nave mn ON mn.id = an.modulo_id
WHERE mn.missao_id = :missao_id
  AND an.timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND an.deleted_at IS NULL
  AND mn.deleted_at IS NULL
GROUP BY mn.id, mn.nome
ORDER BY mn.nome ASC;


-- =========================================================
-- 6. AVALIAÇÕES DE SAÚDE
-- =========================================================

-- 6.1 Histórico de avaliações (mais recentes primeiro)
SELECT id, timestamp_utc, nivel_risco, resumo
FROM avaliacoes_saude
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND deleted_at IS NULL
ORDER BY timestamp_utc DESC
LIMIT :limite OFFSET :deslocamento;

-- 6.2 Dados completos de uma avaliação
SELECT *
FROM avaliacoes_saude
WHERE id = :avaliacao_id
  AND astronauta_id = :astronauta_id
  AND deleted_at IS NULL;

-- 6.3 Avaliações por indicador
-- A busca é textual porque os indicadores ficam armazenados em JSON.
SELECT
    id,
    timestamp_utc,
    nivel_risco,
    resumo,
    indicadores_analisados_json
FROM avaliacoes_saude
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND indicadores_analisados_json LIKE '%' || :indicador || '%'
  AND timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND deleted_at IS NULL
ORDER BY timestamp_utc DESC;

-- 6.4 Evidências disponíveis de uma avaliação
-- Retorna a avaliação e as medições fisiológicas do intervalo informado.
-- :inicio_utc e :fim_utc devem corresponder ao período usado na avaliação.
SELECT
    a.id AS avaliacao_id,
    a.timestamp_utc AS horario_avaliacao,
    a.nivel_risco,
    a.resumo,
    t.id AS telemetria_id,
    t.timestamp_utc AS horario_medicao,
    t.frequencia_cardiaca_bpm,
    t.spo2_percentual,
    t.temperatura_corporal_c,
    t.temperatura_pele_c,
    t.taxa_respiratoria_rpm,
    t.hrv_ms,
    t.pressao_sistolica_mmhg,
    t.pressao_diastolica_mmhg,
    t.glicose_mg_dl
FROM avaliacoes_saude a
LEFT JOIN telemetria_fisiologica t
    ON t.astronauta_id = a.astronauta_id
   AND t.timestamp_utc BETWEEN :inicio_utc AND :fim_utc
   AND t.deleted_at IS NULL
WHERE a.id = :avaliacao_id
  AND a.astronauta_id = :astronauta_id
  AND a.deleted_at IS NULL
ORDER BY t.timestamp_utc ASC;

-- 6.5 Histórico fisiológico para explicação
SELECT
    t.id,
    t.timestamp_utc,
    t.origem_medicao,
    t.frequencia_cardiaca_bpm,
    t.spo2_percentual,
    t.temperatura_corporal_c,
    t.temperatura_pele_c,
    t.taxa_respiratoria_rpm,
    t.hrv_ms,
    t.pressao_sistolica_mmhg,
    t.pressao_diastolica_mmhg,
    t.glicose_mg_dl
FROM telemetria_fisiologica t
WHERE t.astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR t.missao_id = :missao_id)
  AND t.timestamp_utc BETWEEN :inicio_utc AND :fim_utc
  AND t.deleted_at IS NULL
ORDER BY t.timestamp_utc ASC;

-- 6.6 Exames por categoria, fase e período
SELECT
    id,
    missao_id,
    fase_missao,
    data_coleta,
    categoria,
    resultados_json,
    observacoes_medicas
FROM exames_clinicos
WHERE astronauta_id = :astronauta_id
  AND (:missao_id IS NULL OR missao_id = :missao_id)
  AND (:categoria IS NULL OR categoria = :categoria)
  AND (:fase_missao IS NULL OR fase_missao = :fase_missao)
  AND data_coleta BETWEEN :data_inicio AND :data_fim
  AND deleted_at IS NULL
ORDER BY data_coleta DESC;


-- =========================================================
-- 7. PAINEL ATUAL
-- =========================================================
-- Um registro com os últimos valores não nulos de cada indicador.
-- Cada subconsulta é filtrada pelo astronauta e pela missão.

SELECT
    (SELECT frequencia_cardiaca_bpm
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND frequencia_cardiaca_bpm IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS bpm_atual,

    (SELECT timestamp_utc
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND frequencia_cardiaca_bpm IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS bpm_horario,

    (SELECT spo2_percentual
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND spo2_percentual IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS spo2_atual,

    (SELECT timestamp_utc
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND spo2_percentual IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS spo2_horario,

    (SELECT temperatura_corporal_c
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND temperatura_corporal_c IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS temperatura_corporal_atual,

    (SELECT temperatura_pele_c
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND temperatura_pele_c IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS temperatura_pele_atual,

    (SELECT taxa_respiratoria_rpm
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND taxa_respiratoria_rpm IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS respiracao_atual,

    (SELECT hrv_ms
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND hrv_ms IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS hrv_atual,

    (SELECT pressao_sistolica_mmhg
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND pressao_sistolica_mmhg IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS pressao_sistolica_atual,

    (SELECT pressao_diastolica_mmhg
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND pressao_diastolica_mmhg IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS pressao_diastolica_atual,

    (SELECT glicose_mg_dl
     FROM telemetria_fisiologica
     WHERE astronauta_id = :astronauta_id
       AND (:missao_id IS NULL OR missao_id = :missao_id)
       AND glicose_mg_dl IS NOT NULL
       AND deleted_at IS NULL
     ORDER BY timestamp_utc DESC LIMIT 1) AS glicose_atual;


-- =========================================================
-- 8. OBSERVAÇÕES SOBRE ITENS QUE AINDA NÃO EXISTEM NO ESQUEMA
-- =========================================================
-- O documento Queries pede sessões de exercício com horário,
-- modalidade e intensidade. A tabela registros_diarios não possui
-- esses campos. Portanto, não há SELECT possível para esses detalhes
-- até que a tabela de sessões de exercício seja criada.
--
-- O documento Queries também pede historico_conversas_ia, mas essa
-- tabela ainda não aparece na documentação do banco. Os SELECTs
-- abaixo pressupõem que ela seja criada com os campos descritos
-- no documento Queries:
--
-- id, astronauta_id, conversa_id, papel, conteudo, criado_em
--
-- 8.1 Mensagens de uma conversa
SELECT id, astronauta_id, conversa_id, papel, conteudo, criado_em
FROM historico_conversas_ia
WHERE astronauta_id = :astronauta_id
  AND conversa_id = :conversa_id
ORDER BY criado_em ASC;

-- 8.2 Conversas com última mensagem e quantidade de mensagens
SELECT
    conversa_id,
    MAX(criado_em) AS data_ultima_mensagem,
    COUNT(*) AS quantidade_mensagens,
    (SELECT h2.conteudo
     FROM historico_conversas_ia h2
     WHERE h2.astronauta_id = h.astronauta_id
       AND h2.conversa_id = h.conversa_id
     ORDER BY h2.criado_em DESC
     LIMIT 1) AS ultima_mensagem
FROM historico_conversas_ia h
WHERE astronauta_id = :astronauta_id
GROUP BY astronauta_id, conversa_id
ORDER BY data_ultima_mensagem DESC;

-- 8.3 Buscar mensagens por termo
SELECT id, conversa_id, papel, conteudo, criado_em
FROM historico_conversas_ia
WHERE astronauta_id = :astronauta_id
  AND conteudo LIKE '%' || :termo || '%'
ORDER BY criado_em DESC;

-- 8.4 Recuperar as últimas N mensagens de uma conversa
SELECT id, conversa_id, papel, conteudo, criado_em
FROM historico_conversas_ia
WHERE astronauta_id = :astronauta_id
  AND conversa_id = :conversa_id
ORDER BY criado_em DESC
LIMIT :quantidade_mensagens;

-- 8.5 Últimas N mensagens do astronauta, independentemente da conversa
SELECT id, conversa_id, papel, conteudo, criado_em
FROM historico_conversas_ia
WHERE astronauta_id = :astronauta_id
ORDER BY criado_em DESC
LIMIT :quantidade_mensagens;


-- =========================================================
-- 9. CONSULTAS DE APOIO À SINCRONIZAÇÃO
-- =========================================================
-- A documentação define sync_queue, mas não define deleted_at
-- nessa tabela. A consulta abaixo lista operações pendentes.
SELECT
    id,
    tabela,
    registro_id,
    operacao,
    criado_em,
    tentativas,
    ultimo_erro
FROM sync_queue
ORDER BY criado_em ASC;
