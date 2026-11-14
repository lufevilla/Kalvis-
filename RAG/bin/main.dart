import 'dart:convert';
import 'dart:io';
import '../lib/alteracao.dart';
import '../lib/copiloto.dart';
import '../lib/documentos.dart';
import '../lib/entrada.dart';
import '../lib/explicacao.dart';
import '../lib/llm.dart';
import '../lib/testes.dart';

Future<void> main(List<String> a) async {
  final llm = Llm();
  try {
    carregarDocumentos();
    switch (a.firstOrNull) {
      case 'analisar':
        final e = validarEntrada(jsonDecode(await File(a.elementAtOrNull(1) ?? 'modeloEntrada.json').readAsString()));
        final r = await explicar(llm, analisarAlteracao(e));
        print(r['mensagem']);
        print('Fontes: ${r['rag_sources']}');
        await File(a.elementAtOrNull(2) ?? 'resultado.json')
            .writeAsString(const JsonEncoder.withIndent('  ').convert(r));
        print(llm.metricas.resumo());
      case 'chat':
        final c = Copiloto(llm, a.elementAtOrNull(1) ?? 'AST001');
        print('IA de apoio em saúde. Digite "sair" para encerrar.');
        while (true) {
          stdout.write('> ');
          final l = stdin.readLineSync();
          if (l == null || l.trim() == 'sair') break;
          if (l.trim().isEmpty) continue;
          print(await c.perguntar(l));
        }
      case 'testar':
        await testar(llm);
      default:
        print('Use: dart run bin/main.dart [analisar <in> <out> | chat <id> | testar]');
        exitCode = 2;
    }
  } on FormatException catch (e) {
    stderr.writeln('Inválido: ${e.message}');
    exitCode = 1;
  } catch (e) {
    stderr.writeln('Erro: $e');
    exitCode = 1;
  } finally {
    llm.fechar();
  }
}
