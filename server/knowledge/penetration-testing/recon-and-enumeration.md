# Recon & enumeration (authorized targets only)

Find every host, port, service, version, and content path. Thorough enumeration is where
engagements are won. Stay in scope; mind rate limits in bug-bounty programs.

## Passive recon / OSINT (no packets to the target)
```sh
whois example.com ; dig +short example.com ANY ; dig +short txt example.com   # registration, DNS, SPF/DMARC
subfinder -d example.com -all -silent | tee subs.txt        # passive subdomain enum
amass enum -passive -d example.com                          # broader passive sources
theHarvester -d example.com -b all                          # emails, hosts, names from public sources
# tech fingerprint, public exposure:
whatweb https://example.com ; curl -sI https://example.com  # server/framework/headers
# certificate transparency (more subdomains): crt.sh, or:
curl -s 'https://crt.sh/?q=%25.example.com&output=json' | jq -r '.[].name_value' | sort -u
```

## Host discovery & port scanning (active — in scope)
```sh
# fast port discovery then targeted service scan (the standard two-step):
rustscan -a 10.10.10.10 --ulimit 5000 -- -sV -sC      # fast all-ports → nmap service/script
nmap -p- --min-rate 2000 -T4 10.10.10.10 -oA allports # all 65535 ports, grep the open ones
nmap -p 22,80,443 -sV -sC -A 10.10.10.10 -oA detailed # version + default scripts + OS on open ports
nmap -sU --top-ports 50 10.10.10.10                   # top UDP (SNMP/DNS/TFTP often here)
masscan -p1-65535 10.10.10.0/24 --rate 10000          # huge ranges fast (then nmap the hits)
```
Read the output: every open port = an enumeration task. Note versions for `searchsploit`.

## Web content & vhost enumeration
```sh
# directories/files:
ffuf -u https://t/FUZZ -w /usr/share/seclists/Discovery/Web-Content/raft-medium-words.txt -mc 200,204,301,302,401,403
feroxbuster -u https://t -w /usr/share/seclists/.../directory-list-2.3-medium.txt -x php,html,txt -r
gobuster dir -u https://t -w wordlist -x php,bak,old,zip
# virtual hosts (different sites on same IP):
ffuf -u https://t -H "Host: FUZZ.example.com" -w subdomains.txt -fs <baseline-size>
nikto -h https://t                                   # quick known-issue/misconfig scan
# CMS:
wpscan --url https://t --enumerate u,vp,vt           # WordPress users/plugins/themes
```

## Service-specific enumeration (per open port)
```sh
# SMB (139/445):
netexec smb 10.10.10.10 -u '' -p '' --shares          # null session shares
smbclient -L //10.10.10.10/ -N ; enum4linux-ng 10.10.10.10
# DNS (53): zone transfer
dig axfr @10.10.10.10 example.com
# SNMP (161): community strings → device/user/process info
onesixtyone 10.10.10.10 public ; snmpwalk -v2c -c public 10.10.10.10
# LDAP (389): ldapsearch -x -H ldap://10.10.10.10 -b "dc=example,dc=com"
# FTP (21): anonymous login? ftp 10.10.10.10  (user: anonymous)
# RPC (135/111): rpcclient -U "" -N 10.10.10.10  → enumdomusers
# NFS (2049): showmount -e 10.10.10.10
```

## Triage → vulnerability mapping
```sh
searchsploit "apache 2.4.49"          # known exploits for a version (Exploit-DB, offline)
searchsploit -m 50383                  # copy an exploit locally to read/use
nmap --script vuln 10.10.10.10         # NSE vuln scripts (noisy — use targeted)
```
For each service+version, ask: known CVE? default/weak creds? misconfig (anon access, dir listing,
exposed admin)? The answer routes you to web-exploitation or network-services-exploitation.
