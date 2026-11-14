# Heartcare — Dart + Ollama + RAG integrado

Migração do projeto TypeScript para **Dart puro** (sem Flutter e sem pacotes Dart externos). Mantém os 13 documentos de contexto científico, os modelos JSON e o catálogo `FONTES.md`.

## Requisitos
- Dart SDK >= 3.2
- Ollama instalado e ativo na porta 11434
- Servidor ChromaDB com API v2 na porta 8000 (por exemplo `chroma run --path ./chroma_data` em outro terminal)
- Modelos locais: `ollama pull qwen3.5:4b` e `ollama pull nomic-embed-text`

## Comandos
Na pasta do projeto:

```bash
dart pub get
dart run bin/main.dart indexar
dart run bin/main.dart analisar modeloEntrada.json resultado.json
```

## Como funciona
1. `bin/main.dart` lê JSON e chama o pipeline.
2. `lib/rag.dart` lê os documentos, divide em trechos e usa embeddings do Ollama.
3. O ChromaDB guarda os vetores; a busca retorna os trechos semanticamente próximos.
4. `lib/analysis.dart` envia medições e trechos ao Ollama e valida o JSON de saída.
5. `resultado.json` contém os campos esperados e `rag_sources` para rastreabilidade.

## Documentos
As 13 fichas são **resumos de evidências**, não protocolos médicos autorizados. O campo `approved: true` dos arquivos herdados indica aceitação para indexação no protótipo; `clinical_protocol_approved: false` indica que não são procedimentos clínicos aprovados. Verifique todas as fontes antes de qualquer uso real.

## Limites
Não há diagnóstico, triagem ou recomendação clínica automática. `status` é forçado para `insufficient_data`. A implementação não foi testada com serviços reais neste ambiente. A API Chroma v2 pode variar entre versões: se sua instalação for diferente, adapte `lib/rag.dart`. Reindexação não é atômica; não use em produção sem controles de concorrência, autenticação, revisão documental e validação clínica.
