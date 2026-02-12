import 'dart:io';

import 'package:ed25519_edwards/ed25519_edwards.dart';

import '../../lib/src/precompiled/util.dart';

Future<int> runKeygen(List<String> args) async {
  if (args.isNotEmpty) {
    if (args.length == 1 && (args.first == '--help' || args.first == '-h')) {
      _printUsage();
      return 0;
    }
    stderr.writeln('Unknown arguments: ${args.join(' ')}');
    _printUsage();
    return 2;
  }

  final keypair = generateKey();
  final publicKeyHex = hexEncode(keypair.publicKey.bytes);
  final privateKeyHex = hexEncode(keypair.privateKey.bytes);

  if (keypair.publicKey.bytes.length != PublicKeySize ||
      keypair.privateKey.bytes.length != PrivateKeySize) {
    stderr.writeln(
      'Key generation failed: expected $PublicKeySize-byte public key and '
      '$PrivateKeySize-byte private key.',
    );
    return 1;
  }

  stdout.writeln('public_key=$publicKeyHex');
  stdout.writeln('private_key=$privateKeyHex');
  return 0;
}

void _printUsage() {
  stdout.writeln('Usage: dart run libforge_dart keygen');
  stdout.writeln('Outputs:');
  stdout.writeln('  public_key=<32-byte hex>');
  stdout.writeln('  private_key=<64-byte hex (seed + public)>');
}
