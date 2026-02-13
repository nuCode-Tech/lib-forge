import 'package:ed25519_edwards/ed25519_edwards.dart';
import 'package:logging/logging.dart';
import 'package:test/test.dart';

import 'package:xforge_dart/src/precompiled/options.dart';

void main() {
  test('parses mode and logging level overrides', () {
    final overrides = AppPrecompiledOverrides.parse({
      'precompiled_binaries': {
        'mode': 'always',
        'logging': {'level': 'INFO'},
      },
    });

    expect(overrides?.mode, PrecompiledBinaryMode.always);
    expect(overrides?.logLevel, Level.INFO);
  });

  test('boolean false disables precompiled binaries', () {
    final overrides = AppPrecompiledOverrides.parse({
      'precompiled_binaries': false,
    });

    expect(overrides?.mode, PrecompiledBinaryMode.never);
  });

  test('copyWith applies app override precedence', () {
    final config = PrecompiledBinariesConfig(
      repository: 'owner/repo',
      publicKey: PublicKey(List<int>.filled(32, 1)),
      mode: PrecompiledBinaryMode.auto,
    );
    final overrides = AppPrecompiledOverrides.parse({
      'precompiled_binaries': {'mode': 'never'},
    });

    final merged = config.copyWith(mode: overrides?.mode);
    expect(merged.mode, PrecompiledBinaryMode.never);
  });
}
