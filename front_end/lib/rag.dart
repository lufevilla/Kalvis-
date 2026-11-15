import 'package:heartcare_rag/alteracao.dart';
import 'package:heartcare_rag/documentos.dart';
import 'package:heartcare_rag/explicacao.dart';
import 'package:heartcare_rag/llm.dart';
import 'dart:io';

import 'package:heartcare_rag/alteracao.dart';
import 'package:heartcare_rag/documentos.dart';
import 'package:heartcare_rag/explicacao.dart';
import 'package:heartcare_rag/llm.dart';
import 'dart:io';

class RagService {
  RagService() {
    // Caminho absoluto para a pasta RAG/documentos/aprovados na raiz do projeto
    // Detecta se está rodando do bundle (build/linux/x64/release/bundle) ou do source
    String ragDocsPath;
    final currentDir = Directory.current.path;
    
    if (currentDir.contains('front_end/build')) {
      // Rodando do bundle: build/linux/x64/release/bundle -> ../../../../../
      ragDocsPath = '$currentDir/../../../../../../RAG/documentos/aprovados';
    } else if (currentDir.contains('front_end')) {
      // Rodando do source front_end: ../
      ragDocsPath = '$currentDir/../RAG/documentos/aprovados';
    } else {
      // Diretório raiz do projeto
      ragDocsPath = '$currentDir/RAG/documentos/aprovados';
    }
    
    carregarDocumentos(ragDocsPath);
  }

  final _llm = Llm();

  Future<Map<String, dynamic>> analisarAlerta(
    Map<String, dynamic> raw,
    String classificacao,
  ) async {
    final inds = (raw['indicadores'] as List? ?? [])
        .cast<Map<String, dynamic>>();

    final alts = [
      for (final i in inds)
        if (i['nivel_final'] != 'normal')
          {
            'indicator': i['indicador'],
            'atual': i['z_score'] ?? '-',
            'baseline': '-',
            'variacao': i['direcao'] == 'abaixo' ? -1 : 1,
            'persistente': i['persistente'] == true,
            'brusca': false,
            'tendencia': (i['tendencia'] as Map?)?['direcao'] ?? 'estável',
          },
    ];

    final status = switch (raw['nivel_final']) {
      'critico' => 'critical',
      'atencao' => 'attention',
      _ => 'normal',
    };

    final r = await explicar(
      _llm,
      Analise('${raw['astronauta_id']}', status, alts, const [], {
        'fase': raw['fase'],
        'dias_na_fase': raw['dias_na_fase'],
        'episodio': raw['episodio'],
        'motivos': [for (final m in raw['motivos'] ?? []) m['codigo']],
      }),
    );

    return {...r, 'alerta_id': raw['id'], 'classificacao': classificacao};
  }

  void fechar() => _llm.fechar();
}
