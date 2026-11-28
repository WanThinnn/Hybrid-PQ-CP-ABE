#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PLATFORM="${1:-}"
DEPENDENCY="${2:-all}"
EXPLICIT_JOBS="${3:-}"

if [[ -z "$PLATFORM" ]]; then
  case "$(uname -s)" in
    Darwin) PLATFORM="macos" ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
    *) PLATFORM="linux" ;;
  esac
fi

case "$DEPENDENCY" in
  all|cryptopp|liboqs|rabe|tkn20|covercrypt|covercrypt-test|pqc-underskrift) ;;
  *) echo "Unknown dependency: $DEPENDENCY" >&2; exit 2 ;;
esac

DEPS_DIR="$ROOT/build/deps/$PLATFORM"
WORK_DIR="$DEPS_DIR/.work"
INCLUDE_DIR="$DEPS_DIR/include"
LIB_DIR="$DEPS_DIR/lib"
BUILD_VERSION="7"

TOTAL_JOBS="${EXPLICIT_JOBS:-${JOBS:-}}"
if [[ -z "$TOTAL_JOBS" && "${MAKEFLAGS:-}" =~ (^|[[:space:]])-?j[[:space:]]*([0-9]+)($|[[:space:]]) ]]; then
  TOTAL_JOBS="${BASH_REMATCH[2]}"
elif [[ -z "$TOTAL_JOBS" && "${MAKEFLAGS:-}" =~ --jobs(=|[[:space:]])([0-9]+) ]]; then
  TOTAL_JOBS="${BASH_REMATCH[2]}"
fi
if [[ -z "$TOTAL_JOBS" ]]; then
  if command -v nproc >/dev/null 2>&1; then
    TOTAL_JOBS="$(nproc)"
  elif command -v sysctl >/dev/null 2>&1; then
    TOTAL_JOBS="$(sysctl -n hw.ncpu 2>/dev/null || echo 4)"
  else
    TOTAL_JOBS=4
  fi
fi
if ! [[ "$TOTAL_JOBS" =~ ^[1-9][0-9]*$ ]]; then
  echo "Invalid job count: $TOTAL_JOBS" >&2
  exit 2
fi

# Nested tools receive explicit job counts. Do not leak GNU Make jobserver FDs.
unset MAKEFLAGS MFLAGS MAKELEVEL
INNER_JOBS="$TOTAL_JOBS"

required=(cmake make cargo rustc go python3)
if [[ "$DEPENDENCY" == "pqc-underskrift" ]]; then required=(cargo rustc python3); fi
if [[ "$PLATFORM" == "macos" ]]; then
  required+=(clang++)
else
  required+=(gcc g++)
fi
for tool in "${required[@]}"; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Missing required tool: $tool" >&2; exit 1; }
done

mkdir -p "$WORK_DIR" "$INCLUDE_DIR" "$LIB_DIR"

LINUX_C_HARDENING=""
LINUX_CXX_HARDENING=""
LINUX_LD_HARDENING=""
if [[ "$PLATFORM" == "linux" ]]; then
  LINUX_C_HARDENING="$(python3 scripts/linux-hardening.py --compiler "${CC:-gcc}" --language c --format flags)"
  LINUX_CXX_HARDENING="$(python3 scripts/linux-hardening.py --compiler g++ --language c++ --format flags)"
  LINUX_LD_HARDENING="-Wl,-z,noexecstack -Wl,-z,relro -Wl,-z,now"
  echo "Linux C hardening: $LINUX_C_HARDENING"
  echo "Linux C++ hardening: $LINUX_CXX_HARDENING"
fi

linux_rust_hardening() {
  if [[ "$PLATFORM" == "linux" ]]; then
    # Preserve encoded user flags (Cargo gives them precedence over RUSTFLAGS).
    export CARGO_ENCODED_RUSTFLAGS="$(python3 -c 'import os; flags = os.environ["CARGO_ENCODED_RUSTFLAGS"].split(chr(31)) if "CARGO_ENCODED_RUSTFLAGS" in os.environ else os.environ.get("RUSTFLAGS", "").split(); print(chr(31).join([f for f in flags if f] + ["-Crelro-level=full", "-Clink-arg=-Wl,-z,noexecstack"]))')"
  fi
}

if command -v sha256sum >/dev/null 2>&1; then
  hash_text() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }
  hash_file() { sha256sum "$1" | cut -d' ' -f1; }
else
  hash_text() { printf '%s' "$1" | shasum -a 256 | cut -d' ' -f1; }
  hash_file() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

sccache_prefix=""
if command -v sccache >/dev/null 2>&1; then
  sccache_prefix="sccache"
fi

fingerprint_file() {
  printf '%s/%s/.fingerprint' "$LIB_DIR" "$1"
}

native_fingerprint() {
  python3 scripts/native-dependency-cache.py fingerprint "$PLATFORM" "$1" "$2|linux_c=$LINUX_C_HARDENING|linux_cxx=$LINUX_CXX_HARDENING|linux_ld=$LINUX_LD_HARDENING"
}

cache_matches() {
  local name="$1"
  local fingerprint="$2"
  shift 2
  python3 scripts/native-dependency-cache.py check "$PLATFORM" "$name" "$fingerprint" || return 1
  local output
  for output in "$@"; do
    [[ -f "$output" ]] || return 1
  done
  return 0
}

write_fingerprint() {
  local name="$1"
  local fingerprint="$2"
  python3 scripts/native-dependency-cache.py record "$PLATFORM" "$name" "$fingerprint"
  mkdir -p "$LIB_DIR/$name"
  {
    printf 'fingerprint=%s\n' "$fingerprint"
    printf 'platform=%s\n' "$PLATFORM"
    printf 'build_version=%s\n' "$BUILD_VERSION"
  } > "$(fingerprint_file "$name")"
}

build_cryptopp() {
  local upstream compiler fingerprint cxx work outlib
  upstream="$(hash_file vendor/cryptopp/UPSTREAM.md)"
  if [[ "$PLATFORM" == "macos" ]]; then
    compiler="$(clang++ --version | head -n 1)"
    cxx="clang++"
  else
    compiler="$(g++ --version | head -n 1)"
    cxx="g++"
  fi
  fingerprint="$(native_fingerprint cryptopp "cryptopp|$BUILD_VERSION|$PLATFORM|$upstream|$compiler|-O2|-fPIC|static+shared|staged-worktree")"
  outlib="$LIB_DIR/cryptopp/libcryptopp.a"
  local shared_out
  if [[ "$PLATFORM" == "macos" ]]; then
    shared_out="$LIB_DIR/cryptopp/libcryptopp.dylib"
  else
    shared_out="$LIB_DIR/cryptopp/libcryptopp.so"
  fi

  if cache_matches cryptopp "$fingerprint" "$outlib" "$shared_out" "$INCLUDE_DIR/cryptopp/aes.h"; then
    echo "==> Crypto++: cached"
    return
  fi

  echo "==> Crypto++ ($INNER_JOBS jobs)"
  work="$WORK_DIR/cryptopp"
  rm -rf "$work" "$INCLUDE_DIR/cryptopp" "$LIB_DIR/cryptopp"
  mkdir -p "$work" "$INCLUDE_DIR/cryptopp" "$LIB_DIR/cryptopp"

  # Crypto++ GNUmakefile relies on wildcard *.cpp in CWD, so use a disposable
  # staged worktree rather than producing objects inside vendor/cryptopp.
  cp -a vendor/cryptopp/. "$work/"
  make -C "$work" clean >/dev/null 2>&1 || true

  [[ -n "$sccache_prefix" ]] && cxx="sccache $cxx"
  if [[ "$PLATFORM" == "macos" ]]; then
    make -C "$work" -j"$INNER_JOBS" static libcryptopp.dylib CXX="$cxx" CXXFLAGS="-DNDEBUG -O2 -fPIC"
  else
    make -C "$work" -j"$INNER_JOBS" static libcryptopp.so CXX="$cxx" CXXFLAGS="-DNDEBUG -O2 -fPIC $LINUX_CXX_HARDENING" LDFLAGS="${LDFLAGS:-} $LINUX_LD_HARDENING"
  fi

  cp "$work/libcryptopp.a" "$outlib"
  if [[ "$PLATFORM" == "macos" ]]; then
    cp -L "$work/libcryptopp.dylib" "$shared_out"
  else
    cp -L "$work/libcryptopp.so" "$shared_out"
  fi
  cp "$work"/*.h "$INCLUDE_DIR/cryptopp/"
  write_fingerprint cryptopp "$fingerprint"
  rm -rf "$work"
}

build_liboqs() {
  local upstream compiler generator fingerprint work_static work_shared stage_static stage_shared outlib shared_out
  upstream="$(hash_file vendor/liboqs/UPSTREAM.md)"
  if [[ "$PLATFORM" == "macos" ]]; then
    compiler="$(clang --version | head -n 1)"
  else
    compiler="$(gcc --version | head -n 1)"
  fi
  if command -v ninja >/dev/null 2>&1; then
    generator="Ninja"
  else
    generator="Unix Makefiles"
  fi

  fingerprint="$(native_fingerprint liboqs "liboqs|$BUILD_VERSION|$PLATFORM|$upstream|$compiler|$generator|ML-KEM+ML-DSA|minimal|static+shared|PIC|no-openssl")"
  outlib="$LIB_DIR/liboqs/liboqs.a"
  if [[ "$PLATFORM" == "macos" ]]; then
    shared_out="$LIB_DIR/liboqs/liboqs.dylib"
  else
    shared_out="$LIB_DIR/liboqs/liboqs.so"
  fi

  if cache_matches liboqs "$fingerprint" "$outlib" "$shared_out" "$INCLUDE_DIR/liboqs/oqs/oqs.h"; then
    echo "==> liboqs: cached"
    return
  fi

  echo "==> liboqs ($INNER_JOBS jobs, $generator, static + shared)"
  work_static="$WORK_DIR/liboqs-static"
  work_shared="$WORK_DIR/liboqs-shared"
  stage_static="$WORK_DIR/liboqs-stage-static"
  stage_shared="$WORK_DIR/liboqs-stage-shared"

  rm -rf "$INCLUDE_DIR/oqs" "$INCLUDE_DIR/liboqs" "$LIB_DIR/liboqs" "$stage_static" "$stage_shared" "$work_static" "$work_shared"
  mkdir -p "$work_static" "$work_shared" "$stage_static" "$stage_shared" "$INCLUDE_DIR/liboqs" "$LIB_DIR/liboqs"

  local cmake_args=(
    -S vendor/liboqs
    -G "$generator"
    -DCMAKE_BUILD_TYPE=Release
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    -DOQS_BUILD_ONLY_LIB=ON
    -DOQS_USE_OPENSSL=OFF
    "-DOQS_MINIMAL_BUILD=KEM_ml_kem_512;KEM_ml_kem_768;KEM_ml_kem_1024;SIG_ml_dsa_44;SIG_ml_dsa_65;SIG_ml_dsa_87"
  )
  if [[ "$PLATFORM" == "linux" ]]; then
    cmake_args+=("-DCMAKE_C_COMPILER=${CC:-gcc}" "-DCMAKE_CXX_COMPILER=g++"
      "-DCMAKE_C_FLAGS=${CFLAGS:-} $LINUX_C_HARDENING"
      "-DCMAKE_CXX_FLAGS=${CXXFLAGS:-} $LINUX_CXX_HARDENING"
      "-DCMAKE_SHARED_LINKER_FLAGS=${LDFLAGS:-} $LINUX_LD_HARDENING")
  fi
  if [[ -n "$sccache_prefix" ]]; then
    cmake_args+=(-DCMAKE_C_COMPILER_LAUNCHER=sccache -DCMAKE_CXX_COMPILER_LAUNCHER=sccache)
  fi

  # Static and shared use separate CMake/Ninja build trees. This avoids
  # regeneration loops when BUILD_SHARED_LIBS changes between configurations.
  cmake "${cmake_args[@]}" -B "$work_static" -DCMAKE_INSTALL_PREFIX="$stage_static" -DBUILD_SHARED_LIBS=OFF
  cmake --build "$work_static" --config Release --parallel "$INNER_JOBS"
  cmake --install "$work_static" --config Release

  local built_static
  built_static="$(find "$stage_static" -type f -name 'liboqs.a' -print -quit)"
  [[ -n "$built_static" ]] || { echo "liboqs.a not found after static build" >&2; exit 1; }
  cp "$built_static" "$outlib"
  cp -R "$stage_static/include/oqs" "$INCLUDE_DIR/liboqs/oqs"

  # Shared pass uses its own cache. sccache still reuses identical compiler
  # outputs across the two build trees when available.
  cmake "${cmake_args[@]}" -B "$work_shared" -DCMAKE_INSTALL_PREFIX="$stage_shared" -DBUILD_SHARED_LIBS=ON
  cmake --build "$work_shared" --config Release --parallel "$INNER_JOBS"
  cmake --install "$work_shared" --config Release

  local built_shared
  if [[ "$PLATFORM" == "macos" ]]; then
    built_shared="$(find "$stage_shared" -type f -name 'liboqs*.dylib' -print -quit)"
  else
    built_shared="$(find "$stage_shared" -type f -name 'liboqs.so*' -print -quit)"
  fi
  [[ -n "$built_shared" ]] || { echo "liboqs shared library not found after build" >&2; exit 1; }
  cp -L "$built_shared" "$shared_out"

  write_fingerprint liboqs "$fingerprint"
  rm -rf "$work_static" "$work_shared" "$stage_static" "$stage_shared"
}

build_rabe() (
  linux_rust_hardening
  local upstream lock header rustc_info fingerprint work outlib
  upstream="$(hash_file vendor/librabe-ffi/UPSTREAM.md)"
  lock="$(hash_file vendor/librabe-ffi/Cargo.lock)"
  header="$(hash_file vendor/librabe-ffi/rabe.h)"
  rustc_info="$(rustc -vV)"
  fingerprint="$(native_fingerprint rabe "rabe|$BUILD_VERSION|$PLATFORM|$upstream|$lock|$header|$rustc_info|release|panic=unwind|lto=false")"
  outlib="$LIB_DIR/rabe/librabe_ffi.a"

  local shared_name="librabe_ffi.so"
  [[ "$PLATFORM" == "macos" ]] && shared_name="librabe_ffi.dylib"
  if cache_matches rabe "$fingerprint" "$outlib" "$LIB_DIR/rabe/$shared_name" "$INCLUDE_DIR/rabe/rabe.h"; then
    echo "==> librabe-ffi: cached"
    return
  fi

  echo "==> librabe-ffi ($INNER_JOBS jobs)"
  work="$WORK_DIR/rabe-target"
  rm -rf "$work" "$INCLUDE_DIR/rabe" "$LIB_DIR/rabe"
  mkdir -p "$INCLUDE_DIR/rabe" "$LIB_DIR/rabe"

  if [[ -n "$sccache_prefix" ]]; then
    CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_PANIC=unwind RUSTC_WRAPPER=sccache CARGO_TARGET_DIR="$work" cargo build --locked --jobs "$INNER_JOBS" --manifest-path vendor/librabe-ffi/Cargo.toml --release
  else
    CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_PANIC=unwind CARGO_TARGET_DIR="$work" cargo build --locked --jobs "$INNER_JOBS" --manifest-path vendor/librabe-ffi/Cargo.toml --release
  fi

  cp "$work/release/librabe_ffi.a" "$outlib"
  cp vendor/librabe-ffi/rabe.h "$INCLUDE_DIR/rabe/rabe.h"
  if [[ "$PLATFORM" == "macos" ]]; then
    [[ -f "$work/release/librabe_ffi.dylib" ]] && cp "$work/release/librabe_ffi.dylib" "$LIB_DIR/rabe/"
  else
    [[ -f "$work/release/librabe_ffi.so" ]] && cp "$work/release/librabe_ffi.so" "$LIB_DIR/rabe/"
  fi

  write_fingerprint rabe "$fingerprint"
  rm -rf "$work"
)

build_covercrypt() (
  linux_rust_hardening
  local digest fingerprint work outlib shared_name outshared
  digest="$( { find vendor/cover_crypt/src bindings/ffi/covercrypt/src -type f; printf '%s\n' vendor/cover_crypt/Cargo.toml vendor/cover_crypt/Cargo.lock vendor/cover_crypt/UPSTREAM.md bindings/ffi/covercrypt/Cargo.toml bindings/ffi/covercrypt/Cargo.lock bindings/ffi/covercrypt/include/covercrypt_ffi.h; } | LC_ALL=C sort | while IFS= read -r input; do hash_file "$input"; done )"
  fingerprint="$(native_fingerprint covercrypt "covercrypt|$BUILD_VERSION|$PLATFORM|$(rustc -vV)|$digest|curve25519|mlkem-768")"
  outlib="$LIB_DIR/covercrypt/libcovercrypt_ffi.a"
  if [[ "$PLATFORM" == macos ]]; then
    shared_name="libcovercrypt_ffi.dylib"
  else
    shared_name="libcovercrypt_ffi.so"
  fi
  outshared="$LIB_DIR/covercrypt/$shared_name"
  if cache_matches covercrypt "$fingerprint" "$outlib" "$outshared" "$INCLUDE_DIR/covercrypt/covercrypt_ffi.h"; then
    echo "==> CoverCrypt: cached"
    return
  fi
  echo "==> CoverCrypt ($INNER_JOBS jobs, Curve25519 + ML-KEM-768, static + shared C ABI)"
  work="$WORK_DIR/covercrypt-target"
  # Cargo's timestamp cache must not defeat our content-based invalidation.
  rm -rf "$work" "$LIB_DIR/covercrypt" "$INCLUDE_DIR/covercrypt"
  mkdir -p "$LIB_DIR/covercrypt" "$INCLUDE_DIR/covercrypt"
  CARGO_TARGET_DIR="$work" cargo build --locked --release --jobs "$INNER_JOBS" --manifest-path bindings/ffi/covercrypt/Cargo.toml
  cp "$work/release/libcovercrypt_ffi.a" "$outlib"
  cp "$work/release/$shared_name" "$outshared"
  cp bindings/ffi/covercrypt/include/covercrypt_ffi.h "$INCLUDE_DIR/covercrypt/"
  write_fingerprint covercrypt "$fingerprint"
)

build_pqc_underskrift() (
  # Stage the C ABI bridge without linking it into the C++ core yet.
  linux_rust_hardening
  local rust_version fingerprint work outlib
  rust_version="$(rustc -vV)"
  fingerprint="$(native_fingerprint pqc-underskrift "pqc-underskrift|$BUILD_VERSION|$PLATFORM|$rust_version|release|ffi-static+shared|external-composite|panic=unwind")"
  outlib="$LIB_DIR/pqc-underskrift/libpdf_signature_ffi.a"
  local shared_name=libpdf_signature_ffi.so
  [[ "$PLATFORM" == "macos" ]] && shared_name=libpdf_signature_ffi.dylib
  local shared="$LIB_DIR/pqc-underskrift/$shared_name"
  local header="$INCLUDE_DIR/pqc-underskrift/pdf_signature_ffi.h"
  if cache_matches pqc-underskrift "$fingerprint" "$outlib" "$shared" "$header"; then
    echo '==> pqc-underskrift: cached'
    return
  fi
  echo "==> pqc-underskrift ($INNER_JOBS jobs, external-composite, C ABI static + shared)"
  work="$WORK_DIR/pqc-underskrift-target"
  rm -rf "$work" "$LIB_DIR/pqc-underskrift" "$INCLUDE_DIR/pqc-underskrift"
  mkdir -p "$LIB_DIR/pqc-underskrift" "$INCLUDE_DIR/pqc-underskrift"
  CARGO_TARGET_DIR="$work" CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_PANIC=unwind \
    cargo build --locked --release --jobs "$INNER_JOBS" --manifest-path bindings/ffi/pdf-signature/Cargo.toml
  test -f "$work/release/libpdf_signature_ffi.a" || { echo 'PDF C ABI archive not found' >&2; exit 1; }
  test -f "$work/release/$shared_name" || { echo 'PDF C ABI shared library not found' >&2; exit 1; }
  cp "$work/release/libpdf_signature_ffi.a" "$outlib"
  cp "$work/release/$shared_name" "$shared"
  cp bindings/ffi/pdf-signature/include/pdf_signature_ffi.h "$header"
  write_fingerprint pqc-underskrift "$fingerprint"
  rm -rf "$work"
)

test_covercrypt() {
  CARGO_TARGET_DIR="$ROOT/build/artifacts/$PLATFORM/covercrypt-rust-tests" cargo test --locked --release --jobs "$INNER_JOBS" --manifest-path vendor/cover_crypt/Cargo.toml --lib --no-default-features --features curve25519,mlkem-768
}

build_tkn20() (
  local upstream gomod gosum bridge gover cc ccver fingerprint work outlib
  upstream="$(hash_file vendor/circl/UPSTREAM.md)"
  gomod="$(hash_file bindings/ffi/tkn20/go.mod)"
  gosum="$(hash_file bindings/ffi/tkn20/go.sum)"
  bridge="$(hash_file bindings/ffi/tkn20/cpabe_tkn20.go)"
  gover="$(go version)"
  cc="${CC:-cc}"
  ccver="$($cc --version | head -n 1)"
  if [[ "$PLATFORM" == "linux" ]]; then
    local cgo_hardening
    cgo_hardening="$(python3 scripts/linux-hardening.py --compiler "$cc" --language c --format flags)"
    export CGO_CFLAGS="${CGO_CFLAGS:--O2 -g} $cgo_hardening"
    export CGO_CXXFLAGS="${CGO_CXXFLAGS:--O2 -g} $LINUX_CXX_HARDENING"
    export CGO_LDFLAGS="${CGO_LDFLAGS:-} $LINUX_LD_HARDENING"
  fi
  fingerprint="$(native_fingerprint tkn20 "tkn20|$BUILD_VERSION|$PLATFORM|$upstream|$gomod|$gosum|$bridge|$gover|$ccver")"
  outlib="$LIB_DIR/tkn20/libcpabe_tkn20.a"

  local shared_name="libcpabe_tkn20.so"
  [[ "$PLATFORM" == "macos" ]] && shared_name="libcpabe_tkn20.dylib"
  if cache_matches tkn20 "$fingerprint" "$outlib" "$LIB_DIR/tkn20/$shared_name" "$INCLUDE_DIR/tkn20/cpabe_tkn20.h"; then
    echo "==> CIRCL/TKN20: cached"
    return
  fi

  echo "==> CIRCL/TKN20 ($INNER_JOBS jobs)"
  work="$WORK_DIR/tkn20"
  rm -rf "$work" "$INCLUDE_DIR/tkn20" "$LIB_DIR/tkn20"
  mkdir -p "$work" "$INCLUDE_DIR/tkn20" "$LIB_DIR/tkn20"

  (
    cd bindings/ffi/tkn20
    CGO_ENABLED=1 CC="$cc" go build -p "$INNER_JOBS" -buildmode=c-archive -o "$work/libcpabe_tkn20.a" .
  )
  cp "$work/libcpabe_tkn20.a" "$outlib"
  cp "$work/libcpabe_tkn20.h" "$INCLUDE_DIR/tkn20/cpabe_tkn20.h"

  if [[ "$PLATFORM" == "macos" ]]; then
    (
      cd bindings/ffi/tkn20
      CGO_ENABLED=1 CC="$cc" go build -p "$INNER_JOBS" -buildmode=c-shared -o "$work/libcpabe_tkn20.dylib" .
    )
    cp "$work/libcpabe_tkn20.dylib" "$LIB_DIR/tkn20/"
  else
    (
      cd bindings/ffi/tkn20
      CGO_ENABLED=1 CC="$cc" go build -p "$INNER_JOBS" -buildmode=c-shared -o "$work/libcpabe_tkn20.so" .
    )
    cp "$work/libcpabe_tkn20.so" "$LIB_DIR/tkn20/"
  fi

  write_fingerprint tkn20 "$fingerprint"
  rm -rf "$work"
)

print_outputs() {
  echo
  echo "================================================================"
  echo " Dependency build complete: $PLATFORM"
  echo " Includes : build/deps/$PLATFORM/include/"
  echo " Libraries: build/deps/$PLATFORM/lib/"
  echo "----------------------------------------------------------------"
  local dep
  for dep in cryptopp liboqs rabe tkn20 covercrypt pqc-underskrift; do
    echo " [$dep]"
    echo "   include: build/deps/$PLATFORM/include/$dep/"
    echo "   lib    : build/deps/$PLATFORM/lib/$dep/"
    if [[ -d "$LIB_DIR/$dep" ]]; then
      find "$LIB_DIR/$dep" -maxdepth 1 -type f ! -name '.fingerprint' -printf '            %f\n' 2>/dev/null ||         find "$LIB_DIR/$dep" -maxdepth 1 -type f ! -name '.fingerprint' -exec basename {} \;
    fi
  done
  echo "================================================================"
}

write_ready_marker() {
  python3 scripts/native-dependency-cache.py ready "$PLATFORM" all
}

run_dep() {
  local dep="$1"
  local jobs="$2"
  JOBS="$jobs" MAKEFLAGS= MFLAGS= MAKELEVEL=0 bash "$0" "$PLATFORM" "$dep" "$jobs"
}

run_all() {
  local deps=(liboqs cryptopp rabe tkn20 covercrypt pqc-underskrift)
  local count="${#deps[@]}"
  local status=0
  local pids=()
  local pid dep jobs i

  if (( TOTAL_JOBS < count )); then
    echo "Dependency worker budget: total=$TOTAL_JOBS; up to $TOTAL_JOBS dependencies concurrently (1 job each)"
    local running=0
    for dep in "${deps[@]}"; do
      run_dep "$dep" 1 &
      pids+=("$!")
      ((running+=1))
      if (( running == TOTAL_JOBS )); then
        for pid in "${pids[@]}"; do
          if ! wait "$pid"; then status=1; fi
        done
        pids=()
        running=0
      fi
    done
    for pid in "${pids[@]}"; do
      if ! wait "$pid"; then status=1; fi
    done
  else
    local base=$(( TOTAL_JOBS / count ))
    local extra=$(( TOTAL_JOBS % count ))

    echo "Dependency worker budget: total=$TOTAL_JOBS"
    for ((i=0; i<count; i++)); do
      jobs="$base"
      if (( i < extra )); then ((jobs+=1)); fi
      echo "  ${deps[$i]}: $jobs"
      run_dep "${deps[$i]}" "$jobs" &
      pids+=("$!")
    done

    for pid in "${pids[@]}"; do
      if ! wait "$pid"; then status=1; fi
    done
  fi

  rm -rf "$WORK_DIR"
  (( status == 0 )) || return "$status"

  # Migrate/clean legacy dependency build layout. Only the stable consumer
  # surface remains at build/deps/<platform>/{include,lib}.
  find "$DEPS_DIR" -mindepth 1 -maxdepth 1 \
    ! -name include ! -name lib -exec rm -rf {} +

  write_ready_marker
  print_outputs
}

case "$DEPENDENCY" in
  cryptopp) build_cryptopp ;;
  liboqs) build_liboqs ;;
  rabe) build_rabe ;;
  tkn20) build_tkn20 ;;
  covercrypt) build_covercrypt ;;
  pqc-underskrift) build_pqc_underskrift ;;
  covercrypt-test) test_covercrypt ;;
  all) run_all ;;
esac
