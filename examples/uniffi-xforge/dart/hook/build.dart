import 'package:hooks/hooks.dart';
import 'package:libforge_dart/libforge_dart.dart';
import 'package:logging/logging.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    await const PrecompiledBuilder(
      assetName: 'uniffi:uniffi_xforge_ffi',
      cratePath: '..',
      fallback: _runLocalBuild,
    ).run(input: input, output: output);
  });
}

Future<void> _runLocalBuild(
  BuildInput input,
  BuildOutputBuilder output,
  List<AssetRouting> assetRouting,
  Logger? logger,
) async {
  await const RustBuilder(
    assetName: 'uniffi:uniffi_xforge_ffi',
    cratePath: '..',
  ).run(
    input: input,
    output: output,
    assetRouting: assetRouting,
    logger: logger,
  );
}
