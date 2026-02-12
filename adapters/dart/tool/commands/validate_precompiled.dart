import 'dart:io';

import 'package:path/path.dart' as path;

import '../../lib/src/precompiled/artifacts_provider.dart';
import '../../lib/src/precompiled/crate_hash.dart';
import '../../lib/src/precompiled/options.dart';
import '../../lib/src/precompiled/target.dart';

Future<int> runValidatePrecompiled(List<String> args) async {
  _ParsedArgs parsed;
  try {
    parsed = _parseArgs(args);
  } on ArgumentError catch (err) {
    stderr.writeln('Argument error: ${err.message}');
    _printUsage();
    return 2;
  }
  if (parsed.showHelp) {
    _printUsage();
    return 0;
  }

  final crateDir = parsed.crateDir ?? Directory.current.path;
  final options = XforgeOptions.load(crateDir: crateDir);
  final config = options.precompiledBinaries;
  if (config == null) {
    stderr.writeln('xforge.yaml is missing precompiled_binaries config.');
    return 2;
  }

  final buildId =
      parsed.buildId ?? await computeReleaseHash(crateDir: crateDir);
  final target = parsed.target ?? detectHostTargetTriple();

  final provider = PrecompiledArtifactsProvider(
    crateDir: crateDir,
    config: config,
  );

  try {
    final manifest = await provider.fetchVerifiedManifest(buildId: buildId);
    final selection =
        provider.selectArtifact(manifest: manifest, target: target);
    await provider.fetchVerifiedArtifact(
      buildId: buildId,
      artifactName: selection.artifactName,
    );
    stdout.writeln('Validated precompiled artifact:');
    stdout.writeln('  crateDir: ${path.normalize(crateDir)}');
    stdout.writeln('  buildId: $buildId');
    stdout.writeln('  target: $target');
    stdout.writeln('  artifact: ${selection.artifactName}');
    return 0;
  } catch (err) {
    stderr.writeln('Validation failed: $err');
    return 1;
  }
}

class _ParsedArgs {
  _ParsedArgs({
    required this.showHelp,
    this.crateDir,
    this.buildId,
    this.target,
  });

  final bool showHelp;
  final String? crateDir;
  final String? buildId;
  final String? target;
}

_ParsedArgs _parseArgs(List<String> args) {
  String? crateDir;
  String? buildId;
  String? target;
  var showHelp = false;

  for (var i = 0; i < args.length; i++) {
    final arg = args[i];
    if (arg == '--help' || arg == '-h') {
      showHelp = true;
      continue;
    }
    if (arg.startsWith('--crate-dir=')) {
      crateDir = arg.substring('--crate-dir='.length);
      continue;
    }
    if (arg == '--crate-dir') {
      crateDir = _valueOrThrow(args, ++i, '--crate-dir');
      continue;
    }
    if (arg.startsWith('--build-id=')) {
      buildId = arg.substring('--build-id='.length);
      continue;
    }
    if (arg == '--build-id') {
      buildId = _valueOrThrow(args, ++i, '--build-id');
      continue;
    }
    if (arg.startsWith('--target=')) {
      target = arg.substring('--target='.length);
      continue;
    }
    if (arg == '--target') {
      target = _valueOrThrow(args, ++i, '--target');
      continue;
    }
    throw ArgumentError('Unknown argument: $arg');
  }

  return _ParsedArgs(
    showHelp: showHelp,
    crateDir: crateDir,
    buildId: buildId,
    target: target,
  );
}

String _valueOrThrow(List<String> args, int index, String flag) {
  if (index >= args.length) {
    throw ArgumentError('Missing value for $flag');
  }
  return args[index];
}

void _printUsage() {
  stdout.writeln('Usage: dart run xforge_dart validate-precompiled [options]');
  stdout.writeln('Options:');
  stdout.writeln('  --crate-dir <path>   Crate directory (default: cwd)');
  stdout.writeln('  --build-id <id>      Build id override');
  stdout.writeln('  --target <triple>    Rust target triple (default: host)');
  stdout.writeln('  --help               Show this help');
}
