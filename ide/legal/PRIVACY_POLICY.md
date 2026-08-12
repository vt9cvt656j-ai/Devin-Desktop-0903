# Mr. Day One — Privacy Policy

**Draft for legal review. Not yet in force.**
Version 0.1 · Last updated [DATE] · Effective [DATE]

> **To the operator:** `[BRACKETED]` items are facts only you can confirm — entity, address,
> retention windows, which model providers you actually route to, whether you have a DPO. Several
> statements below assert what your system does; I have written them to match the software as built,
> but you must verify each one before publishing, because a privacy policy that misdescribes your
> practices is itself a violation in most of the regimes you operate under.

---

## 1. Who this covers, and who is responsible

This policy explains what Mr. Day One (formerly presented as "Michael IDE") does with personal
information. The controller is [LEGAL ENTITY NAME], [REGISTERED ADDRESS]. Contact us at
[PRIVACY EMAIL].

It covers the desktop application, our hosted gateway (`api.michaelide.xyz`, `code.mrday.one`), the
website, and the account console.

## 2. The short version

- Your **code and prompts** are sent to our gateway and on to an AI model provider so we can answer
  you. That is the product working.
- **We do not train AI models on your code, prompts, or files**, and we do not sell your data.
- Most of what the application knows **stays on your computer** — conversation history, project
  memory, settings, indexes.
- We keep the minimum on our servers to run accounts, billing and quotas, and to stop abuse.
- Some tools in this product are **powerful and intrusive by design** — they can read your terminal,
  capture network traffic, query your databases and see your screen. Section 5 explains what that
  means, because you should know it before you turn them on rather than after.

## 3. What we collect

### 3.1 You give us

| | |
|---|---|
| **Account** | Email address, password (stored only as a bcrypt hash — we never see the password), display name, and any organisation you belong to. |
| **Billing** | Plan, purchase history, and the tokens our payment processor returns. **We do not receive or store your full card number.** |
| **Prompts and code** | Your instructions, and the file contents, selections, diffs, terminal output and other context the application attaches to answer them. |
| **Support and feedback** | What you send us, including any transcript you choose to attach. |

### 3.2 Generated when you use the Service

| | |
|---|---|
| **Usage metering** | Timestamps, model used, token counts in and out, cost, plan, and the quota state we compute from them. This is how billing works. |
| **Routing metadata** | The mode a request was made in, the names of the tools enabled for it, and a compact capability profile. These are decisions, not your text. |
| **Technical** | IP address, approximate region derived from it, application version, operating system, and error diagnostics. |
| **Security** | Sign-in events, failed attempts, and signals we use to detect account sharing, quota circumvention and abuse. |

### 3.3 What we do *not* collect

We do not collect your contacts, your browsing history outside the tools you explicitly invoke, or
your location beyond what an IP address approximates. We do not use third-party advertising
trackers, and we do not build advertising profiles.

## 4. What stays on your computer

The following is stored locally and is **not** sent to us as a matter of course:

- **Conversation history and session transcripts**, kept in plaintext under your user profile so
  sessions can be resumed. Delete them at any time from within the application or by deleting the
  files.
- **Project memory and global preferences** — the notes you write for the assistant to remember.
- **Local caches, indexes and settings**, including a local SQLite database.
- **Workspace files.** The application reads your project from disk; the files themselves are not
  uploaded anywhere except as excerpts included in a request you make.

Content leaves your machine when you send a request that includes it, when you invoke a tool that
reaches an external service, or when you explicitly send us a report.

## 5. The intrusive tools, stated plainly

Some capabilities collect far more than a normal editor would. They are off until you enable or
approve them, and they are worth understanding first:

- **Terminal execution** captures the output of the commands it runs. That output can contain
  secrets, tokens and customer data present in your environment.
- **Network capture** records real HTTP traffic — URLs, headers, cookies, request and response
  bodies. This will include authentication credentials and session tokens for whatever you point it
  at.
- **Browser automation** loads pages using a browser profile you select, and can therefore act with
  whatever sessions that profile holds.
- **Database access** runs queries against databases you configure and returns the rows. If those
  rows contain other people's personal data, that data enters the request.
- **Desktop automation** can read the screen, the clipboard, and window contents of applications you
  grant access to.
- **MCP servers and other connectors** send data to third parties you have chosen and configured. We
  do not control them.

Where these tools produce content that becomes part of a request, that content goes to our gateway
and to a model provider like anything else. **If you would not send it to a third-party processor,
do not put it in front of these tools.**

## 6. Why we use it, and on what legal basis

| Purpose | Data | Basis (GDPR) |
|---|---|---|
| Provide the Service — route requests, return answers | Prompts, code, routing metadata | Performance of a contract |
| Accounts and authentication | Account, security | Performance of a contract |
| Billing, quotas, fraud prevention | Billing, usage metering | Contract; legitimate interests |
| Keep the Service secure and stop abuse | Technical, security | Legitimate interests |
| Diagnose faults and improve reliability | Technical, aggregated usage | Legitimate interests |
| Answer support requests | What you send us | Contract; legitimate interests |
| Legal compliance | As required | Legal obligation |

Where we rely on legitimate interests, we have weighed them against your rights, and you may object
— see §11.

For users in **mainland China**, our basis under PIPL is your consent and the necessity of
processing to perform the contract you have with us; separate consent is obtained where PIPL
requires it, including for cross-border transfer (§10).

## 7. Model providers and other recipients

To answer your requests we send your prompt and its attached context to a **third-party AI model
provider** [LIST PROVIDERS — e.g. Anthropic]. This is unavoidable: it is how the Service functions.

- Providers process that data under their own terms and privacy policies, which you should read.
- We route under **commercial terms that prohibit training on customer content** [CONFIRM this is
  true of every provider you use — it is the single most important representation in this document].
- Where a provider offers zero-retention processing and we have enabled it, we say so at
  [STATUS PAGE / DOCS].

Other recipients:

- **Infrastructure and hosting** — our servers and managed database and cache services.
- **Payment processing** — [PROCESSOR].
- **Email delivery** — for account, billing and security messages.
- **Software distribution and updates** — the application checks a release endpoint for updates,
  which reveals your IP address and current version to that host.
- **Authorities**, where we are legally required, and where we are satisfied the request is valid.
- **A successor**, if we are acquired or merged, subject to this policy continuing to apply.

We do not sell personal information, and we do not share it for cross-context behavioural
advertising, as those terms are used in California law.

## 8. Training

**We do not use your code, prompts, files, or conversations to train AI models** — not ours, and we
do not permit our providers to under the terms we route through.

If we ever want to change that, it will be **opt-in**, offered separately and clearly, off by
default, and revocable. It will never be buried in an update to this policy.

Aggregate statistics that identify no one — error rates, token volumes, latency — are used to
operate and improve the Service.

## 9. Retention

| Data | Kept for |
|---|---|
| Account records | For the life of the account, then [30] days |
| Usage and billing records | [7 years] or as tax and accounting law requires |
| Prompts and code at the gateway | Not persisted beyond the request, except as noted below |
| Abuse and security logs | [90] days |
| Support correspondence | [24] months |
| Local session transcripts | On your machine until you delete them; [configurable] |

**On transient handling:** our gateway processes your prompt in memory to assemble and route the
request. Where we log for debugging, logs are [SPECIFY: redacted / retained N days / disabled in
production] — confirm and state this accurately, since it is what most readers will care about.

When you delete your account we delete or irreversibly de-identify your data within [30] days,
except where we must keep it for legal or accounting reasons.

## 10. International transfers

We are established in [COUNTRY] and use infrastructure in [REGIONS]. Model providers may process
data in [REGIONS]. Your data therefore crosses borders.

- For the **EEA/UK**: transfers rely on Standard Contractual Clauses (and the UK Addendum), with a
  transfer risk assessment on file. Ask us for a copy at [PRIVACY EMAIL].
- For **mainland China**: cross-border transfer is subject to PIPL Article 38. [STATE WHICH ROUTE
  YOU RELY ON — standard contract filed with the CAC, certification, or security assessment — and
  obtain the separate consent PIPL requires. This is not optional if you serve mainland users, and
  it is the compliance gap most likely to bite you.]

## 11. Your rights

Wherever you are, you can ask us to: **access** the data we hold about you; **correct** it;
**delete** it; **export** it in a portable form; **restrict** or **object to** processing; and
**withdraw consent** where we relied on it.

Write to [PRIVACY EMAIL]. We respond within [30] days, and we do not charge for a first request. We
may ask you to verify your identity — we will not use what you send for verification for anything
else.

- **EEA/UK:** you may complain to your supervisory authority.
- **California:** you have the rights above, plus the right not to be discriminated against for
  exercising them. We do not sell or share personal information.
- **Mainland China (PIPL):** you additionally have the right to an explanation of our processing
  rules, and to have your data transferred to another provider where technically feasible. You may
  designate someone to exercise these rights after your death.

We do not make decisions producing legal or similarly significant effects about you by automated
means alone.

## 12. Security

We use TLS in transit, encryption at rest on our infrastructure, hashed passwords (bcrypt),
scoped access tokens, and least-privilege access for staff. Update artefacts are cryptographically
signed so your application will not install one we did not publish.

No system is perfectly secure. If a breach affects your personal data, we will notify you and the
relevant regulator as the law requires and without undue delay.

**Report a vulnerability** to [SECURITY EMAIL]. We will not pursue legal action against good-faith
security research that respects user privacy and does not degrade the Service.

## 13. Children

The Service is not directed at children under [18/16/13 — set per market], and we do not knowingly
collect their personal data. If you believe a child has given us data, write to [PRIVACY EMAIL] and
we will delete it.

## 14. Cookies

The website and account console use cookies that are strictly necessary for sign-in and security,
and [STATE whether any analytics are used and how consent is obtained — required for the EEA/UK].
The desktop application does not use advertising cookies.

## 15. Changes

We will post any change here with a new effective date. For a material change we will give at least
[30] days' notice by email or in the application before it takes effect. Superseded versions stay
available at [ARCHIVE URL].

## 16. Contact

[LEGAL ENTITY NAME]
[POSTAL ADDRESS]
Privacy: [PRIVACY EMAIL] · Security: [SECURITY EMAIL]
[EU/UK representative, if you have one] · [DPO, if you are required to appoint one]
