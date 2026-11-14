sua responsabilidade é apenas fazer a parte do RAG, sem estruturar back-end e dados, só simular, tentando encontrar o processamento mais rápido e mais eficiente

aq tá um contexto doq é pra fazer, pode mudar se achar necessário, tente aperfeiçoar pra deixar mais veloz e preciso, coda ou vibe coda em typescript

lembre-se que a IA gasta mais processamento pra saída doq pra entrada

## vamos usar as LLMs pra 2 coisas:

- alteração de sintomas
- copiloto de saúde (a q explica sobre coisas de saúde pro user)

## aq tem uma conversa com o gpt com o contexto:

https://chatgpt.com/share/6af6790f-7acc-83e9-bd70-249b70f5b4ff

## Separe em 3 aplicações, todas com a mesma instância pra economia de processamento (funções ou classes), pode sugerir ou fazer alterações:

- **Análise de alteração** (a q vai identificar os dados que o usuário teve, oq mudou, o quão emergencial é e os dados do contexto (tipo se ele tava fazendo exercício ou algo do tipo, ela vai receber os dados em JSON do back-end pra etapa de alteração de saúde ou da explicação histórica pra copiloto de saúde))

- **consulta documental** (a q vai consultar o documento de médico com chamadas específicas pra economizar texto)

- **explicação histórica** (a q vai pegar os dados da análise de alteração, usar a tool de consulta documental quando preciso e gerar o diagnóstico pro usuário do que ele pode fazer, a q conversa com ele, essa vai ser apenas pra alteração porque vai receber diretamente os dados em JSON a serem alterados)

- **Busca histórica** (a q vai procurar no histórico do banco de dados um dado específico)

- **Chat** (a q vai usar as tools calls necesśarias para responder pro usuário, no caso será usada apenas pro ajudante de saúde)

## Regras pro processamento:

- estrutura um model com JSON pra envio de dado
- coloca instruções curtas e específicas pra cada modelo (tipo "seja direto", "busque não responder em tom familiar" etc)
- usa temperatura baixa pra ser direto quando necessário (tipo a tool de consulta deve ser direta nas palavras chave)
- as relações entre IAs devem ser em JSON estruturado (tipo, a de explicação histórica pede pra IA de consulta documental oq ela sabe sobre radiação, aí coloca um JSON disso tipo type: radiação, context: "exposição a radiação no módulo 3 da nave")
- cada entrada e saída deve ter uma estrutura pré-definida pra IA seguir, aí vc trabalha com output e input com class model
- permita com que o modelo não responda por falta de evidências
- não simular amizade, familiaridade, validação do humano, não incentivar conversas desnecessárias ou prolongadas, deixar claro q é uma ia, incentivar contato com outros tripulantes quando apropriado
- use quantização tipo Q4

## Documento médico:

Vc deve gerar um documento com uma estrutura clara, pode gerar com 1 só tópico no momento (BPM), aq tá um exemplo de como pode ser

| Campo | Exemplo |
|---|---|
| document_id | NASA_MED_001 |
| title | Documento de referência médica |
| source | NASA |
| section | Cardiovascular |
| indicator | Heart Rate |
| topic | Alterações de frequência cardíaca |
| population | Astronautas |
| version | 1.0 |
| page | 42 |
| authority | Protocolo oficial |

ela pode fazer buscas semânticas(no texto) ou trechos específicos, pegando o contexto em JSON de necessidades da pesquisa

## Chamar as tools:

use a tool call do ollama, o exemplo te enviei no whatsapp

## Comparação com baseline:

c pode identificar persistência de dados, queda brusca etc pro usuário entender o impacto da diferença, além disso tem q fazer a IA responder de maneira objetiva, direta e entendível pro astronauta prq ele não é técnico

## Comprovação/testes:

P comprovar simula alguns testes:

| Teste | O que queremos verificar |
|---|---|
| Sintoma com documentação correspondente | Recupera a evidência correta. |
| Sintoma sem documentação | Reconhece que não encontrou evidência. |
| Indicadores contraditórios | Não escolhe arbitrariamente uma explicação. |
| Dados incompletos | Não inventa informações ausentes. |
| Pergunta sobre histórico | Recupera os registros corretos. |
| Pergunta sobre causalidade | Não afirma uma relação não comprovada. |
| Situação crítica | Respeita a orientação de escalonamento do backend. |
| Pergunta emocional | Mantém limites apropriados e não incentiva dependência. |

tenta medir as característisticas do processamento q eu citei

cara por fim acho q c pode fazer com q caso a conversa passe de um limite implementar tipo uma compactação aí usa uma tool q faz um resumo pra IA tlg, e pode usar uma memória bem compacta tipo chatgpt como system prompt de cada IA pra ela ter noção do usuário e se adaptar um pouco mais e entender cada conversa.

Pode criar uma tool de memória também, pra captar memórias de chats anteriores (simula os dados, dps agnt cria o banco)
