# RAG — Especificação de Implementação

## 1. Objetivo

Implementar, em TypeScript, os cinco papéis de IA descritos em
`docs/NASA Space Apps — Implementação de IA.md`, usando como base documental os
13 arquivos de `RAG/documentos/aprovados` e o catálogo `RAG/FONTES.md`.

Os cinco papéis são:

1. **Análise de alteração** — compara dados atuais com baseline/histórico e
   descreve o que mudou, sem diagnóstico.
2. **Consulta documental** — consulta o RAG com chamadas específicas para
   economizar contexto.
3. **Explicação histórica** — transforma a análise em explicação objetiva para
   o astronauta, usando consulta documental quando necessário.
4. **Busca histórica** — procura registros específicos no histórico simulado.
5. **Chat/copiloto de saúde** — responde ao astronauta com tools, memória e
   contexto recente de mensagens.

## 2. Decisões fixadas

- **Armazenamento vetorial:** ChromaDB local, já usado pelo projeto.
- **Contexto de mensagens:** **recente + busca textual**. Não haverá busca
  semântica de mensagens nesta etapa.
- **Mensagens simuladas:** `RAG/simulacao/mensagens.json`, com interface pronta
  para substituir pelo banco depois.
- **Histórico simulado:** `RAG/simulacao/historico.json`.
- **`backend_assessment`:** simulado na entrada e única fonte para status,
  urgência, ações permitidas e escalonamento.
- **Idioma:** automático. Chat e explicação respondem no idioma da mensagem ou
  da entrada do astronauta. As chaves JSON permanecem em inglês.
- **Sem limiares clínicos inventados:** as regras em TypeScript calculam apenas
  diferenças, tendência, persistência, quedas e dados ausentes.
- **Fora do escopo nesta etapa:** criar/alterar documento médico, back-end real,
  banco real, APIs externas e configuração de quantização.

## 3. Contratos

### 3.1 Entrada estendida

A entrada atual (`modeloEntrada.json`) continua válida. Novos campos são
opcionais:

```ts
interface Entrada {
  mission: JsonObject;
  astronaut: JsonObject;
  timestamp: string;
  physiological: JsonObject;
  psychological: JsonObject;
  environment: JsonObject;
  activities: JsonObject;
  baseline?: JsonObject;
  history?: JsonObject;
  backend_assessment?: BackendAssessment;
  language?: string;
}
```

### 3.2 `backend_assessment` simulado

```ts
interface BackendAssessment {
  risk_level: 'stable' | 'attention' | 'critical' | 'unknown';
  escalation: {
    recommended: boolean;
    priority: 'routine' | 'urgent' | 'emergency' | 'undetermined';
    reason: string;
  };
  allowed_actions: Array<{
    action: string;
    requires_crew_medical_officer: boolean;
  }>;
}
```

Regras:

- A IA **não** sobrescreve `risk_level`, `escalation` ou `allowed_actions`.
- Sem `backend_assessment`, o resultado é `insufficient_data` e o escalonamento
  é `undetermined`.
- A IA pode explicar o que o backend informou, mas não pode aumentar ou reduzir
  a urgência.

### 3.3 Mensagem simulada

Compatível com a tabela `historico_conversas_ia` descrita em
`docs/NASA Space Apps — Queries.md`:

```ts
interface Mensagem {
  id: string;
  astronaut_id: string;
  conversa_id: string;
  papel: 'USER' | 'ASSISTANT' | 'TOOL';
  conteudo: string;
  criado_em: string;
}
```

### 3.4 Contexto de mensagens

```ts
interface ContextoMensagens {
  conversa_id: string;
  mensagens_recentes: Mensagem[];
  resumo?: string;
  memorias_relevantes: string[];
}
```

O contexto é montado com:

1. últimas N mensagens da conversa, em ordem cronológica;
2. resumo compacto, se a conversa tiver sido compactada;
3. memórias relevantes recuperadas por **busca textual** no histórico
   simulado.

### 3.5 Consulta documental

```ts
interface NecessidadeConsulta {
  type: string;
  context: string;
  indicator?: string;
  category?: string;
  top_k?: number;
  max_chars?: number;
}

interface ResultadoConsulta {
  found: boolean;
  evidence: Array<{
    chunk_id: string;
    document_id: string;
    title: string;
    source: string;
    version: string;
    text: string;
    distance: number | null;
  }>;
  limitations: string[];
}
```

Regras:

- `top_k` padrão: 3.
- `max_chars` padrão: 1500 por consulta.
- Se nenhum trecho ficar abaixo do limiar de distância, retornar
  `found: false`.
- Deduplicar por `document_id`.
- Filtrar metadados em TypeScript para não depender da sintaxe `where` da versão
  do Chroma.

### 3.6 Comparação descritiva

```ts
interface ComparacaoIndicador {
  indicator: string;
  current: number | string | null;
  baseline: number | string | null;
  delta: number | null;
  percent_change: number | null;
  trend: 'up' | 'down' | 'stable' | 'unknown';
  consecutive_changes: number;
  missing: boolean;
}
```

Não há classificação clínica. `trend` e `consecutive_changes` são descritivos.

### 3.7 Relatório de alteração

```ts
interface RelatorioAlteracao {
  assessment_id: string;
  status: 'stable' | 'attention' | 'critical' | 'insufficient_data';
  observations: Array<{
    indicator: string;
    finding: string;
    evidence: string;
  }>;
  possible_explanations: Array<{
    hypothesis: string;
    confidence: 'undetermined' | 'low' | 'moderate';
  }>;
  missing_data: string[];
  recommended_actions: Array<{
    action: string;
    source: 'backend_assessment';
    requires_crew_medical_officer: boolean;
  }>;
  earth_consultation: {
    recommended: boolean;
    priority: string;
    reason: string;
  };
  limitations: string[];
  rag_sources: Array<{
    chunk_id: string;
    document_id: string;
    source: string;
    version: string;
  }>;
}
```

O LLM gera apenas `observations`, `possible_explanations` e `limitations`.
`status`, `missing_data`, `recommended_actions`, `earth_consultation` e
`assessment_id` são calculados em TypeScript.

### 3.8 Explicação histórica

```ts
interface ExplicacaoHistorica {
  summary: string;
  what_changed: string[];
  possible_meaning: string;
  what_to_do: string[];
  sources: string[];
  limitations: string[];
  escalation: {
    recommended: boolean;
    priority: string;
    reason: string;
  };
}
```

Regras:

- Não afirmar causalidade.
- `what_to_do` só pode conter ações autorizadas pelo `backend_assessment`.
- Se não houver evidência, usar `insufficient_evidence` e explicar a limitação.

### 3.9 Chat

```ts
interface RespostaChat {
  reply: string;
  sources: string[];
  used_tools: string[];
  needs_more_data: boolean;
  escalation: {
    recommended: boolean;
    priority: string;
    reason: string;
  };
}
```

Regras de comportamento:

- Deixar claro que é uma IA.
- Não simular amizade, familiaridade ou validação do humano.
- Não incentivar conversa desnecessária ou prolongada.
- Não diagnosticar, não recomendar medicamento ou tratamento.
- Dizer quando não há evidência suficiente.
- Incentivar contato com outros tripulantes quando apropriado.
- Responder no idioma do astronauta.

## 4. Arquitetura

```
RAG/src/
  main.ts          CLI
  ai.ts            fachada compartilhada; uma instância de Ollama/Chroma
  analysis.ts      análise de alteração
  explanation.ts   explicação histórica
  chat.ts          chat/copiloto
  tools.ts         consultarDocumentos / buscarHistorico / buscarMensagens
  history.ts       repositório histórico simulado
  mensagens.ts     contexto recente + busca textual
  memory.ts        memória compacta + compactação de conversa
  rules.ts         comparações e regras determinísticas
  rag.ts           indexação e busca no ChromaDB
  ollama.ts        chat, embeddings, tools e métricas
  prompts.ts       instruções curtas por papel
  types.ts         contratos e validadores
```

Arquivos de simulação:

```
RAG/simulacao/
  mensagens.json
  historico.json
  entradas/
    com_backend_assessment.json
    dados_incompletos.json
    sinais_divergentes.json
```

## 5. Fases

### Fase 0 — Contratos, simulação e harness

- Estender `types.ts` com os contratos desta especificação.
- Criar os arquivos de `RAG/simulacao`.
- Adicionar `node:test` + `tsx --test` e fakes de Ollama/Chroma.
- Adicionar scripts `test` e `bench` no `package.json`.

**Aceite:** `npm run check` passa; testes rodam; comportamento atual preservado.

### Fase 1 — ChromaDB, consulta documental e regras

- Manter `indexar`/`buscar`.
- Indexar `category`, `document_type` e `clinical_protocol_approved`.
- Implementar `consultarDocumentos(need)` com `top_k`, `max_chars`, limiar de
  distância, deduplicação e `found: false`.
- Implementar `rules.ts`:
  - `assessment_id` determinístico;
  - comparação descritiva;
  - detecção de dados ausentes;
  - detecção de sinais divergentes;
  - status/escalonamento/ações a partir de `backend_assessment`.
- Refatorar `analysis.ts` para usar as regras e reduzir o schema do LLM.

**Aceite:** testes 1, 2, 3, 4 e 7 passam com LLM fake; testes 1 e 2 também
contra os documentos reais.

### Fase 2 — Histórico, mensagens e memória

- Implementar `history.ts` com `MockHistoricoRepository`.
- Implementar `mensagens.ts`:
  - `carregarContextoMensagens`;
  - `buscarMensagens` textual.
- Implementar `memory.ts`:
  - memória compacta;
  - compactação de conversa;
  - injeção de resumo + mensagens recentes no prompt.
- Implementar `tools.ts`.

**Aceite:** teste 5 passa; teste de compactação passa; contexto recente é
injetado no chat e na explicação.

### Fase 3 — Explicação histórica e chat

- Implementar `explanation.ts` com tool loop limitado a 2 iterações.
- Implementar `chat.ts` com as tools e as regras de comportamento.
- Refatorar `ollama.ts` para suportar `tools`, `keep_alive`, `num_predict` e
  métricas de tokens.
- Implementar `ai.ts` como fachada compartilhada.

**Aceite:** testes 6 e 8 passam; chat usa contexto recente e não chama tools sem
necessidade.

### Fase 4 — CLI, testes completos e benchmark

- Adicionar comandos em `main.ts`: `indexar`, `analisar`, `explicar`, `chat`,
  `buscar-historico`, `consultar`, `bench`.
- Implementar `bench.ts` com as métricas da seção 7.
- Fechar os 8 cenários de teste.

**Aceite:** `npm run check`, `npm test` e `npm run bench` passam.

## 6. Testes

| Teste | Módulo principal | Critério |
|---|---|---|
| Sintoma com documentação correspondente | `rag.ts` / `analysis.ts` | Recupera a evidência correta e cita a fonte |
| Sintoma sem documentação | `rag.ts` | Nenhum trecho abaixo do limiar; `insufficient_evidence` |
| Indicadores contraditórios | `rules.ts` | Marca sinais divergentes; não escolhe uma explicação |
| Dados incompletos | `rules.ts` / `analysis.ts` | Lista `missing_data`; não inventa informação |
| Pergunta sobre histórico | `history.ts` | Recupera os registros corretos |
| Pergunta sobre causalidade | `explanation.ts` / `chat.ts` | Não afirma relação não comprovada |
| Situação crítica | `rules.ts` | Respeita `backend_assessment.escalation` |
| Pergunta emocional | `chat.ts` | Mantém limites e sugere tripulação quando apropriado |

## 7. Benchmark

Medir por execução:

- `embedding_ms`
- `retrieval_ms`
- `llm_ms`
- `total_ms`
- `prompt_eval_count`
- `eval_count`
- `retrieved_chunks`
- `context_chars`
- `output_chars`

Comparar o fluxo novo com o fluxo atual de `analysis.ts`.

## 8. Riscos e verificações antes de codar

- Confirmar na versão instalada do Ollama o formato exato de
  `message.tool_calls` e o envio do resultado com `role: 'tool'`.
- Confirmar com `/api/show` se o modelo configurado suporta tools.
- Confirmar a compatibilidade entre a versão do cliente npm do ChromaDB e o
  servidor local.
- Não usar os dados simulados como evidência clínica real.
- Não criar limiares clínicos sem protocolo validado.

## 9. Fora de escopo

- Criação ou alteração do documento médico.
- Back-end real, banco real e sincronização.
- APIs externas.
- Configuração de quantização do modelo.
- Diagnóstico, prescrição ou recomendação de tratamento.
