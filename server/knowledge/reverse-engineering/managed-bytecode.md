# Managed bytecode — .NET, Java, Android (near-perfect decompilation)

Bytecode languages decompile back to almost-original source because they keep metadata
(names, types, structure). If `die`/`file`/`rabin2 -I` says .NET / Mono / Java / DEX, you are
in luck — this is the EASIEST RE: you get readable source, not pseudo-C.

## .NET (C# / VB) — CIL assemblies
Tell-tale: `file` says "PE32 ... .NET assembly", or imports `mscoree.dll`, or `rabin2 -I` lang=cil.
```sh
# ilspycmd — official ILSpy CLI, dumps full C# source tree
dotnet tool install -g ilspycmd
ilspycmd target.dll -o out_src -p        # -p = per-type project structure; reads exe/dll
# alternatives:
monodis target.exe                       # Mono disassembler → CIL
# dnSpyEx (GUI, Windows) = best for editing/debugging .NET; ilspycmd for headless dump
```
Obfuscated .NET (de4dot for the common obfuscators — ConfuserEx, SmartAssembly, etc.):
```sh
de4dot target.dll -o clean.dll           # auto-detects & undoes many .NET obfuscators, restores names
ilspycmd clean.dll -o out_src            # then decompile the cleaned assembly
```

## Java — .class / .jar
```sh
# jadx — best all-rounder (also does APK), produces clean Java + Gradle project
jadx target.jar -d out_src               # decompile jar → java source tree
jadx --show-bad-code target.jar -d out   # keep partially-failed methods too
# CFR / Procyon — excellent decompilers, good on tricky generics/lambdas:
cfr target.jar --outputdir out_cfr
procyon -jar target.jar -o out_proc
unzip -o target.jar -d raw && javap -c -p raw/com/x/Foo.class   # raw bytecode of one class
```

## Android APK (DEX + resources + native libs)
```sh
# resources + smali (for editing/repacking) :
apktool d app.apk -o out_apk             # decodes AndroidManifest.xml, resources, smali
# DEX → Java source (best readability) :
jadx -d out_src app.apk                  # gives Java + decoded resources in one shot
# manifest, permissions, entry points first:
aapt dump badging app.apk | grep -E 'package|launchable|uses-permission'
# native libs inside (lib/*/*.so) → treat each with native-binaries workflow
unzip -l app.apk | grep '\.so$'
```
Read order: `AndroidManifest.xml` (entry activities/services, exported components, permissions) →
the launcher Activity → networking/crypto classes. String resources: `out_apk/res/values/strings.xml`.

## What to look for once decompiled
- Networking: `HttpURLConnection`/`OkHttp`/`fetch`/`HttpClient` calls → endpoints, headers, signing.
- Crypto/signing: `Mac`/`Cipher`/`MessageDigest`/`HMAC`/`AES`/`RSA` usage → algorithm + where the key/secret comes from.
- Config: hardcoded URLs, keys, feature flags in constants/resources.
- Entry points: `Main`/`main`/launcher Activity/`static` initializers.
Search the decompiled tree fast: `grep -rInE 'sign|hmac|secret|api|http|aes|token' out_src | head -40`.
