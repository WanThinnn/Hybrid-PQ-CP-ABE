---
title: Download
description: Download the prebuilt PQ-CP-ABE native SDK for Windows, Linux or macOS.
icon: lucide/download
---

# Download a prebuilt native SDK

For most users, downloading a prebuilt SDK is faster than building the native library from source.

## Choose your platform

Choose the package for your platform. These links point directly to the archive, so the download starts without opening the GitHub Releases page.

<div class="pq-download-grid" markdown>

[:lucide-download: **Windows x86-64**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="windows" data-arch="x86_64" }

[:lucide-download: **Linux x86-64**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="linux" data-arch="x86_64" }

[:lucide-download: **macOS 12+ · Apple Silicon**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="macos" data-arch="arm64" }

[:lucide-download: **macOS 12+ · Intel**](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest){ .md-button .pq-download-button data-platform="macos" data-arch="x86_64" }

</div>

## Use the SDK

Extract the archive and use its `include/` directory for headers and `lib/` directory for linking. Keep the headers and native libraries from the **same release**.

!!! note
    The download buttons resolve the latest stable GitHub Release at page load and link to its real versioned archive, for example `libhybrid-pq-cp-abe_linux_x86_64_v6.1.0.zip`. If JavaScript is unavailable, the buttons fall back to the latest Release page. The macOS builds target macOS 12.0 or later.

If a suitable prebuilt archive is not available for your OS or architecture, follow the [build-from-source guide](../native/build.md).


## Example

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

Use a normal **PowerShell** window. The SDK is already built; you do not need
to rebuild the library or install its source-build dependencies.

#### 1. Select your extracted SDK

Set `$SDK` to the folder containing `bin`, `include` and `lib`. Change the path
and version below to match your download:

```powershell
$SDK = "$env:USERPROFILE\Downloads\libhybrid-pq-cp-abe_win_x86_64_vX.Y.Z\libhybrid-pq-cp-abe_win_x86_64_vX.Y.Z"
& "$SDK\bin\pq-cp-abe.exe" --help
```

If you see the command help, the SDK's ready-made CLI works. You can use it
immediately by following the [CLI examples](../native/cli.md). No compiler is needed
for this option.

#### 2. Compile demo.c if you want to use the C API

To turn your own C source into an executable, you need a C compiler. This
example uses **Windows GCC**. Check that it is available in PowerShell:

```powershell
gcc --version
```

If `gcc` is not found, follow the [Windows C compiler setup](../native/build.md#windows-c-compiler-setup),
then return here. If you only want to use the bundled CLI, skip this step.

Create a fresh example folder and save the C code above as `demo.c` inside it:

```powershell
New-Item -ItemType Directory -Path "$env:USERPROFILE\Downloads\pq-cp-abe-demo"
Set-Location "$env:USERPROFILE\Downloads\pq-cp-abe-demo"
```

Compile the example and copy the two runtime files next to it:

```powershell
gcc -std=c17 -DUSE_DLL "-I$SDK/include" demo.c "$SDK/lib/libhybrid-pq-cp-abe_dll.lib" -o demo.exe
if ($LASTEXITCODE -ne 0) { throw "Compilation failed" }
Copy-Item "$SDK\lib\libhybrid-pq-cp-abe.dll" .
Copy-Item "$SDK\lib\libcpabe_tkn20.dll" .
```

#### 3. Run the example

In the same example folder, create an input file, run the program and compare
the recovered file:

```powershell
Set-Content -Path message.txt -Value "hello from the native SDK" -Encoding ascii
.\demo.exe
if ($LASTEXITCODE -ne 0) { throw "The demo failed" }
fc.exe /b message.txt recovered.txt
```

If the comparison reports no differences, encryption and decryption succeeded.
Keep the two DLLs beside `demo.exe`. Once built, the program also runs from
ordinary CMD (`demo.exe`) or Git Bash (`./demo.exe`).


### Compile and run on Linux / macOS

Use GCC on Linux or Apple Clang from the Xcode Command Line Tools on macOS.
`SDK` is a shell variable holding the **full path to the extracted SDK folder**,
which contains both `include/` and `lib/`. Replace `vX.Y.Z` with your downloaded
version. Keep these commands in one terminal session.

Create a fresh working folder, then save the C example above as `demo.c` there:

```sh
mkdir -p "$HOME/Downloads/pq-cp-abe-demo"
cd "$HOME/Downloads/pq-cp-abe-demo"
```

#### Linux

Select the extracted Linux x86-64 package and compile against its `.so`:

```sh
SDK="$HOME/Downloads/libhybrid-pq-cp-abe_linux_x86_64_vX.Y.Z"
gcc -std=c17 -Wall -Wextra -I"$SDK/include" demo.c -L"$SDK/lib" -lhybrid-pq-cp-abe -o demo
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
clang -std=c17 -Wall -Wextra -I"$SDK/include" demo.c -L"$SDK/lib" -lhybrid-pq-cp-abe -Wl,-rpath,"$SDK/lib" -o demo
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
