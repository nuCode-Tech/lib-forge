import 'dart:io';

import 'package:archive/archive.dart';
import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:logging/logging.dart';
import 'package:path/path.dart' as path;

import 'artifacts_provider.dart';
import 'crate_hash.dart';
import 'options.dart';

// Build hook that prefers signed precompiled binaries, with local fallback.
final _log = Logger('libforge.precompiled_builder');

// Callback used when no precompiled artifact is available.
typedef FallbackBuilder =
    Future<void> Function(
      BuildInput input,
      BuildOutputBuilder output,
      List<AssetRouting> assetRouting,
      Logger? logger,
    );

// Builder that downloads verified artifacts when available.
final class PrecompiledBuilder implements Builder {
  const PrecompiledBuilder({
    required this.assetName,
    required this.fallback,
    this.cratePath,
    this.buildModeName = 'release',
  });

  final String assetName;
  final FallbackBuilder fallback;
  final String? cratePath;
  final String buildModeName;

  @override
  Future<void> run({
    required BuildInput input,
    required BuildOutputBuilder output,
    List<AssetRouting> assetRouting = const [ToAppBundle()],
    Logger? logger,
  }) async {
    _initLogging();
    if (!input.config.buildCodeAssets) {
      return;
    }

    logger ??= _log;

    final crateDirectory = _resolveCrateDirectory(
      rootPath: path.fromUri(input.packageRoot),
      cratePathOptions: cratePath != null
          ? [cratePath!]
          : const ['..', 'native', 'rust'],
    );

    final options = LibforgeOptions.load(crateDir: crateDirectory.path);
    final config = options.precompiledBinaries;
    if (config == null) {
      logger.info(
        'No precompiled_binaries config; falling back to local build.',
      );
      await fallback(input, output, assetRouting, logger);
      return;
    }

    if (config.mode == PrecompiledBinaryMode.never) {
      logger.info(
        'Precompiled binaries disabled by config (mode=never); falling back to local build.',
      );
      await fallback(input, output, assetRouting, logger);
      return;
    }

    final codeConfig = input.config.code;
    final targetTriple = _targetTripleFor(codeConfig);
    final linkMode = _linkModeFor(codeConfig);

    logger.info('Resolving precompiled binary for $targetTriple');
    logger.fine('Precompiled mode: ${config.mode}');
    final buildId = await computeReleaseHash(crateDir: crateDirectory.path);
    logger.fine('Computed build id: $buildId');
    final provider = PrecompiledArtifactsProvider(
      crateDir: crateDirectory.path,
      config: config,
    );

    final rustAvailable = UserOptions.rustupExists();
    ArtifactResolution resolution;
    try {
      resolution = await provider.resolveArtifact(
        buildId: buildId,
        target: targetTriple,
        mode: config.mode,
        rustAvailable: rustAvailable,
      );
    } catch (err, stack) {
      logger.severe('Failed to resolve precompiled artifact: $err', err, stack);
      rethrow;
    }

    if (resolution.downloaded && resolution.artifact != null) {
      final artifact = resolution.artifact!;
      output.dependencies.add(artifact.absolute.uri);

      File extracted;
      try {
        extracted = await _extractLibraryFromArchive(
          archive: artifact,
          buildId: buildId,
          targetTriple: targetTriple,
          codeConfig: codeConfig,
          crateDir: crateDirectory,
        );
      } catch (err, stack) {
        logger.severe('Failed to extract library: $err', err, stack);
        rethrow;
      }
      output.dependencies.add(extracted.absolute.uri);
      logger.fine('Extracted library to ${extracted.path}');

      for (final routing in assetRouting) {
        output.assets.code.add(
          CodeAsset(
            package: input.packageName,
            name: assetName,
            linkMode: linkMode,
            file: extracted.absolute.uri,
          ),
          routing: routing,
        );
      }
      logger.info('Using precompiled binary for $targetTriple');
      return;
    }

    logger.info(
      'Falling back to local build for $targetTriple. ${resolution.reason ?? ''}',
    );
    await fallback(input, output, assetRouting, logger);
  }

  Directory _resolveCrateDirectory({
    required String rootPath,
    required List<String> cratePathOptions,
  }) {
    for (final option in cratePathOptions) {
      final dir = Directory(path.normalize(path.join(rootPath, option)));
      if (dir.existsSync()) {
        return dir;
      }
    }
    throw StateError(
      'Could not find crate directory. Checked: $cratePathOptions at $rootPath',
    );
  }
}

Future<File> _extractLibraryFromArchive({
  required File archive,
  required String buildId,
  required String targetTriple,
  required CodeConfig codeConfig,
  required Directory crateDir,
}) async {
  final cacheRoot = Directory(
    path.join(
      crateDir.path,
      '.dart_tool',
      'libforge',
      'extracted',
      buildId,
      targetTriple,
    ),
  );
  if (!cacheRoot.existsSync()) {
    cacheRoot.createSync(recursive: true);
  }

  final expectedExtension = path.extension(
    codeConfig.targetOS.libraryFileName('libforge', _linkModeFor(codeConfig)),
  );

  final existing = _findLibraryInDir(
    cacheRoot,
    expectedExtension: expectedExtension,
  );
  if (existing != null) {
    return existing;
  }

  final bytes = await archive.readAsBytes();
  final archiveData = _decodeArchive(archive.path, bytes);
  final entry = _selectLibraryEntry(
    archiveData,
    expectedExtension: expectedExtension,
  );
  if (entry == null) {
    throw StateError(
      'No library with extension "$expectedExtension" found in ${archive.path}',
    );
  }

  final outPath = path.join(cacheRoot.path, path.basename(entry.name));
  final outFile = File(outPath);
  await outFile.writeAsBytes(entry.content, flush: true);
  return outFile;
}

Archive _decodeArchive(String filePath, List<int> bytes) {
  if (filePath.endsWith('.zip')) {
    return ZipDecoder().decodeBytes(bytes);
  }
  if (filePath.endsWith('.tar.gz') || filePath.endsWith('.tgz')) {
    final decompressed = GZipDecoder().decodeBytes(bytes);
    return TarDecoder().decodeBytes(decompressed);
  }
  throw StateError('Unsupported archive type: $filePath');
}

ArchiveFile? _selectLibraryEntry(
  Archive archive, {
  required String expectedExtension,
}) {
  ArchiveFile? fallback;
  for (final entry in archive.files) {
    if (!entry.isFile) {
      continue;
    }
    final entryExtension = path.posix.extension(entry.name);
    if (entryExtension != expectedExtension) {
      continue;
    }
    final parts = entry.name.split('/');
    if (parts.contains('lib')) {
      return entry;
    }
    fallback ??= entry;
  }
  return fallback;
}

File? _findLibraryInDir(
  Directory dir, {
  required String expectedExtension,
}) {
  if (!dir.existsSync()) {
    return null;
  }
  for (final entity in dir.listSync()) {
    if (entity is File && path.extension(entity.path) == expectedExtension) {
      return entity;
    }
  }
  return null;
}

String _targetTripleFor(CodeConfig codeConfig) {
  final targetOS = codeConfig.targetOS;
  final targetArchitecture = codeConfig.targetArchitecture;
  if (targetOS == OS.android) {
    return switch (targetArchitecture) {
      Architecture.arm64 => 'aarch64-linux-android',
      Architecture.arm => 'armv7-linux-androideabi',
      Architecture.x64 => 'x86_64-linux-android',
      _ => throw UnsupportedError(
        'Unsupported target: $targetOS on $targetArchitecture',
      ),
    };
  }
  if (targetOS == OS.iOS) {
    return switch (targetArchitecture) {
      Architecture.arm64
          when codeConfig.iOS.targetSdk == IOSSdk.iPhoneSimulator =>
        'aarch64-apple-ios-sim',
      Architecture.arm64 => 'aarch64-apple-ios',
      Architecture.x64 => 'x86_64-apple-ios',
      _ => throw UnsupportedError(
        'Unsupported target: $targetOS on $targetArchitecture',
      ),
    };
  }
  if (targetOS == OS.windows) {
    return switch (targetArchitecture) {
      Architecture.arm64 => 'aarch64-pc-windows-msvc',
      Architecture.x64 => 'x86_64-pc-windows-msvc',
      _ => throw UnsupportedError(
        'Unsupported target: $targetOS on $targetArchitecture',
      ),
    };
  }
  if (targetOS == OS.linux) {
    return switch (targetArchitecture) {
      Architecture.arm64 => 'aarch64-unknown-linux-gnu',
      Architecture.x64 => 'x86_64-unknown-linux-gnu',
      _ => throw UnsupportedError(
        'Unsupported target: $targetOS on $targetArchitecture',
      ),
    };
  }
  if (targetOS == OS.macOS) {
    return switch (targetArchitecture) {
      Architecture.arm64 => 'aarch64-apple-darwin',
      Architecture.x64 => 'x86_64-apple-darwin',
      _ => throw UnsupportedError(
        'Unsupported target: $targetOS on $targetArchitecture',
      ),
    };
  }
  throw UnsupportedError(
    'Unsupported target: $targetOS on $targetArchitecture',
  );
}

LinkMode _linkModeFor(CodeConfig codeConfig) {
  return switch (codeConfig.linkModePreference) {
    LinkModePreference.dynamic ||
    LinkModePreference.preferDynamic => DynamicLoadingBundled(),
    LinkModePreference.static ||
    LinkModePreference.preferStatic => StaticLinking(),
    _ => throw UnsupportedError(
      'Unsupported LinkModePreference: ${codeConfig.linkModePreference}',
    ),
  };
}

bool _loggingInitialized = false;

void _initLogging() {
  if (_loggingInitialized) return;
  _loggingInitialized = true;

  final verbose =
      Platform.environment['LIBFORGE_DART_PRECOMPILED_VERBOSE'] == '1';
  Logger.root.level = verbose ? Level.ALL : Level.INFO;
  Logger.root.onRecord.listen((rec) {
    final out = rec.level >= Level.WARNING ? stderr : stdout;
    out.writeln('${rec.level.name}: ${rec.message}');
    if (rec.error != null) {
      out.writeln(rec.error);
    }
    if (rec.stackTrace != null && verbose) {
      out.writeln(rec.stackTrace);
    }
  });
}
