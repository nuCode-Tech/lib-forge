import 'dart:io';

import 'package:uniffi_xforge/uniffi_xforge.dart';

Future<void> main(List<String> args) async {
  final name = args.isNotEmpty ? args.first : 'world';

  try {
    ensureInitialized();
  } catch (err) {
    stderr.writeln('Failed to initialize native library: $err');
    exitCode = 1;
    return;
  }

  final result = ping(name);
  stdout.writeln(result);
}
