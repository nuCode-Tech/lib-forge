import 'dart:convert';
import 'dart:io';

import 'package:path/path.dart' as path;

import 'options.dart';
import 'util.dart';

const _manifestFileName = 'xforge-manifest.json';

class ManifestPlatform {
  ManifestPlatform({
    required this.name,
    required this.triples,
    required this.artifacts,
  });

  final String name;
  final List<String> triples;
  final List<String> artifacts;

  bool matchesTarget(String target) => name == target || triples.contains(target);
}

class XforgeManifest {
  XforgeManifest({required this.platforms});

  final List<ManifestPlatform> platforms;
}

class ArtifactSelection {
  ArtifactSelection({required this.platform, required this.artifactName});

  final ManifestPlatform platform;
  final String artifactName;
}

class ArtifactResolution {
  ArtifactResolution._({
    required this.downloaded,
    this.artifact,
    this.reason,
  });

  factory ArtifactResolution.downloaded(File artifact) {
    return ArtifactResolution._(downloaded: true, artifact: artifact);
  }

  factory ArtifactResolution.fallback(String reason) {
    return ArtifactResolution._(downloaded: false, reason: reason);
  }

  final bool downloaded;
  final File? artifact;
  final String? reason;
}

class ManifestSignatureException implements Exception {
  ManifestSignatureException(this.message);
  final String message;
  @override
  String toString() => message;
}

class ArtifactSignatureException implements Exception {
  ArtifactSignatureException(this.message);
  final String message;
  @override
  String toString() => message;
}

class PlatformNotFoundException implements Exception {
  PlatformNotFoundException(this.message);
  final String message;
  @override
  String toString() => message;
}

class ArtifactNotFoundException implements Exception {
  ArtifactNotFoundException(this.message);
  final String message;
  @override
  String toString() => message;
}

class PrecompiledArtifactsProvider {
  PrecompiledArtifactsProvider({
    required this.crateDir,
    required this.config,
  });

  final String crateDir;
  final PrecompiledBinariesConfig config;

  Directory get _cacheRoot =>
      Directory(path.join(crateDir, '.dart_tool', 'xforge'));

  Directory _manifestCacheDir(String buildId) =>
      Directory(path.join(_cacheRoot.path, 'manifests', buildId));

  Directory _artifactCacheDir(String buildId) =>
      Directory(path.join(_cacheRoot.path, 'artifacts', buildId));

  Future<XforgeManifest> fetchVerifiedManifest({
    required String buildId,
  }) async {
    final manifestDir = _manifestCacheDir(buildId);
    final manifestFile = File(path.join(manifestDir.path, _manifestFileName));
    final manifestSig = File('${manifestFile.path}.sig');

    final manifestUrl =
        config.fileUrl(buildId: buildId, fileName: _manifestFileName);
    final sigUrl =
        config.fileUrl(buildId: buildId, fileName: '$_manifestFileName.sig');

    final manifestBytes = await readOrDownloadBytes(manifestFile, manifestUrl);
    final sigBytes = await readOrDownloadBytes(manifestSig, sigUrl);

    final valid = verifyEd25519Signature(
      publicKey: config.publicKey,
      message: manifestBytes,
      signature: sigBytes,
    );
    if (!valid) {
      _deleteIfExists(manifestFile);
      _deleteIfExists(manifestSig);
      throw ManifestSignatureException(
        'Manifest signature verification failed.',
      );
    }

    final decoded = utf8.decode(manifestBytes);
    return _parseManifest(decoded);
  }

  ArtifactSelection selectArtifact({
    required XforgeManifest manifest,
    required String target,
  }) {
    final platform = manifest.platforms.firstWhere(
      (entry) => entry.matchesTarget(target),
      orElse: () => throw PlatformNotFoundException(
        'No platform match for target "$target" in manifest.',
      ),
    );

    if (platform.artifacts.isEmpty) {
      throw ArtifactNotFoundException(
        'Manifest platform "${platform.name}" has no artifacts.',
      );
    }

    return ArtifactSelection(
      platform: platform,
      artifactName: platform.artifacts.first,
    );
  }

  Future<File> fetchVerifiedArtifact({
    required String buildId,
    required String artifactName,
  }) async {
    final artifactDir = _artifactCacheDir(buildId);
    final artifactFile = File(path.join(artifactDir.path, artifactName));
    final sigFile = File('${artifactFile.path}.sig');

    final artifactUrl =
        config.fileUrl(buildId: buildId, fileName: artifactName);
    final sigUrl =
        config.fileUrl(buildId: buildId, fileName: '$artifactName.sig');

    final artifactBytes = await readOrDownloadBytes(artifactFile, artifactUrl);
    final sigBytes = await readOrDownloadBytes(sigFile, sigUrl);

    final valid = verifyEd25519Signature(
      publicKey: config.publicKey,
      message: artifactBytes,
      signature: sigBytes,
    );
    if (!valid) {
      _deleteIfExists(artifactFile);
      _deleteIfExists(sigFile);
      throw ArtifactSignatureException(
        'Artifact signature verification failed.',
      );
    }

    return artifactFile;
  }

  Future<ArtifactResolution> resolveArtifact({
    required String buildId,
    required String target,
    required PrecompiledBinaryMode mode,
    required bool rustAvailable,
  }) async {
    final manifest = await fetchVerifiedManifest(buildId: buildId);

    ArtifactSelection selection;
    try {
      selection = selectArtifact(manifest: manifest, target: target);
    } on PlatformNotFoundException catch (err) {
      return _fallbackOrThrow(err.toString(), mode, rustAvailable);
    } on ArtifactNotFoundException catch (err) {
      return _fallbackOrThrow(err.toString(), mode, rustAvailable);
    }

    try {
      final artifact = await fetchVerifiedArtifact(
        buildId: buildId,
        artifactName: selection.artifactName,
      );
      return ArtifactResolution.downloaded(artifact);
    } on ArtifactSignatureException catch (err) {
      return _fallbackOrThrow(err.toString(), mode, rustAvailable);
    }
  }

  ArtifactResolution _fallbackOrThrow(
    String reason,
    PrecompiledBinaryMode mode,
    bool rustAvailable,
  ) {
    if (mode == PrecompiledBinaryMode.always) {
      throw StateError(
        'Precompiled binaries are required (mode=always). $reason',
      );
    }
    if (rustAvailable) {
      return ArtifactResolution.fallback(
        '$reason Falling back to local build.',
      );
    }
    throw StateError(
      '$reason Rust toolchain not detected; cannot fall back to local build.',
    );
  }
}

XforgeManifest _parseManifest(String jsonString) {
  final raw = jsonDecode(jsonString);
  if (raw is! Map<String, dynamic>) {
    throw FormatException('Manifest must be a JSON object.');
  }
  final platformsNode = raw['platforms'];
  if (platformsNode is! Map<String, dynamic>) {
    throw FormatException('Manifest platforms must be a map.');
  }
  final targetsNode = platformsNode['targets'];
  if (targetsNode is! List) {
    throw FormatException('Manifest platforms.targets must be a list.');
  }
  final platforms = <ManifestPlatform>[];
  for (final entry in targetsNode) {
    if (entry is! Map) {
      throw FormatException('Manifest platform entry must be a map.');
    }
    final name = entry['name'];
    if (name is! String || name.isEmpty) {
      throw FormatException('Manifest platform.name must be a string.');
    }
    final triplesRaw = entry['triples'];
    final triples = _stringListOrEmpty(triplesRaw);
    final artifactsRaw = entry['artifacts'];
    final artifacts = _stringListOrEmpty(artifactsRaw);
    platforms.add(
      ManifestPlatform(name: name, triples: triples, artifacts: artifacts),
    );
  }
  return XforgeManifest(platforms: platforms);
}

List<String> _stringListOrEmpty(Object? raw) {
  if (raw == null) {
    return const [];
  }
  if (raw is! List) {
    throw FormatException('Expected a list of strings.');
  }
  return raw
      .whereType<String>()
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .toList();
}

void _deleteIfExists(File file) {
  if (file.existsSync()) {
    file.deleteSync();
  }
}
