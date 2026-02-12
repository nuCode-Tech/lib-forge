import 'dart:io';

import 'package:ed25519_edwards/ed25519_edwards.dart';
import 'package:path/path.dart' as path;
import 'package:yaml/yaml.dart';

import 'util.dart';

enum PrecompiledBinaryMode { auto, always, never }

class PrecompiledBinariesConfig {
  PrecompiledBinariesConfig({
    required this.repository,
    required this.publicKey,
    this.urlPrefix,
    this.mode = PrecompiledBinaryMode.auto,
  });

  final String repository;
  final String? urlPrefix;
  final PublicKey publicKey;
  final PrecompiledBinaryMode mode;

  Uri fileUrl({required String buildId, required String fileName}) {
    final prefix = urlPrefix;
    if (prefix != null && prefix.isNotEmpty) {
      return Uri.parse('$prefix$buildId/$fileName');
    }
    return Uri.parse(
      'https://github.com/$repository/releases/download/$buildId/$fileName',
    );
  }

  static PrecompiledBinariesConfig parse(YamlNode node) {
    if (node is! YamlMap) {
      throw FormatException('precompiled_binaries must be a map');
    }

    String? urlPrefix;
    final urlPrefixNode = node.nodes['url_prefix'];
    if (urlPrefixNode != null) {
      if (urlPrefixNode is! YamlScalar || urlPrefixNode.value is! String) {
        throw FormatException('precompiled_binaries.url_prefix must be a string');
      }
      urlPrefix = urlPrefixNode.value as String;
    }

    PrecompiledBinaryMode mode = PrecompiledBinaryMode.auto;
    final modeNode = node.nodes['mode'];
    if (modeNode != null) {
      if (modeNode is! YamlScalar || modeNode.value is! String) {
        throw FormatException('precompiled_binaries.mode must be a string');
      }
      final m = (modeNode.value as String).trim();
      final parsed = _parsePrecompiledBinaryMode(m);
      if (parsed == null) {
        throw FormatException(
          'precompiled_binaries.mode must be one of: auto, always, never (aliases: download->always, build->never)',
        );
      }
      mode = parsed;
    }

    final repoNode = node.nodes['repository'];
    if (repoNode is! YamlScalar || repoNode.value is! String) {
      throw FormatException('precompiled_binaries.repository must be a string');
    }
    final repository = _normalizeOwnerRepo(repoNode.value as String);
    if (repository == null) {
      throw FormatException(
        'precompiled_binaries.repository must be in owner/repo format (or github.com/owner/repo)',
      );
    }

    final publicKeyNode = node.nodes['public_key'];
    if (publicKeyNode is! YamlScalar || publicKeyNode.value is! String) {
      throw FormatException('precompiled_binaries.public_key must be a string');
    }
    final keyBytes = decodeHex(publicKeyNode.value as String);
    if (keyBytes.length != 32) {
      throw FormatException('public_key must be 32 bytes');
    }
    return PrecompiledBinariesConfig(
      repository: repository,
      publicKey: PublicKey(keyBytes),
      urlPrefix: urlPrefix,
      mode: mode,
    );
  }
}

class XforgeOptions {
  XforgeOptions({required this.precompiledBinaries});

  final PrecompiledBinariesConfig? precompiledBinaries;

  static XforgeOptions load({required String crateDir}) {
    final file = File(path.join(crateDir, 'xforge.yaml'));
    if (!file.existsSync()) {
      return XforgeOptions(precompiledBinaries: null);
    }
    final root = loadYamlNode(file.readAsStringSync(), sourceUrl: file.uri);
    if (root is! YamlMap) {
      throw FormatException('xforge.yaml must be a map');
    }
    final node = root.nodes['precompiled_binaries'];
    if (node == null) {
      return XforgeOptions(precompiledBinaries: null);
    }
    return XforgeOptions(precompiledBinaries: PrecompiledBinariesConfig.parse(node));
  }
}

class UserOptions {
  UserOptions({required this.usePrecompiledBinaries});

  final bool usePrecompiledBinaries;

  static bool _rustupExists() {
    final envPath = Platform.environment['PATH'];
    final envPathSeparator = Platform.isWindows ? ';' : ':';
    final home = Platform.isWindows
        ? Platform.environment['USERPROFILE']
        : Platform.environment['HOME'];
    final paths = [
      if (home != null) path.join(home, '.cargo', 'bin'),
      if (envPath != null) ...envPath.split(envPathSeparator),
    ];
    for (final p in paths) {
      final rustup = Platform.isWindows ? 'rustup.exe' : 'rustup';
      if (File(path.join(p, rustup)).existsSync()) {
        return true;
      }
    }
    return false;
  }

  static bool rustupExists() => _rustupExists();

  static bool defaultUsePrecompiledBinaries() => !_rustupExists();

  static UserOptions load({required bool hasConfig}) {
    if (!hasConfig) {
      return UserOptions(usePrecompiledBinaries: false);
    }
    return UserOptions(usePrecompiledBinaries: defaultUsePrecompiledBinaries());
  }
}

PrecompiledBinaryMode? _parsePrecompiledBinaryMode(String raw) {
  final v = raw.trim().toLowerCase();
  return switch (v) {
    'auto' => PrecompiledBinaryMode.auto,
    'always' || 'download' => PrecompiledBinaryMode.always,
    'never' || 'build' || 'off' || 'disabled' => PrecompiledBinaryMode.never,
    _ => null,
  };
}

String? _normalizeOwnerRepo(String raw) {
  var v = raw.trim();
  v = v.replaceFirst(RegExp(r'^https?://'), '');
  v = v.replaceFirst(RegExp(r'^github\.com/'), '');
  v = v.replaceAll(RegExp(r'/+$'), '');
  final parts = v.split('/');
  if (parts.length != 2) {
    return null;
  }
  if (parts[0].isEmpty || parts[1].isEmpty) {
    return null;
  }
  return '${parts[0]}/${parts[1]}';
}
