library uniffi_xforge;

import "dart:async";
import "dart:convert";
import "dart:ffi";
import "dart:typed_data";

import "package:ffi/ffi.dart";

class UniffiInternalError implements Exception {
  static const int bufferOverflow = 0;
  static const int incompleteData = 1;
  static const int unexpectedOptionalTag = 2;
  static const int unexpectedEnumCase = 3;
  static const int unexpectedNullPointer = 4;
  static const int unexpectedRustCallStatusCode = 5;
  static const int unexpectedRustCallError = 6;
  static const int unexpectedStaleHandle = 7;
  static const int rustPanic = 8;
  final int errorCode;
  final String? panicMessage;
  const UniffiInternalError(this.errorCode, this.panicMessage);
  static UniffiInternalError panicked(String message) {
    return UniffiInternalError(rustPanic, message);
  }

  @override
  String toString() {
    switch (errorCode) {
      case bufferOverflow:
        return "UniFfi::BufferOverflow";
      case incompleteData:
        return "UniFfi::IncompleteData";
      case unexpectedOptionalTag:
        return "UniFfi::UnexpectedOptionalTag";
      case unexpectedEnumCase:
        return "UniFfi::UnexpectedEnumCase";
      case unexpectedNullPointer:
        return "UniFfi::UnexpectedNullPointer";
      case unexpectedRustCallStatusCode:
        return "UniFfi::UnexpectedRustCallStatusCode";
      case unexpectedRustCallError:
        return "UniFfi::UnexpectedRustCallError";
      case unexpectedStaleHandle:
        return "UniFfi::UnexpectedStaleHandle";
      case rustPanic:
        return "UniFfi::rustPanic: $panicMessage";
      default:
        return "UniFfi::UnknownError: $errorCode";
    }
  }
}

const int CALL_SUCCESS = 0;
const int CALL_ERROR = 1;
const int CALL_UNEXPECTED_ERROR = 2;

final class RustCallStatus extends Struct {
  @Int8()
  external int code;
  external RustBuffer errorBuf;
}

void checkCallStatus(UniffiRustCallStatusErrorHandler errorHandler,
    Pointer<RustCallStatus> status) {
  if (status.ref.code == CALL_SUCCESS) {
    return;
  } else if (status.ref.code == CALL_ERROR) {
    throw errorHandler.lift(status.ref.errorBuf);
  } else if (status.ref.code == CALL_UNEXPECTED_ERROR) {
    if (status.ref.errorBuf.len > 0) {
      throw UniffiInternalError.panicked(
          FfiConverterString.lift(status.ref.errorBuf));
    } else {
      throw UniffiInternalError.panicked("Rust panic");
    }
  } else {
    throw UniffiInternalError.panicked(
        "Unexpected RustCallStatus code: \${status.ref.code}");
  }
}

T rustCall<T>(T Function(Pointer<RustCallStatus>) callback,
    [UniffiRustCallStatusErrorHandler? errorHandler]) {
  final status = calloc<RustCallStatus>();
  try {
    final result = callback(status);
    checkCallStatus(errorHandler ?? NullRustCallStatusErrorHandler(), status);
    return result;
  } finally {
    calloc.free(status);
  }
}

T rustCallWithLifter<T, F>(
    F Function(Pointer<RustCallStatus>) ffiCall, T Function(F) lifter,
    [UniffiRustCallStatusErrorHandler? errorHandler]) {
  final status = calloc<RustCallStatus>();
  try {
    final rawResult = ffiCall(status);
    checkCallStatus(errorHandler ?? NullRustCallStatusErrorHandler(), status);
    return lifter(rawResult);
  } finally {
    calloc.free(status);
  }
}

class NullRustCallStatusErrorHandler extends UniffiRustCallStatusErrorHandler {
  @override
  Exception lift(RustBuffer errorBuf) {
    errorBuf.free();
    return UniffiInternalError.panicked("Unexpected CALL_ERROR");
  }
}

abstract class UniffiRustCallStatusErrorHandler {
  Exception lift(RustBuffer errorBuf);
}

final class RustBuffer extends Struct {
  @Uint64()
  external int capacity;
  @Uint64()
  external int len;
  external Pointer<Uint8> data;
  static RustBuffer alloc(int size) {
    return rustCall(
        (status) => ffi_uniffi_xforge_rustbuffer_alloc(size, status));
  }

  static RustBuffer fromBytes(ForeignBytes bytes) {
    return rustCall(
        (status) => ffi_uniffi_xforge_rustbuffer_from_bytes(bytes, status));
  }

  void free() {
    rustCall((status) => ffi_uniffi_xforge_rustbuffer_free(this, status));
  }

  RustBuffer reserve(int additionalCapacity) {
    return rustCall((status) =>
        ffi_uniffi_xforge_rustbuffer_reserve(this, additionalCapacity, status));
  }

  Uint8List asUint8List() {
    final dataList = data.asTypedList(len);
    final byteData = ByteData.sublistView(dataList);
    return Uint8List.view(byteData.buffer);
  }

  @override
  String toString() {
    return "RustBuffer{capacity: \$capacity, len: \$len, data: \$data}";
  }
}

RustBuffer toRustBuffer(Uint8List data) {
  final length = data.length;
  final Pointer<Uint8> frameData = calloc<Uint8>(length);
  final pointerList = frameData.asTypedList(length);
  pointerList.setAll(0, data);
  final bytes = calloc<ForeignBytes>();
  bytes.ref.len = length;
  bytes.ref.data = frameData;
  return RustBuffer.fromBytes(bytes.ref);
}

final class ForeignBytes extends Struct {
  @Int32()
  external int len;
  external Pointer<Uint8> data;
  void free() {
    calloc.free(data);
  }
}

class LiftRetVal<T> {
  final T value;
  final int bytesRead;
  const LiftRetVal(this.value, this.bytesRead);
  LiftRetVal<T> copyWithOffset(int offset) {
    return LiftRetVal(value, bytesRead + offset);
  }
}

abstract class FfiConverter<D, F> {
  const FfiConverter();
  D lift(F value);
  F lower(D value);
  D read(ByteData buffer, int offset);
  void write(D value, ByteData buffer, int offset);
  int size(D value);
}

mixin FfiConverterPrimitive<T> on FfiConverter<T, T> {
  @override
  T lift(T value) => value;
  @override
  T lower(T value) => value;
}
Uint8List createUint8ListFromInt(int value) {
  int length = value.bitLength ~/ 8 + 1;
  if (length != 4 && length != 8) {
    length = (value < 0x100000000) ? 4 : 8;
  }
  Uint8List uint8List = Uint8List(length);
  for (int i = length - 1; i >= 0; i--) {
    uint8List[i] = value & 0xFF;
    value >>= 8;
  }
  return uint8List;
}

class FfiConverterString {
  static String lift(RustBuffer buf) {
    return utf8.decoder.convert(buf.asUint8List());
  }

  static RustBuffer lower(String value) {
    return toRustBuffer(Utf8Encoder().convert(value));
  }

  static LiftRetVal<String> read(Uint8List buf) {
    final end = buf.buffer.asByteData(buf.offsetInBytes).getInt32(0) + 4;
    return LiftRetVal(utf8.decoder.convert(buf, 4, end), end);
  }

  static int allocationSize([String value = ""]) {
    return utf8.encoder.convert(value).length + 4;
  }

  static int write(String value, Uint8List buf) {
    final list = utf8.encoder.convert(value);
    buf.buffer.asByteData(buf.offsetInBytes).setInt32(0, list.length);
    buf.setAll(4, list);
    return list.length + 4;
  }
}

const int UNIFFI_RUST_FUTURE_POLL_READY = 0;
const int UNIFFI_RUST_FUTURE_POLL_MAYBE_READY = 1;
typedef UniffiRustFutureContinuationCallback = Void Function(Uint64, Int8);
Future<T> uniffiRustCallAsync<T, F>(
  Pointer<Void> Function() rustFutureFunc,
  void Function(
          Pointer<Void>,
          Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
          Pointer<Void>)
      pollFunc,
  F Function(Pointer<Void>, Pointer<RustCallStatus>) completeFunc,
  void Function(Pointer<Void>) freeFunc,
  T Function(F) liftFunc, [
  UniffiRustCallStatusErrorHandler? errorHandler,
]) async {
  final rustFuture = rustFutureFunc();
  final completer = Completer<int>();
  late final NativeCallable<UniffiRustFutureContinuationCallback> callback;
  void poll() {
    pollFunc(
      rustFuture,
      callback.nativeFunction,
      Pointer<Void>.fromAddress(0),
    );
  }

  void onResponse(int _idx, int pollResult) {
    if (pollResult == UNIFFI_RUST_FUTURE_POLL_READY) {
      completer.complete(pollResult);
    } else {
      poll();
    }
  }

  callback =
      NativeCallable<UniffiRustFutureContinuationCallback>.listener(onResponse);
  try {
    poll();
    await completer.future;
    callback.close();
    final status = calloc<RustCallStatus>();
    try {
      final result = completeFunc(rustFuture, status);
      return liftFunc(result);
    } finally {
      calloc.free(status);
    }
  } finally {
    freeFunc(rustFuture);
  }
}

class UniffiHandleMap<T> {
  final Map<int, T> _map = {};
  int _counter = 1;
  int insert(T obj) {
    final handle = _counter;
    _counter += 2;
    _map[handle] = obj;
    return handle;
  }

  T get(int handle) {
    final obj = _map[handle];
    if (obj == null) {
      throw UniffiInternalError(
          UniffiInternalError.unexpectedStaleHandle, "Handle not found");
    }
    return obj;
  }

  void remove(int handle) {
    if (_map.remove(handle) == null) {
      throw UniffiInternalError(
          UniffiInternalError.unexpectedStaleHandle, "Handle not found");
    }
  }
}

const _uniffiAssetId = "package:uniffi_xforge/uniffi:uniffi_xforge_ffi";
String ping(
  String name,
) {
  return rustCallWithLifter(
      (status) => uniffi_uniffi_xforge_fn_func_ping(
          FfiConverterString.lower(name), status),
      FfiConverterString.lift,
      null);
}

@Native<RustBuffer Function(RustBuffer, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external RustBuffer uniffi_uniffi_xforge_fn_func_ping(
    RustBuffer name, Pointer<RustCallStatus> uniffiStatus);

@Native<RustBuffer Function(Uint64, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external RustBuffer ffi_uniffi_xforge_rustbuffer_alloc(
    int size, Pointer<RustCallStatus> uniffiStatus);

@Native<RustBuffer Function(ForeignBytes, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external RustBuffer ffi_uniffi_xforge_rustbuffer_from_bytes(
    ForeignBytes bytes, Pointer<RustCallStatus> uniffiStatus);

@Native<Void Function(RustBuffer, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rustbuffer_free(
    RustBuffer buf, Pointer<RustCallStatus> uniffiStatus);

@Native<RustBuffer Function(RustBuffer, Uint64, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external RustBuffer ffi_uniffi_xforge_rustbuffer_reserve(
    RustBuffer buf, int additional, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_u8(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_u8(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_u8(Pointer<Void> handle);

@Native<Uint8 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_u8(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_i8(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_i8(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_i8(Pointer<Void> handle);

@Native<Int8 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_i8(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_u16(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_u16(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_u16(Pointer<Void> handle);

@Native<Uint16 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_u16(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_i16(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_i16(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_i16(Pointer<Void> handle);

@Native<Int16 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_i16(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_u32(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_u32(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_u32(Pointer<Void> handle);

@Native<Uint32 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_u32(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_i32(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_i32(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_i32(Pointer<Void> handle);

@Native<Int32 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_i32(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_u64(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_u64(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_u64(Pointer<Void> handle);

@Native<Uint64 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_u64(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_i64(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_i64(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_i64(Pointer<Void> handle);

@Native<Int64 Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_rust_future_complete_i64(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_f32(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_f32(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_f32(Pointer<Void> handle);

@Native<Float Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external double ffi_uniffi_xforge_rust_future_complete_f32(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_f64(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_f64(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_f64(Pointer<Void> handle);

@Native<Double Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external double ffi_uniffi_xforge_rust_future_complete_f64(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_rust_buffer(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_rust_buffer(
    Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_rust_buffer(
    Pointer<Void> handle);

@Native<RustBuffer Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external RustBuffer ffi_uniffi_xforge_rust_future_complete_rust_buffer(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<
    Void Function(
        Pointer<Void>,
        Pointer<NativeFunction<UniffiRustFutureContinuationCallback>>,
        Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_poll_void(
    Pointer<Void> handle,
    Pointer<NativeFunction<UniffiRustFutureContinuationCallback>> callback,
    Pointer<Void> callback_data);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_cancel_void(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>)>(assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_free_void(Pointer<Void> handle);

@Native<Void Function(Pointer<Void>, Pointer<RustCallStatus>)>(
    assetId: _uniffiAssetId)
external void ffi_uniffi_xforge_rust_future_complete_void(
    Pointer<Void> handle, Pointer<RustCallStatus> uniffiStatus);

@Native<Uint16 Function()>(assetId: _uniffiAssetId)
external int uniffi_uniffi_xforge_checksum_func_ping();

@Native<Uint32 Function()>(assetId: _uniffiAssetId)
external int ffi_uniffi_xforge_uniffi_contract_version();

void _checkApiVersion() {
  final bindingsVersion = 30;
  final scaffoldingVersion = ffi_uniffi_xforge_uniffi_contract_version();
  if (bindingsVersion != scaffoldingVersion) {
    throw UniffiInternalError.panicked(
        "UniFFI contract version mismatch: bindings version \$bindingsVersion, scaffolding version \$scaffoldingVersion");
  }
}

void _checkApiChecksums() {
  if (uniffi_uniffi_xforge_checksum_func_ping() != 61771) {
    throw UniffiInternalError.panicked("UniFFI API checksum mismatch");
  }
}

void ensureInitialized() {
  _checkApiVersion();
  _checkApiChecksums();
}
