# NASA Space Apps — Queries

> Documento de referência das consultas necessárias ao SQLite. Descreve **o que cada consulta deve retornar**, não sua implementação SQL.
>
> Banco local: SQLite · Arquitetura: offline-first

Vai ter o banco de dados local e o postgree, o postgree vai ser exatamente como o SQL só vai armazenar tudo (outras missões tb etc) e esse local vai ficar com os dados da missão local só (só limita a missão em 1)
## 1. Convenções

- Toda consulta operacional deve receber `astronauta_id` e, quando aplicável, `missao_id`.
- Registros com `deleted_at` preenchido não aparecem nas consultas normais.
- Ordenar séries temporais por `timestamp_utc` crescente; telas de histórico podem usar decrescente.
- Retornar o `id` de cada registro para permitir edição, exclusão e sincronização.
- Não substituir valores ausentes por zero: manter `NULL`.
- Consultas de comparação devem informar também o horário de cada medição.

## 2. CRUD — consultas por entidade

| Consulta | O que deve fazer / retornar |
|---|---|
| Criar | Persistir um novo registro e retornar o registro criado. |
| Buscar por ID | Retornar um registro específico, se existir e não estiver excluído. |
| Listar | Retornar registros da entidade, com filtros, ordenação e paginação. |
| Atualizar | Alterar os campos permitidos e retornar o registro atualizado. |
| Excluir | Marcar `deleted_at` (exclusão lógica) e retornar a confirmação. |
| Restaurar | Limpar `deleted_at`, quando a regra permitir, e retornar o registro restaurado. |

## 3. Consulta de comparação pra saber as alterações

Deve fornecer os dois dados com inner join, ent por exemplo BPM eu quero a baseline com o inner join do atual, eu preciso das queries de todos os dados fisiológicos de comparação

## 4. Contexto da medição

| Consulta | O que deve gerar |
|---|---|
| Contexto diário | Registro diário mais próximo ou correspondente à data da medição: sono, exercícios, hidratação, estresse, humor e fadiga. |
| Presença no momento da medição | Módulo em que o astronauta estava no horário informado, se houver intervalo de presença correspondente. |
| Ambiente no momento da medição | Leituras ambientais do módulo ocupado, próximas ao horário fisiológico: CO₂, radiação, temperatura e pressão, com horários e diferença temporal entre as coletas. |
| Contexto de um intervalo | Presenças e leituras ambientais que coincidem com o intervalo analisado. |
| Contexto de avaliação | Pacote consolidado com dados fisiológicos, baseline, rotina diária, atividade registrada, módulo e ambiente relacionados à avaliação. |

**Limitação do esquema atual:** `registros_diarios` guarda minutos totais de exercício por dia, mas não registra horário, modalidade ou intensidade de cada sessão. ENt nois precisa criar nova tabela disso

## 5. Consultas para monitoramento e gráficos

As consultas abaixo devem retornar séries prontas para plotagem, sem agregações que escondam os dados necessários.

| Consulta | O que deve retornar |
|---|---|
| Série de BPM | Horário e frequência cardíaca por medição, no período selecionado. |
| Série de SpO₂ | Horário e saturação de oxigênio por medição. |
| Série de temperatura | Horário, temperatura corporal e temperatura da pele, em séries separadas. |
| Série respiratória | Horário e taxa respiratória. |
| Série de HRV | Horário e HRV. |
| Série de pressão arterial | Horário, pressão sistólica e diastólica. |
| Série de glicose | Horário e glicose. |
| Série ambiental | Horário e valores de CO₂, radiação, temperatura e pressão por módulo. |
| Resumo do período | Primeiro, último, mínimo, máximo, média e quantidade de amostras por indicador. |
| Resumo diário | Um ponto por dia com agregações definidas para cada indicador e total de registros. |
| Histórico de avaliações | Data, nível de risco, resumo e identificador da avaliação (tem q ser decrescente). |
| Dados do painel atual | Últimos valores disponíveis, horário da última coleta e estado de disponibilidade de cada indicador i. |

Parâmetros comuns: `astronauta_id`, `missao_id` opcional, `inicio_utc`, `fim_utc`, indicador(es), intervalo de agregação e limite de pontos.

Para períodos longos, oferecer agregação por hora/dia. Para períodos curtos, retornar as medições originais. Não interpolar dados ausentes sem sinalizar isso.

## 6. Consultas para IA e avaliações

| Consulta | O que deve gerar |
|---|---|
| Pacote para análise de alteração | Medições recentes, comparação com medições anteriores, baseline aplicável e contexto ambiental/diário disponível. |
| Dados de uma avaliação | Avaliação completa por ID, incluindo indicadores, fatores contextuais, explicação e orientação. |
| Avaliações recentes | Últimas avaliações do astronauta, com nível de risco e resumo. |
| Avaliações por indicador | Avaliações em que determinado indicador foi analisado, com período e nível de risco. |
| Evidências de uma avaliação | IDs e valores das medições, baseline e leituras contextuais que fundamentaram a avaliação. |
| Histórico para explicação | Registros fisiológicos e contextuais relevantes a uma pergunta e período, com horários e origem. |
| Exames do astronauta | Exames por categoria, fase da missão e data, incluindo resultados JSON e observações. |

A IA deve receber somente o recorte necessário à pergunta. O resultado de uma análise deve poder ser armazenado em `avaliacoes_saude`, mantendo as evidências que a originaram.

## 7. Histórico compacto das conversas da IA

Adicionar uma tabela local chamada `historico_conversas_ia`. Uma linha representa uma mensagem, não a conversa inteira.

| Campo | Finalidade |
|---|---|
| `id` | UUID da mensagem. |
| `astronauta_id` | Dono do histórico. |
| `conversa_id` | Agrupa mensagens da mesma conversa. |
| `papel` | `USER`, `ASSISTANT` ou `TOOL`. |
| `conteudo` | Texto da mensagem ou resultado resumido da ferramenta. |
| `criado_em` | Data/hora UTC. |

Consultas necessárias:

| Consulta | O que deve gerar |
|---|---|
| Criar mensagem | Persistir uma mensagem e retornar seu ID. |
| Listar conversas | Uma linha por conversa, com última mensagem, data e quantidade de mensagens. |
| Abrir conversa | Mensagens de uma conversa em ordem cronológica. |
| Buscar mensagens | Mensagens que correspondam a um termo, dentro do histórico do astronauta. |
| Recuperar contexto recente | Últimas N mensagens ou resumo compacto da conversa para continuidade. |
| Excluir conversa | Excluir logicamente ou remover todas as mensagens da conversa, conforme política de retenção. |

**Para manter compacto:** não duplicar medições, documentos RAG ou grandes JSONs no histórico. Salve só a mensagem e, quando necessário, IDs de avaliação/documento consultados. Limitar o contexto enviado ao LLM às mensagens recentes e a um resumo curto. Definir limite de retenção e proteger o histórico como dado médico sensível.

## 8. Offline-first e sincronização
Coloca deleted_at (vai manter o dado marcardo assim até o servidor deletar, não vamos sincronizar esses dados), updated_at e salva o uuid e id de cada row da tabela, agnt precisa trabalhar isso mas n sei se é back ou banco de dados


## 9. Índices recomendados para as consultas

Sempre gera a query com o id do astronauta e o tempo

## 10. Pontos do esquema a resolver

1. **Atividades:** criar tabela de sessões de exercício se a análise precisar saber modalidade, intensidade e horário exato.
2. **Fase da missão:** a fase usada para selecionar o baseline precisa estar disponível no contexto da consulta ou ser derivável de uma regra documentada.
3. **Ambiente:** relacionar leitura ambiental ao astronauta por presença no módulo e proximidade temporal; não associar apenas pelo módulo.
4. **Baseline:** garantir que a seleção retorne um único baseline válido para astronauta, indicador, fase e missão.

---