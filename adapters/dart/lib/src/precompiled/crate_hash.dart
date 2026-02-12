import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:path/path.dart' as path;

const _hashVersion = 'b1';

Future<String> computeReleaseHash({required String crateDir}) async {
  final cargoToml = await _readRequired(crateDir, 'Cargo.toml');
  final cargoLock = await _readRequiredCargoLock(crateDir);
  final xforgeYaml = await _readOptional(crateDir, 'xforge.yaml');

  final canonical = canonicalJsonWithoutTarget(
    cargoToml: cargoToml,
    cargoLock: cargoLock,
    xforgeYaml: xforgeYaml,
    uniffiUdl: null,
  );
  final digest = sha256.convert(utf8.encode(canonical));
  return '$_hashVersion-${digest.toString()}';
}

String canonicalJsonWithoutTarget({
  required String cargoToml,
  required String cargoLock,
  String? xforgeYaml,
  String? uniffiUdl,
}) {
  final fields = <Map<String, dynamic>>[
    _field('cargo.toml', cargoToml),
    _field('cargo.lock', cargoLock),
    _field('rust.target_triple', null),
    _field('uniffi.udl', uniffiUdl),
    _field('xforge.yaml', xforgeYaml),
  ];

  fields.sort((a, b) => (a['name'] as String).compareTo(b['name'] as String));

  final root = <String, dynamic>{
    'inputs': fields,
    'version': _hashVersion,
  };

  return jsonEncode(root);
}

Map<String, dynamic> _field(String name, String? value) {
  return <String, dynamic>{
    'affects_abi': true,
    'name': name,
    'value': value,
  };
}

Future<String> _readRequired(String crateDir, String fileName) async {
  final file = File(path.join(crateDir, fileName));
  if (!file.existsSync()) {
    throw FileSystemException('Missing required file: $fileName', file.path);
  }
  return file.readAsString();
}

Future<String> _readRequiredCargoLock(String crateDir) async {
  var current = path.normalize(crateDir);
  while (true) {
    final candidate = File(path.join(current, 'Cargo.lock'));
    if (candidate.existsSync()) {
      return candidate.readAsString();
    }
    final parent = path.dirname(current);
    if (parent == current) {
      break;
    }
    current = parent;
  }
  throw FileSystemException(
    'Missing required file: Cargo.lock',
    path.join(crateDir, 'Cargo.lock'),
  );
}

Future<String?> _readOptional(String crateDir, String fileName) async {
  final file = File(path.join(crateDir, fileName));
  if (!file.existsSync()) {
    return null;
  }
  return file.readAsString();
}
