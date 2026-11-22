---
icon: lucide/code-xml
---

# Native C and C++ API

Use the native API when your application links the SDK directly. C and C++
call the same public C ABI. Include `hybrid_pq_cp_abe/hybrid-pq-cp-abe.h` and
link a library built for your platform. [Native build guide](build.md)

## Download a prebuilt native SDK

For most users, downloading a prebuilt SDK is faster than building the native library from source.

Choose the package for your platform. These links point directly to the archive, so the download starts without opening the GitHub Releases page.

<div class="pq-download-grid" markdown>

[:lucide-download: **Windows x86-64**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="windows" data-arch="x86_64" }

[:lucide-download: **Linux x86-64**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="linux" data-arch="x86_64" }

[:lucide-download: **macOS 12+ · Apple Silicon**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="macos" data-arch="arm64" }

[:lucide-download: **macOS 12+ · Intel**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="macos" data-arch="x86_64" }

</div>

Extract the archive and use its `include/` directory for headers and `lib/` directory for linking. Keep the headers and native libraries from the **same release**.

!!! note
    The download buttons resolve the latest stable GitHub Release at page load and link to its real versioned archive, for example `libhybrid-pq-cp-abe_linux_x86_64_v6.1.0.zip`. If JavaScript is unavailable, the buttons fall back to the latest Release page. The macOS builds target macOS 12.0 or later.

If a suitable prebuilt archive is not available for your OS or architecture, follow the [build-from-source guide](build.md).

## Encrypt and decrypt a file

This example uses the current native API to create an AC17 authority, issue a key for
`admin`, and encrypt a file under that policy. Run it in a fresh working
directory. Setup writes `authority_pk.key` and `authority_msk.key`.

Save as `demo.c`.

```c
#include <stdio.h>
#include "hybrid_pq_cp_abe/hybrid-pq-cp-abe.h"

static int succeeded(int status, const char *operation)
{
    if (status == HCPABE_SUCCESS)
        return 1;
    fprintf(stderr, "%s failed (%d): %s\n", operation, status, getErrorMessage(status));
    return 0;
}

int main(void)
{
    const CPABEScheme scheme = CPABE_SCHEME_AC17;
    if (!succeeded(setup_with_scheme("authority", scheme), "setup") ||
        !succeeded(generateSecretKey_with_scheme("authority_msk.key", "admin",
                                                 "user.key", scheme), "keygen") ||
        !succeeded(hybrid_cpabe_encrypt_with_scheme("authority_pk.key", "message.txt",
                                                    "admin", "message.enc", scheme), "encrypt") ||
        !succeeded(hybrid_cpabe_decrypt("user.key", "message.enc", "recovered.txt"), "decrypt"))
        return 1;

    return 0;
}
```

### Compile and run on Windows

Use the **Windows x86-64 SDK** with the **MSVC x64 build tools** and a Windows
SDK. Open **x64 Native Tools Command Prompt for Visual Studio** from the Start
menu. The following commands use **cmd**, not PowerShell. If the compiler is not
installed, follow the [Windows toolchain prerequisites](build.md).

Set `SDK` to the **full path of the extracted folder containing `include/` and
`lib/`**. Replace the user, folder name and `vX.Y.Z` with your actual path:

```cmd
set "SDK=C:\Users\alex\Downloads\libhybrid-pq-cp-abe_windows_x86_64_vX.Y.Z"
```

Create a fresh working folder and save the C example above as `demo.c` inside it:

```cmd
mkdir "%USERPROFILE%\pq-cp-abe-demo"
cd /d "%USERPROFILE%\pq-cp-abe-demo"
```

In the **same x64 Native Tools cmd window**, compile against the DLL import
library, copy both runtime DLLs beside the executable, then run the example:

```cmd
cl /nologo /W4 /TC /std:c11 /MD /DUSE_DLL /I"%SDK%\include" demo.c /Fe:demo.exe /link /LIBPATH:"%SDK%\lib" libhybrid-pq-cp-abe_dll.lib
copy /Y "%SDK%\lib\libhybrid-pq-cp-abe.dll" .
copy /Y "%SDK%\lib\libcpabe_tkn20.dll" .
echo hello from the native SDK> message.txt
demo.exe
fc /b message.txt recovered.txt
```

`/DUSE_DLL` selects the header's DLL-import declarations; `/MD` matches the
SDK's MSVC runtime. Link **`libhybrid-pq-cp-abe_dll.lib`**, the DLL import library,
rather than the separate static archive `libhybrid-pq-cp-abe.lib`.
Keep **both** DLLs beside `demo.exe`, even for this AC17 example. Do not mix
headers, import libraries or DLLs from different releases.

If `demo.exe` succeeds and `fc /b` reports no differences (exit code 0), the
recovered file matches the original byte for byte. A missing `cl` command means
the MSVC toolchain is not installed or the terminal has not been initialized.
A missing-DLL error means the runtime DLLs or the Microsoft Visual C++ x64
runtime are unavailable. See the [native build guide](build.md) for toolchain
setup; using the prebuilt SDK does not require building its dependencies.

### Compile and run on Linux / macOS

Use GCC on Linux or Apple Clang from the Xcode Command Line Tools on macOS.
`SDK` is a shell variable holding the **full path to the extracted SDK folder**,
which contains both `include/` and `lib/`. Replace `vX.Y.Z` with your downloaded
version. Keep these commands in one terminal session.

Create a fresh working folder, then save the C example above as `demo.c` there:

```sh
mkdir -p "$HOME/pq-cp-abe-demo"
cd "$HOME/pq-cp-abe-demo"
```

#### Linux

Select the extracted Linux x86-64 package and compile against its `.so`:

```sh
SDK="$HOME/Downloads/libhybrid-pq-cp-abe_linux_x86_64_vX.Y.Z"
gcc -std=c11 -Wall -Wextra -I"$SDK/include" demo.c -L"$SDK/lib" -lhybrid-pq-cp-abe -o demo
printf '%s\n' 'hello from the native SDK' > message.txt
LD_LIBRARY_PATH="$SDK/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./demo
cmp message.txt recovered.txt
```

`LD_LIBRARY_PATH` makes the SDK's `libhybrid-pq-cp-abe.so` available to the loader
for this run without changing your shell configuration.

#### macOS

Choose the package matching your machine: **arm64** for Apple Silicon or
**x86_64** for Intel. The prebuilt SDK requires macOS 12 or newer. If the compiler
is missing, install the Xcode Command Line Tools with `xcode-select --install`.

```sh
SDK="$HOME/Downloads/libhybrid-pq-cp-abe_macos_arm64_vX.Y.Z"
# On Intel, use the extracted macos_x86_64 folder instead.
clang -std=c11 -Wall -Wextra -I"$SDK/include" demo.c -L"$SDK/lib" -lhybrid-pq-cp-abe -Wl,-rpath,"$SDK/lib" -o demo
printf '%s\n' 'hello from the native SDK' > message.txt
./demo
cmp message.txt recovered.txt
```

The macOS SDK provides `libhybrid-pq-cp-abe.dylib`. The linker embeds `$SDK/lib`
as a runtime search path to resolve the library's `@rpath` install name. If you
move the SDK directory, rebuild the example with its new path. Use a compiler
target matching the downloaded archive, including when running under Rosetta.

#### Check the result

Both Unix examples create the keys, `message.txt`, `message.enc` and
`recovered.txt` in the working folder. If `cmp` prints nothing and exits
successfully, the recovered file matches the original byte for byte.

If you built from repository source with `make shared`, set `SDK` to that
checkout instead, for example `SDK="$HOME/projects/Hybrid-PQ-CP-ABE"`. It must
contain the platform's built shared library under `lib/`. Only `message.enc`
is encrypted; the input and recovered text files contain plaintext.


## Choose the operations you need

| Task | C entry points |
| --- | --- |
| Create an authority and issue a file-based key | `setup_with_scheme`, `generateSecretKey_with_scheme` |
| Create an authority or issue a key in memory | `hybrid_cpabe_setupBuffer_with_scheme`, `hybrid_cpabe_genkeyBuffer_with_scheme` |
| Encrypt with an explicit AES key size | `hybrid_cpabe_encrypt_with_params`, `hybrid_cpabe_encryptBuffer_with_params` |
| Create signing keys and sign or verify files | `hybrid_cpabe_setup_with_signing_keys`, `hybrid_cpabe_encrypt_and_sign_with_params`, `hybrid_cpabe_decrypt_and_verify` |
| Configure CoverCrypt in memory | `hybrid_cpabe_covercrypt_setupBuffer_from_json` |
| Release native output buffers | `freeBuffer` |

For CoverCrypt file setup, set `HCPABE_COVERCRYPT_CONFIG` to the dimensions
JSON path and choose `CPABE_SCHEME_COVERCRYPT`. Decryption detects the
ciphertext's scheme and parameters. Supply a trusted verification key to
the verify API when signature verification is required. [CoverCrypt configuration](cli.md#covercrypt-with-ml-dsa-signatures)

The `_with_params` encryptors emit V3. Use the compatible existing encrypt
functions when an older runtime must read the ciphertext. Release each
library-owned output buffer with `freeBuffer` from the same library.
[Parameter and buffer contracts](parameters.md) · [Public C header](../../include/hybrid_pq_cp_abe/hybrid-pq-cp-abe.h)
