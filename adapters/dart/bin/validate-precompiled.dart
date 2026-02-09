import 'dart:io';

import '../tool/commands/validate_precompiled.dart';

Future<void> main(List<String> args) async {
  final code = await runValidatePrecompiled(args);
  // Dart's exit code defaults to 0, so ensure we exit explicitly.
  // ignore: dart_io_exit
  exit(code);
}
