# Deobfuscation, unpacking & anti-analysis recognition

When the binary fights back: packers, obfuscators, anti-debug. Unpack/normalize FIRST, then
re-run triage on the clean output. Legitimate analysis contexts only.

## Detect the protector
```sh
die ./target            # Detect-It-Easy — names the packer/protector/compiler (UPX, Themida, VMProtect, ASPack, .NET Reactor…)
rabin2 -I ./target | grep -iE 'lang|static|stripped'
binwalk -E ./target     # entropy plot: a flat high plateau = packed/encrypted region
```

## Static unpackers (try these before manual unpacking)
```sh
upx -d -o clean.bin ./target                 # UPX (most common) — trivial, just decompress
# .NET:
de4dot ./target.dll -o clean.dll             # ConfuserEx, SmartAssembly, Eazfuscator, .NET Reactor (many)
# JS:
npx webcrack in.js -o out/                   # webpack unpack + deob
npx synchrony deobfuscate in.js              # obfuscator.io string-array/control-flow undo
# Android/Java string-deob: jadx already simplifies; for heavy cases use simplify (smali) virtualization deobfuscator
```

## Manual unpack (packer with no static tool) — concept
Packed binaries decompress themselves in memory then jump to the real entry (OEP). Approach:
1. Run under a debugger in an **isolated VM**; set a breakpoint after the unpacking stub (often a tail
   jump to a different section, or on `VirtualProtect`/`VirtualAlloc` of the unpacked region).
2. When stopped at the OEP, **dump the process memory** (Scylla / `r2 frida` / `pe-sieve`) and rebuild the
   import table (Scylla/ImpREC). Result = an unpacked PE you can decompile normally.
```sh
pe-sieve /pid <PID>          # scans a running process, dumps unpacked/injected PE + reconstructs imports
frida -p <PID> -l dump.js    # script: Memory.scan / Module.dump after OEP
```

## String/resource decryption (very common)
Strings are often XOR/AES-decrypted at runtime so `strings` shows nothing useful.
- Find the decrypt routine: look for a small function called from MANY places right before string use
  (`axt` xrefs in r2), or a loop doing `xor`/`add`/`sub` over a buffer.
- Recover statically: read the key + algorithm, reimplement the transform in Python over the encrypted blob.
- Or dynamically: `frida-trace` the decrypt function and log its **return value** — you get plaintext for free.
```python
# typical single-byte XOR recovery
enc = open('blob','rb').read(); key = 0x5A
print(bytes(b ^ key for b in enc))
```

## Anti-debug / anti-VM (recognize, then neutralize in your analysis env)
Common checks the binary may run — knowing them explains "why does it exit immediately":
- `IsDebuggerPresent`, `CheckRemoteDebuggerPresent`, PEB `BeingDebugged`, `NtQueryInformationProcess`.
- Timing: `rdtsc`/`GetTickCount` deltas (a debugger makes them huge).
- VM artifacts: MAC OUI, `vmtoolsd`, registry keys, CPUID hypervisor bit, low core/RAM.
For analysis, patch the check (force the "no debugger" branch) or use anti-anti-debug plugins
(ScyllaHide, `r2`'s `dbg.` options, frida hooks returning 0). Do this only on targets you're authorized to analyze.

## Go / Rust / Swift "stripped" recovery
Not really obfuscated — metadata survives:
```sh
GoReSym ./target                 # Go: recover function names, types, build info from pclntab
nm ./target | rustfilt           # Rust: demangle symbol names
objdump -d ./target | c++filt    # C++: demangle
```

## Workflow summary
`die`/`binwalk -E` → identify protector → static unpacker (upx -d / de4dot / webcrack) → if none, dynamic
dump (pe-sieve/frida) → re-triage the clean output → decompile (native-binaries / managed-bytecode /
python-js-unpack). Re-run `strings`/`rabin2 -I` on the unpacked result — it usually reveals the real
language, imports, and strings the packer was hiding.
