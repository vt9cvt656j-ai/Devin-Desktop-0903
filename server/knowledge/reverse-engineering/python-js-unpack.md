# Python & JavaScript packaging — extract back to source

Apps shipped as a single binary or bundle but written in Python/JS keep their bytecode or
source inside. These RE to readable code very well.

## PyInstaller (the common "Python → .exe")
Tell-tale: strings contain `PyInstaller`, `pyi-`, `python3X.dll`, or a `MEIPASS` reference.
```sh
# 1) extract the embedded archive (PYZ + .pyc files + bundled libs)
python pyinstxtractor.py target.exe        # → target.exe_extracted/  (use pyinstxtractor-ng for new versions)
# 2) inside, the entry script is the .pyc with the same name as the exe (no extension), plus pyc files.
#    PyInstaller strips the pyc header — fix it before decompiling (copy a magic from any sibling .pyc).
# 3) decompile pyc → py:
pip install decompyle3 uncompyle6           # decompyle3 for py3.7-3.8; uncompyle6 older; pycdc for newest
decompyle3 target.exe_extracted/main.pyc > main.py
pycdc main.pyc                              # C++ decompiler, handles 3.9+ where python ones fail
pycdas main.pyc                             # disassemble pyc to bytecode if decompile fails
```
py2exe / cx_Freeze: similar — `library.zip` (for py2exe) holds the `.pyc`; unzip then decompile.

## Raw .pyc / __pycache__
```sh
pip install decompyle3 ; decompyle3 mod.cpython-38.pyc > mod.py
# if version mismatch: check magic with `python -c "import importlib.util,sys;print(...)"`, or just try pycdc.
```

## JavaScript: minified / bundled / obfuscated
The source IS there, just mangled. Restore readability:
```sh
# 1) pretty-print (always do this first):
npx prettier --write bundle.js      ||   js-beautify -r bundle.js
# 2) source maps — if a .map exists or //# sourceMappingURL= present, you get ORIGINAL source back:
npx source-map-explorer bundle.js bundle.js.map
npx shuji bundle.js.map -o out_src         # reconstruct original files from a sourcemap
# 3) webpack/rollup bundle → split into modules:
npx webcrack bundle.js -o out_modules      # unminifies, unpacks webpack, undoes common obfuscation
# 4) obfuscator.io / heavy obfuscation:
npx deobfuscator bundle.js   ||  use https://github.com/ben-sb/javascript-deobfuscator
#    these undo string-array rotation, control-flow flattening, dead-code, hex identifiers.
```
AST-level work (custom deob) — parse, transform, regenerate:
```js
// babel: const ast = parser.parse(src); traverse(ast, { /* simplify nodes */ }); generate(ast).code
```

## Electron (JS desktop app)
```sh
npx @electron/asar extract resources/app.asar out_src    # full JS tree, then prettier/webcrack as above
```

## Source-map / signing recon (common goal)
After deobfuscation, find the algorithm:
```sh
grep -rInE 'sign|sig|hmac|md5|sha|aes|encrypt|secret|token|salt|nonce|timestamp|navigator|webdriver' out_src | head -40
```
Then read those functions to recover the request-signing scheme (params order, hash, key source).
For dynamic confirmation, hook the function in a real browser (devtools / console) and log inputs→output.
