# NASA Space Apps
## Documentação do Banco de Dados

**Tecnologia:** SQLite  
**Arquitetura:** Offline-first, com sincronização posterior com servidor em nuvem.  
**Objetivo:** Armazenar, contextualizar e analisar informações fisiológicas, ambientais e comportamentais de astronautas durante missões espaciais.

---

# 1. Visão geral

O sistema vai permitir

- Monitoramento contínuo da saúde do usuário
- Comparação dos indicadores atuais com o histórico do usuário.
- Acompanhamento de sono, exercícios, cognição e bem-estar.
- Monitoramento das condições ambientais da nave.
- Guardar infos de exames clínicos e laboratoriais.
- Identificação e registro de possíveis alterações fisiológicas.
- Geração de avaliações e alertas explicáveis.
- Funcionamento offline e sincronização posterior com a Terra.

Os dados devem ser contextualizados de acordo com o astronauta, a missão e a fase da missão.

# 2. Estrutura das entidades

O banco será organizado em dez entidades principais e uma entidade complementar para identificar a localização dos astronautas dentro da nave.

| Entidade | Responsabilidade |
|---|---|
| astronautas | Informações cadastrais dos astronautas |
| missoes | Informações das missões espaciais |
| participacao_missao | Relacionamento entre astronautas e missões |
| telemetria_fisiologica | Indicadores fisiológicos coletados |
| registros_diarios | Informações diárias de rotina e bem-estar |
| modulos_nave | Identificação dos módulos da nave (tipo sala de estar, banheiro etc so q de uma nave espacial aí são outros lugares tipo módulo do laboratório do japao, outro dos EUA) |
| ambiente_nave | Dados ambientais(CO2, temperatura, pressao) dos módulos da nave |
| presenca_modulo | Histórico de localização dos astronautas |
| exames_clinicos | Resultados de exames médicos |
| baseline_fisiologico | Padrões fisiológicos(tipo BPM e outros dados que uma pessoa tem a mantém ao longo da vida) individuais |
| avaliacoes_saude | Avaliações, explicações e alertas gerados |

---

# 3. Estrutura SQL

## 3.1. Astronautas

Armazena as informações básicas dos astronautas cadastrados.

```sql
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
```

**Observações:**
- O identificador utiliza UUID gerado pela aplicação.
- O peso é uma referência inicial, podendo ser atualizado conforme novas avaliações.
- A data de nascimento utiliza o formato `YYYY-MM-DD`.

## 3.2. Missões

Armazena as informações das missões espaciais.

```sql
CREATE TABLE missoes (
    id TEXT PRIMARY KEY,
    nome_missao TEXT NOT NULL,
    data_lancamento TEXT,
    data_retorno TEXT,

    updated_at TEXT,
    deleted_at TEXT
);
```

Exemplos:
- Artemis II.
- ISS Expedition 72.

## 3.3. Participação em missões

Relaciona os astronautas às missões das quais participam.

Um astronauta pode participar de várias missões, e uma missão pode possuir vários astronautas.

```sql
CREATE TABLE participacao_missao (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    missao_id TEXT NOT NULL,

    funcao TEXT,

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (astronauta_id, missao_id),

    updated_at TEXT,
    deleted_at TEXT
);
```

---

## 3.4. Telemetria fisiológica

Armazena os indicadores fisiológicos coletados por dispositivos vestíveis, equipamentos médicos ou inserção manual.

Essa tabela representa uma das principais fontes de dados do sistema.

```sql
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

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    updated_at TEXT,
    deleted_at TEXT
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
```

**Observações:**

- Nem todos os indicadores precisam estar presentes em cada coleta.
- Os valores podem ser nulos quando determinado sensor não estiver disponível.
- O timestamp deve utilizar UTC.
- A identificação da missão permite separar os dados coletados em diferentes missões.

---

## 3.5. Registros diários

Armazena informações relacionadas à rotina, recuperação física e bem-estar psicológico e cognitivo.

Diferentemente da telemetria, esses dados podem ser preenchidos diretamente pelo astronauta.

```sql
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
    ),

    updated_at TEXT,
    deleted_at TEXT
);

CREATE INDEX idx_registros_diarios_astro_data
ON registros_diarios(
    astronauta_id,
    data_registro
);
```

**Observações:**

- A qualidade do sono utiliza escala de 1 a 10.
- Estresse, humor e fadiga também utilizam escala de 1 a 10.
- A restrição `UNIQUE` impede registros diários duplicados para o mesmo astronauta.

---

## 3.6. Módulos da nave

Armazena os módulos existentes em cada missão.

Essa separação permite relacionar os dados ambientais ao local onde foram coletados.

```sql
CREATE TABLE modulos_nave (
    id TEXT PRIMARY KEY,

    missao_id TEXT NOT NULL,
    nome TEXT NOT NULL,

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (missao_id, nome),

    updated_at TEXT,
    deleted_at TEXT
);
```

Exemplos:
- Módulo Columbus.
- Cápsula Orion.
- Módulo Destiny.

## 3.7. Telemetria ambiental

Armazena informações ambientais coletadas pelos sensores da nave.

```sql
CREATE TABLE ambiente_nave (
    id TEXT PRIMARY KEY,

    modulo_id TEXT NOT NULL,
    timestamp_utc TEXT NOT NULL,

    co2_ppm REAL,
    radiacao_msv_hora REAL,

    temperatura_cabine_c REAL,
    pressao_kpa REAL,

    FOREIGN KEY (modulo_id)
        REFERENCES modulos_nave(id),

    updated_at TEXT,
    deleted_at TEXT
);

CREATE INDEX idx_ambiente_modulo_tempo
ON ambiente_nave(
    modulo_id,
    timestamp_utc
);
```

**Objetivo:**

Permitir que o sistema considere as condições ambientais ao analisar alterações fisiológicas.

Por exemplo, um aumento da frequência cardíaca pode ser analisado juntamente com as condições de temperatura e CO₂ da cabine.

A relação temporal não deve ser interpretada automaticamente como causalidade.

## 3.8. Presença nos módulos

Registra em qual módulo da nave o astronauta estava durante determinado intervalo.

Essa tabela permite relacionar corretamente a telemetria ambiental ao contexto individual do astronauta.

```sql
CREATE TABLE presenca_modulo (
    id TEXT PRIMARY KEY,

    astronauta_id TEXT NOT NULL,
    modulo_id TEXT NOT NULL,

    inicio_utc TEXT NOT NULL,
    fim_utc TEXT,

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (modulo_id)
        REFERENCES modulos_nave(id),

    CHECK (
        fim_utc IS NULL
        OR fim_utc >= inicio_utc
    ),

    updated_at TEXT,
    deleted_at TEXT
);

CREATE INDEX idx_presenca_astro_inicio
ON presenca_modulo(
    astronauta_id,
    inicio_utc
);
```

---

## 3.9. Exames clínicos

Armazena exames laboratoriais, clínicos e avaliações especializadas.

Como os exames possuem estruturas diferentes, seus resultados serão armazenados em JSON.

```sql
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

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    CHECK (json_valid(resultados_json)),

    updated_at TEXT,
    deleted_at TEXT
);
```

**Exemplo de resultado JSON:**

Exame sanguíneo:

```json
{
    "cortisol_ug_dl": 18.5,
    "vitamina_d_ng_ml": 42.0,
    "glicose_mg_dl": 95
}
```

Exame ocular:

```json
{
    "pressao_intraocular_mmhg": 16,
    "espessura_retiniana_um": 250
}
```

Categorias previstas:
- SANGUE
- OCULAR_SANS
- DENSITOMETRIA
- OMICA
- OUTROS

---

## 3.10. Baseline fisiológico

Armazena os padrões fisiológicos individuais dos astronautas.

É uma das entidades mais importantes do projeto, pois permite identificar alterações em relação ao comportamento habitual de cada indivíduo.

```sql
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

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    UNIQUE (
        astronauta_id,
        indicador,
        fase_missao,
        missao_id
    ),

    updated_at TEXT,
    deleted_at TEXT
);
```

**Exemplo:**

| Astronauta | Indicador | Fase | Referência |
|---|---|---|---:|
| 1 | FREQUENCIA_CARDIACA | PRE_VOO | 65 |
| 1 | SPO2 | PRE_VOO | 98 |
| 1 | TEMPERATURA | PRE_VOO | 36.5 |
| 1 | HORAS_SONO | PRE_VOO | 7.5 |

O baseline deve ser calculado a partir de dados históricos confiáveis.

Não deve ser atualizado automaticamente com qualquer medição, especialmente quando houver suspeita de alteração fisiológica.

---

## 3.11. Avaliações de saúde

Armazena os resultados das análises realizadas pelo sistema.

Essa entidade representa a transformação dos dados brutos em informações compreensíveis para a tripulação.

```sql
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

    FOREIGN KEY (astronauta_id)
        REFERENCES astronautas(id),

    FOREIGN KEY (missao_id)
        REFERENCES missoes(id),

    CHECK (json_valid(indicadores_analisados_json)),

    CHECK (
        fatores_contextuais_json IS NULL
        OR json_valid(fatores_contextuais_json)
    ),

    updated_at TEXT,
    deleted_at TEXT
);

CREATE INDEX idx_avaliacoes_astro_tempo
ON avaliacoes_saude(
    astronauta_id,
    timestamp_utc
);
```

**Exemplo de avaliação:**

```json
{
    "nivel_risco": "ATENCAO",
    "indicadores": {
        "frequencia_cardiaca": {
            "atual": 92,
            "baseline": 65,
            "variacao_percentual": 41.5
        },
        "horas_sono": {
            "atual": 4,
            "baseline": 7.5
        }
    },
    "fatores_contextuais": {
        "co2_ppm": 1200,
        "temperatura_cabine_c": 24
    }
}
```

A avaliação deve registrar:

- Indicadores que apresentaram alterações.
- Comparação com o baseline individual.
- Fatores ambientais relevantes.
- Explicação da alteração identificada.
- Orientações baseadas em protocolos médicos validados.

A inteligência artificial poderá auxiliar na interpretação e comunicação dessas informações, mas não deverá realizar diagnósticos médicos por conta própria.

---

# 4. Estratégia de sincronização

O sistema deve funcionar offline, armazenando os dados localmente e sincronizando posteriormente com o servidor.

Para isso, as tabelas sincronizáveis possuem três campos:

| Campo | Responsabilidade |
|---|---|
| updated_at | Data da última atualização |
| deleted_at | Identifica exclusões lógicas |
| sync_status | Não será necessário em todas as tabelas se o estado for controlado separadamente |

A estratégia recomendada é manter uma tabela central de controle de sincronização.

```sql
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
```

**Funcionamento:**

1. O astronauta registra uma informação.
2. O SQLite armazena o registro localmente.
3. A operação é adicionada à fila de sincronização.
4. Quando houver conexão, o sistema envia os dados.
5. O servidor confirma o recebimento.
6. A operação é removida da fila.

Os identificadores UUID permitem que o registro mantenha a mesma identidade no banco local e no servidor.

A sincronização também deverá considerar conflitos de atualização, exclusões lógicas e reenvio de operações após falhas.

---

# 5. Relacionamentos

A estrutura de relacionamentos fica organizada da seguinte maneira:

```text
ASTRONAUTAS
    |
    |----< PARTICIPACAO_MISSAO >---- MISSOES
    |
    |----< TELEMETRIA_FISIOLOGICA
    |
    |----< REGISTROS_DIARIOS
    |
    |----< EXAMES_CLINICOS
    |
    |----< BASELINE_FISIOLOGICO
    |
    |----< AVALIACOES_SAUDE
    |
    |----< PRESENCA_MODULO >---- MODULOS_NAVE
                                      |
                                      |
                                  MISSOES
                                      |
                                      |
                              AMBIENTE_NAVE
```

---

# 6. Índices e desempenho

Como o sistema trabalhará com séries temporais, os índices são fundamentais.

As principais consultas deverão considerar:

- Histórico fisiológico por astronauta.
- Indicadores fisiológicos por período.
- Condições ambientais por módulo.
- Histórico de avaliações.
- Registros diários.
- Histórico de localização nos módulos.

Os índices apresentados nas definições das tabelas atendem às principais consultas iniciais.

A quantidade de dados poderá crescer consideravelmente durante missões longas. Portanto, futuramente será importante avaliar estratégias de retenção, agregação e armazenamento histórico.

---

# 7. Considerações técnicas

### SQLite

O banco local será responsável pelo armazenamento durante períodos sem conexão.

Recomendações:

- Ativar `PRAGMA foreign_keys = ON` em cada conexão.
- Utilizar transações para operações relacionadas.
- Considerar WAL para melhorar a concorrência de leitura e escrita.
- Utilizar UUID gerado pela aplicação.
- Armazenar datas em UTC.
- Validar os dados antes da persistência.
- Utilizar criptografia para proteger informações médicas.

### JSON

Os campos JSON dependem dos recursos JSON1 do SQLite.

A validação utiliza:

```sql
CHECK (json_valid(campo_json))
```

É necessário verificar a disponibilidade desses recursos na versão do SQLite utilizada.

### Segurança

Os dados médicos são sensíveis.

O sistema deve utilizar:

- Criptografia do banco local.
- Comunicação criptografada com o servidor.
- Autenticação e autorização.
- Controle de acesso aos registros.
- Proteção das credenciais.
- Registro de operações relevantes.

---

# 8. Ordem recomendada de implementação

Para evitar desenvolver todas as funcionalidades simultaneamente, a implementação deve seguir uma ordem incremental.

| Etapa | Implementação | Objetivo |
|---|---|---|
| 1 | Astronautas e missões | Criar a estrutura básica |
| 2 | Telemetria fisiológica | Armazenar indicadores |
| 3 | Registros diários | Registrar rotina e bem-estar |
| 4 | Baseline fisiológico | Estabelecer padrões individuais |
| 5 | Módulos e ambiente | Contextualizar as medições |
| 6 | Presença nos módulos | Relacionar astronautas e ambientes |
| 7 | Avaliações de saúde | Identificar e registrar alterações |
| 8 | Exames clínicos | Ampliar as informações médicas |
| 9 | Sincronização | Integrar banco local e servidor |

---

# 9. Objetivo final da arquitetura

O banco de dados não deve funcionar apenas como um repositório de informações médicas.

Sua estrutura deve permitir que o sistema compreenda o contexto de cada astronauta, acompanhe alterações fisiológicas, considere fatores ambientais e apresente informações relevantes para a tomada de decisão.

O fluxo principal será:

**Coleta de dados → Armazenamento local → Comparação com baseline → Análise contextual → Avaliação de saúde → Orientação → Sincronização com a Terra.**

Essa arquitetura estabelece a base de dados necessária para desenvolver o sistema de monitoramento e apoio à saúde dos astronautas.