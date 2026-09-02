# Native binaries — disassembly & decompilation (PE / ELF / Mach-O)

When the logic is real machine code (C/C++/Rust/Go/Swift). Goal: get from bytes → functions →
readable pseudo-C. Use radare2/rizin for fast CLI triage, Ghidra for the best free decompiler.
Legitimate use only (your own / authorized / malware analysis / CTF / interop).

## radare2 / rizin — scriptable CLI (fast, no GUI, great for agents)
```sh
r2 -A ./target            # open + auto-analyze (aaa). rizin: rz -A ./target
```
Inside (or pipe with `r2 -qc '<cmd>' ./target`):
```
aaa            # analyze all (functions, xrefs, strings)
afl            # list functions (addr, size, name)
afl~main       # grep functions containing "main"
iz             # strings in data section;  izz = all sections
ii             # imports;  iE = exports;  is = symbols
axt @ sym.foo  # who CALLS foo (xrefs to);  axf = calls from
s main; pdf    # seek to main, print disassembly of function
pdc @ sym.foo  # r2's pseudo-decompile (rough C-ish);  rizin has better
s 0x401000; pd 40   # disassemble 40 instrs at address
```
One-liners (non-interactive, ideal in run_cmd):
```sh
r2 -qc 'aaa; afl' ./target                       # function list
r2 -qc 'aaa; izz~http' ./target                  # strings containing http
r2 -qc 'aaa; s main; pdf' ./target               # disassemble main
r2 -qc 'aaa; axt @ sym.imp.strcmp' ./target      # everywhere strcmp is called (license/passwd checks!)
```

## Ghidra headless — best free decompiler, scriptable (no GUI needed)
```sh
$GHIDRA/support/analyzeHeadless /tmp proj -import ./target -postScript Decompile.java -deleteProject
# or use the community headless wrapper "pyghidra" / a postScript that dumps decompiled C for all funcs:
analyzeHeadless /tmp proj -import ./target -scriptPath ./scripts -postScript DumpDecomp.java
```
A DumpDecomp.java/python script iterates `getFunctionManager().getFunctions(true)` and prints
`DecompInterface.decompileFunction(f).getDecompiledFunction().getC()`. Output = readable pseudo-C.
This is the highest-signal step for understanding algorithms (e.g. a sign/license routine).

## objdump / gdb — universal, already installed
```sh
objdump -d -M intel ./target | less          # full disassembly (Intel syntax)
objdump -d ./target | awk '/<main>:/,/^$/'   # just main
objdump -T ./target.so                       # dynamic symbol table (ELF)
objdump -s -j .rodata ./target               # dump read-only data (constants/strings)
gdb -q ./target -ex 'info functions' -ex quit
```

## Go / Rust / Swift specifics (stripped but recoverable)
```sh
# Go: symbols survive in pclntab even when "stripped"
strings ./target | grep -E '\.go$' ; r2 -qc 'aaa; afl~go.' ./target
GoReSym ./target            # recovers Go function names/types/version
# Rust: names are mangled — demangle:
nm ./target | rustfilt ; objdump -d ./target | rustfilt
# C++ mangled names:
nm ./target | c++filt
```

## Dynamic tracing (when static is slow) — sandbox only
```sh
ltrace ./target            # library calls (Linux) — see strcmp/memcmp args for checks
strace -f ./target         # syscalls — files opened, network, exec
frida-trace -i 'strcmp' -f ./target.exe       # hook functions, log args/returns live (cross-platform)
```
Finding a check (license/password/signature): trace `strcmp`/`memcmp`/`*cmp*`/crypto calls,
read the arguments — the "expected" value is often passed in plaintext at compare time.
