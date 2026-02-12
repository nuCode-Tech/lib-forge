import 'dart:io';

import 'commands/keygen.dart';
import 'commands/validate_precompiled.dart';

Future<void> main(List<String> args) async {
  if (args.isEmpty || args.first == '--help' || args.first == '-h') {
    _printUsage();
    exit(0);
  }

  final command = args.first;
  final rest = args.sublist(1);

  switch (command) {
    case 'keygen':
      final code = await runKeygen(rest);
      exit(code);
    case 'validate-precompiled':
      final code = await runValidatePrecompiled(rest);
      exit(code);
    default:
      stderr.writeln('Unknown command: $command');
      _printUsage();
      exit(2);
  }
}

void _printUsage() {
  stdout.writeln('libforge_dart commands:');
  stdout.writeln('  keygen');
  stdout.writeln('  validate-precompiled');
  stdout.writeln('Run with --help for command options.');
}
