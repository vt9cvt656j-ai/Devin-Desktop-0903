# Unpacking Installers & Archives (Windows & cross-platform)

How to extract the real payload (files, scripts, configs) out of an installer or archive
WITHOUT running it. Most "reverse the .exe" jobs are actually "unpack the installer" — the
logic is in plain scripts/resources inside, not in machine code. Identify the installer type
first (`file`, `die`, `binwalk`, or strings), then use its dedicated extractor.

## NSIS (Nullsoft Scriptable Install System)  ← very common; `file` says "Nullsoft Installer"

The install logic is an NSIS script; resources are LZMA/bzip2/zlib-compressed inside.
```sh
7z l " installer.exe"                 # LIST contents (NSIS is a 7z-readable format)
7z x installer.exe -oout_dir          # EXTRACT all files + the $PLUGINSDIR
# the decompiled install script (the actual logic) lives as:  out_dir/[NSIS].nsi  (with 7z ≥ some builds)
```
If 7z doesn't surface the script, recover it with:
```sh
# nsis-decompiler / 7-Zip-NSIS fork, or:
pip install nsis-extractor  ||  use https://github.com/idle-git/nsisunbz
# extracted tree shows: $INSTDIR files, $PLUGINSDIR/*.dll plugins, and the install .nsi script
```
Read `*.nsi` for: download URLs, registry keys written, files dropped, exec'd commands, license/serial checks.

## Inno Setup  (`file` says "Inno Setup")
```sh
innoextract -l setup.exe              # list
innoextract setup.exe -d out_dir      # extract (handles all versions; best tool)
# the install script is compiled Pascal (CompiledCode.bin) — view with Inno Setup Unpacker / innounp:
innounp -x setup.exe                  # innounp extracts files + install_script.iss (the readable logic)
```

## MSI (Windows Installer database)
```sh
msiextract package.msi -C out_dir     # (msitools) extract files
7z x package.msi -oout_dir            # alt
msiinfo tables package.msi; msiinfo export package.msi CustomAction   # read the install tables/actions
lessmsi x package.msi out_dir\        # Windows GUI/CLI alternative
```

## Self-extracting / generic / compound
```sh
7z x file.exe -oout                   # 7z opens SFX-7z, SFX-RAR, CAB, ISO, WIM, many SFX EXEs
cabextract file.cab                   # standalone CAB
binwalk -e --dd='.*' file.bin         # carve EVERYTHING embedded (firmware, concatenated archives)
foremost -i file.bin -o carved        # alt file carver by signature
```

## Electron apps (cross-platform, JS inside)
The real app is JavaScript in an `asar` archive — not compiled.
```sh
find . -name app.asar                 # usually resources/app.asar
npx @electron/asar extract app.asar out_dir   # or: npx asar extract app.asar out_dir
# now you have the full JS source tree (main process + renderer). Then deobfuscate JS if minified.
```

## Java / Android packaging
```sh
unzip -o app.jar -d out               # jar/war/apk ARE zips
unzip -o app.apk -d out               # then jadx for dex→java (see managed-bytecode)
```

## After extraction — orient fast
```sh
find out_dir -type f | sed 's/.*\.//' | sort | uniq -c | sort -rn   # file-type histogram → where's the logic
grep -rIl --include='*.js' --include='*.nsi' --include='*.py' -e 'http' -e 'api' -e 'key' out_dir | head
```
Look for: scripts (`.nsi/.iss/.js/.py/.ps1/.bat`), configs (`.json/.ini/.xml`), and dropped binaries
(recurse: an installer often drops ANOTHER packed binary → re-triage it).
