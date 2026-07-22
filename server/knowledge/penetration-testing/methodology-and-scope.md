# Penetration testing — methodology, scope & authorization

For AUTHORIZED security testing only: signed engagements, bug-bounty targets within scope,
CTF, or systems you own/control. **Authorization and scope come first** — testing a system
you're not authorized to test is illegal regardless of intent. This cheat-sheet is the
standard professional methodology (PTES / OSCP / OWASP). Each `##` is self-contained.

## Step 0 — authorization & scope (never skip)
- Confirm a **written authorization** / rules-of-engagement (RoE) exists and you're in it:
  in-scope hosts/domains/IP ranges, allowed test windows, allowed techniques, explicit
  out-of-scope (prod DBs, third-party, social-engineering, DoS usually EXCLUDED).
- Bug bounty: read the program scope + rules. Out-of-scope assets, prohibited actions
  (no automated high-rate scanning, no data exfil beyond PoC, no pivoting) are binding.
- CTF / your own lab: scope is the box/network you were given.
- **Do not**: test out-of-scope assets, run DoS/stress, exfiltrate real user data, pivot
  outside scope, or leave persistence/backdoors behind. Minimize impact; prefer read-only PoC.

## The phases (PTES)
1. **Recon / OSINT** — passive info gathering (no packets to target): domains, employees, tech, leaks.
2. **Scanning / Enumeration** — active: open ports, services, versions, web content, users, shares.
3. **Vulnerability analysis** — map findings to known CVEs / misconfigs / logic flaws.
4. **Exploitation** — gain a foothold (web bug, service exploit, weak creds). Get the minimum needed to prove impact.
5. **Post-exploitation** — privesc, situational awareness, (in-scope) lateral movement, evidence collection.
6. **Reporting** — findings with severity (CVSS), reproduction steps, evidence, and **remediation**. This is the deliverable.

## Working principles
- **Enumerate, enumerate, enumerate** — most boxes fall to thorough enumeration, not 0-days. "If you can't get in, you haven't enumerated enough."
- **Take notes continuously** — every host/port/cred/finding with timestamps and commands. Tools: CherryTree, Obsidian, a markdown log, or `tmux` + `script`.
- **Prove impact, minimize damage** — a screenshot of `id`/`whoami`/one record is enough PoC; don't dump the whole DB.
- **Stay in scope, log everything** — so the client can attribute your traffic and you can prove you stayed in bounds.
- **Clean up** — remove uploaded tools, test accounts, and any artifacts after the engagement (note them in the report).

## Standard toolkit (Kali/ParrotOS bundle most of these)
Recon: `amass`, `subfinder`, `theHarvester`, `whois`, `dnsrecon`. Scan: `nmap`, `masscan`, `rustscan`.
Web: Burp Suite, `ffuf`/`gobuster`/`feroxbuster`, `nikto`, `sqlmap`, `wpscan`. Exploit: Metasploit,
`searchsploit` (Exploit-DB). Creds: `hydra`, `netexec`(crackmapexec), `hashcat`, `john`. AD: `bloodhound`,
`impacket` suite, `kerbrute`. Post: `linpeas`/`winpeas`, `pspy`, `chisel`/`ligolo` (pivot).
Install ad-hoc: `apt install <tool>`, `pipx install <tool>`, or clone from the tool's GitHub.
