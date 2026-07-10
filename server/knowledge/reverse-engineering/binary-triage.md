# Binary Triage — first look at any unknown file

Practical cheat-sheet for reverse engineering / security analysis of an unknown binary
(legitimate use: malware analysis, security audits, interoperability, your own/ authorized
targets, CTF). Each `##` section is self-contained, command-first. **Always identify the
format BEFORE reaching for a tool** — running the wrong tool wastes turns.

## Step 0 — identify what you're holding

Never guess from the extension; a `.exe` may be NSIS, Inno, PyInstaller, .NET, Electron, UPX-packed…
```sh
file ./target.bin                 # format: PE32 / ELF / Mach-O / installer / archive / data
xxd ./target.bin | head -40       # magic bytes (MZ=PE, 7F454C46=ELF, CAFEBABE=Java/Mach-O fat, PK=zip)
binwalk ./target.bin              # finds EMBEDDED files/archives/filesystems (installers, firmware)
binwalk -E ./target.bin           # entropy — flat high (~0.95+) = packed/encrypted/compressed
trid ./target.bin                 # probabilistic type ID when `file` is vague
```
Magic-byte quick map: `MZ`→Windows PE · `\x7fELF`→Linux · `\xCA\xFE\xBA\xBE`→Java class or Mach-O fat ·
`\xFE\xED\xFA`→Mach-O · `PK\x03\x04`→zip/jar/apk/asar/docx · `7z\xBC\xAF`→7z · `Rar!`→rar ·
`!<arch>`→.a/.deb · `dex\n`→Android DEX · `\x1f\x8b`→gzip.

## Step 1 — cheap wins: strings + metadata

`strings` alone often answers the question (URLs, keys, paths, error messages, version, library names).
```sh
strings -n 6 ./target.bin | less              # ASCII, min length 6
strings -e l -n 6 ./target.bin                # UTF-16LE (Windows binaries hide strings here!)
strings ./target.bin | grep -iE 'http|api|key|token|password|secret|\.dll|\.so|/[a-z]+/'
nm -D ./target.so 2>/dev/null                 # dynamic symbols (ELF)
objdump -p ./target.exe | grep -i dll         # PE imports (what APIs it calls → behavior hints)
rabin2 -izzq ./target.bin                     # radare's string dump (all sections)
rabin2 -I ./target.bin                        # arch/bits/endian/lang/compiler/canary/nx/pic/stripped
```
**Read the imports** — `WININET`/`ws2_32`=network, `crypt32`/`bcrypt`=crypto, `CreateRemoteThread`=injection,
`RegSetValue`=persistence. Imports tell you behavior before you disassemble a single instruction.

## Step 2 — is it packed / obfuscated?

Signs: very few imports, high entropy section, weird section names (`UPX0`, `.themida`), tiny code +
huge data. Detect, then unpack (see deobfuscation topic).
```sh
die ./target.bin            # Detect-It-Easy: packer/compiler/linker/protector signatures (best single tool)
upx -t ./target.bin && upx -d -o out.bin ./target.bin   # UPX → just decompress
rabin2 -I ./target.bin | grep -i lang        # detects .NET, Go, Rust, PyInstaller, etc.
```

## Step 3 — route to the right workflow

| What `file`/`die`/`rabin2` says | Go to topic / tool |
|---|---|
| NSIS / Inno Setup / MSI / self-extracting / "installer" | **installers-archives** (7z, innoextract) |
| PE32/PE64, ELF, Mach-O (native) | **native-binaries** (radare2 / Ghidra / objdump) |
| ".NET assembly" / "Mono" / CIL | **managed-bytecode** (ilspycmd / dnSpy) |
| Java class / jar | **managed-bytecode** (jadx / cfr / procyon) |
| Android APK / DEX | **managed-bytecode** (apktool + jadx) |
| PyInstaller / py2exe / "Python" | **python-js-unpack** (pyinstxtractor + decompyle3) |
| Electron / asar / "Node" | **python-js-unpack** (asar extract + JS deobf) |
| UPX/Themida/VMProtect/high entropy | **deobfuscation** (unpack first, then re-triage) |

## Safety when the binary may be malware

- Analyze **statically first** (file/strings/disasm) — no execution needed for most answers.
- If you must run it: **isolated VM / container, no host mounts, network off or to a sink**. Never on the user's host.
- Snapshot before, diff after (filesystem + registry + network). Tools: `procmon`, `inetsim`/`fakedns`, `frida`.
- Treat extracted scripts/resources as hostile too (don't `eval`/execute them to "see what they do").
