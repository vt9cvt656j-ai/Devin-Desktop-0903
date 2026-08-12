# Mr. Day One — User Agreement

**Draft for legal review. Not yet in force.**
Version 0.1 · Last updated [DATE] · Effective [DATE]

> **To the operator:** every `[BRACKETED]` item is a fact only you can supply — legal entity name,
> registered address, jurisdiction, refund window. Sections 8 and 9 are the clauses you asked for
> and are the least standard part of this document; read them closely. This draft has not been
> reviewed by a lawyer, and it should be before you publish it.

---

## 1. Who we are, and what this covers

Mr. Day One (formerly presented as "Michael IDE") is desktop software for writing and running code with
the assistance of AI models. It is published by [LEGAL ENTITY NAME], of [REGISTERED ADDRESS]
("we", "us").

This Agreement governs your use of:

- the **Mr. Day One desktop application**, on every platform we distribute it for;
- the **Mr. Day One service** — our hosted gateway at `api.michaelide.xyz` and `code.mrday.one`,
  which routes your requests to AI model providers, meters usage, and holds your account;
- the **website and account console**; and
- any documentation, updates or support we provide for the above.

Together, the "Service". Our [Privacy Policy](./PRIVACY_POLICY.md) forms part of this Agreement.

If you are using the Service on behalf of an organisation, you confirm you are authorised to bind
that organisation, and "you" means that organisation.

## 2. Accepting these terms

You accept this Agreement by creating an account, or by using the Service. If you do not accept it,
do not use the Service.

You must be at least [18 / the age of majority where you live] to hold an account. The Service is
not directed at children.

## 3. Your account

Keep your credentials secret; you are responsible for what happens under your account. Give us
accurate registration details and keep them current. Tell us promptly at [SECURITY CONTACT] if you
believe your account has been compromised.

One account is for one user. Do not share, resell or transfer your account without our written
agreement.

## 4. What the Service is — and what it is not

Mr. Day One is a tool that **acts on your computer at your direction**. Depending on the permissions
you grant, it can read and write files, run terminal commands, install packages, drive a browser,
query databases you have configured, capture network traffic, control desktop applications, and call
external services and Model Context Protocol (MCP) servers you connect.

Three consequences follow, and you should read them as the operative terms they are:

**(a) You are responsible for what you tell it to do.** Actions the software takes at your
instruction are your actions. This includes actions taken by an autonomous agent loop that you
started, including ones you did not individually foresee. Run it against systems and data you are
authorised to touch, and no others.

**(b) AI output is unreliable and must be checked.** Models produce confident, plausible text that
is sometimes wrong. Generated code may be insecure, may infringe third-party rights, may not do what
it appears to do, and may destroy data. Review it before you rely on it, and before you run it
anywhere that matters. Do not use the Service where an undetected error would put someone's safety,
health, legal position or livelihood at risk without a qualified human checking first — see §9.

**(c) Permission prompts are a safety feature, not an obstacle.** Where the software asks you to
approve a command, a file write, a network destination or a tool, that approval is the point at
which you take responsibility. Configuring blanket approvals, disabling sandboxing, or running in a
permissive mode is your decision and shifts that responsibility to you entirely.

We do not warrant that the Service will produce correct, complete, secure, or non-infringing output.

## 5. Licence, and what you may not do with the software

We grant you a personal, non-exclusive, non-transferable, revocable licence to install and use
Mr. Day One for as long as this Agreement is in force and your account is in good standing.

You may not:

- copy, distribute, sublicense, rent or sell the software, except as this Agreement allows;
- reverse engineer, decompile or disassemble it, except to the extent that restriction is
  unenforceable where you live;
- remove or obscure any notice of ownership, licence, or version;
- circumvent metering, quotas, licence checks, or region restrictions;
- use the Service to build, train, fine-tune, distil, or benchmark a competing AI model or a
  competing developer tool;
- extract our prompts, tool definitions, or routing logic for use outside the Service, or scrape the
  Service at scale by any automated means; or
- share your seat with others, including by proxying the Service to people without accounts.

Open-source components ship with their own licences. Where those conflict with this section, their
licences win for those components.

## 6. Your code, your prompts, and what comes back

**Your material stays yours.** You keep all rights in the code, files, prompts, and other material
you provide ("Inputs"). We claim no ownership.

**Output is yours too.** As between you and us, we assign you whatever rights we hold in what the
Service returns for you ("Outputs"). This assignment depends on your compliance with this Agreement.

Two honest caveats you should understand rather than discover later:

- **Output is not guaranteed unique.** Models can return materially similar text to different users
  from similar prompts. We make no claim that Output is original or that it does not resemble
  material owned by someone else.
- **Output may reproduce third-party material.** You are responsible for checking that what you ship
  respects other people's copyright, licences and patents.

You warrant that you hold the rights necessary to submit your Inputs to us and to the model
providers we route them to, and that doing so does not breach any obligation you owe someone else —
including your employer, your clients, and any non-disclosure agreement or open-source licence.

**We do not train models on your code.** See the Privacy Policy for the full statement and for what
model providers do under their own terms.

## 7. Model providers and other third parties

The Service routes your requests to third-party AI model providers, and can connect to third-party
services you configure — MCP servers, package registries, code hosts, databases, browsers, and
search and documentation providers.

- Your use of those services is also governed by **their** terms and policies, and you must comply
  with them. Where a model provider's usage policy is stricter than this Agreement, the stricter one
  applies to that provider's models.
- We are not responsible for third-party services: their availability, their accuracy, their
  security, or what they do with data you send them.
- Connecting a third-party service is your decision, and the access you grant it is your risk.

## 8. Acceptable use

You may not use the Service to do, plan, or help anyone else do, any of the following.

### 8.1 Unlawful and harmful acts

- Anything illegal where you are or where the effect lands.
- Building, acquiring or improving weapons, explosives, or chemical, biological, radiological or
  nuclear materials.
- Attacking or degrading critical infrastructure — power, water, medical devices, transport,
  telecommunications, financial market infrastructure.
- Creating malware, ransomware, botnets or denial-of-service tooling; breaking into systems you are
  not authorised to access; or evading security controls. Authorised, documented security testing of
  systems you own or have written permission to test is permitted and expected.
- Infringing intellectual property, misappropriating trade secrets, or breaching confidentiality
  obligations.
- Producing child sexual abuse material, or anything that sexualises a minor. There is no exception
  to this and we report it.
- Harassment, stalking, doxxing, incitement to violence, or promotion of hatred against people on
  the basis of a protected characteristic.
- Fraud, phishing, forged documents, fake reviews, spam, or deceptive commercial practices.

### 8.2 Uses directed against people

These are the clauses that matter most to us, and we will enforce them.

**(a) Autonomous force.** You may not use the Service to design, build, train, or operate a system
that selects or engages a target — a person, a vehicle, a structure — without a human being making
the decision to do so. This covers autonomous weapons and any system whose output causes physical
harm without a person in the decision.

**(b) Mass surveillance and social control.** You may not use the Service to build systems for
untargeted surveillance of populations, social credit or trustworthiness scoring, biometric
categorisation of people by race, ethnicity, religion, sexual orientation, political opinion or
union membership, emotion inference used to grade or discipline people, predictive policing directed
at individuals, or tracking a person's location or communications without their consent or a lawful
warrant.

**(c) Deception about who — or what — someone is dealing with.** You may not use the Service to
impersonate a real person or organisation; to generate a real person's voice, face or likeness
without their consent; or to run a system that presents itself as human to people who have not been
told they are interacting with AI. If your product talks to people using output from this Service,
tell them so.

**(d) Manipulation.** You may not use the Service to build systems designed to exploit a person's
vulnerabilities, addictions, cognitive limitations, age, or distress in order to move them to act
against their own interests. This includes engineered compulsion loops, deceptive interface design
intended to extract consent or money, and targeting people identified as susceptible.

**(e) Denial of essential goods.** You may not use the Service to build systems that deny or
withdraw housing, credit, insurance, healthcare, education, employment or public benefits on grounds
that are unlawful, undisclosed, or not open to human review — see §9.

### 8.3 Abuse of the Service

Circumventing quotas or billing; automated bulk access outside a normal editing workflow; probing or
attacking our infrastructure; using the Service after we have terminated your access; or acting to
degrade the Service for other users.

## 9. Human accountability

This section exists because a tool that writes and runs software can be used to build systems that
put no person in the way of a harmful outcome. That is the failure mode we are trying to prevent —
not automation itself.

**9.1 Someone must be accountable.** Where you use the Service to build or operate a system that
makes or materially shapes decisions about identifiable people, a named human being must be
accountable for those decisions. "The model decided" is not an answer this Agreement accepts, and
you may not design a system so that it becomes one.

**9.2 Review before harm, in high-stakes settings.** Where output of a system you build using the
Service affects a person's employment, housing, credit, insurance, healthcare, education, legal
position, immigration status, liberty, or access to public services, a qualified human must be able
to review, and be able to overturn, an adverse outcome before it takes effect. That reviewer must
have the authority, the information, and the time to actually do it — a rubber stamp does not
satisfy this section.

**9.3 Tell people.** Where a system you build using the Service produces advice, decisions or
communications delivered to people, disclose that AI was involved, at least at the start of each
interaction.

**9.4 Displacement.** You may not use the Service principally to eliminate a workforce while
concealing that fact from the people affected, from their representatives, or from a regulator to
whom you owe notice, nor to evade obligations you owe to workers — notice periods, consultation,
redundancy terms, collective bargaining, or health and safety duties — by routing their work through
automation.

To be plain about the boundary, because it would otherwise swallow ordinary use: this section does
**not** prohibit automating work. Writing code that does a job a person used to do by hand is the
normal, intended, permitted use of this software, and always has been. What §9.4 prohibits is doing
it deceptively, or to escape a duty you owe to the people affected.

**9.5 Attention, not abdication.** You remain responsible for reviewing what the Service produces
before it reaches anyone else. Deploying unreviewed output into a context where it acts on people is
a breach of this Agreement, whatever the outcome happens to be.

## 10. Plans, credits and payment

**Plans and quotas.** The Service is offered on plans — currently trial, basic, pro, power and ultra
— each carrying an overall allowance and rolling caps over shorter windows. Current allowances and
prices are on the [PRICING PAGE]. Usage is metered by the compute your requests consume, not by
message count; a single instruction that runs a long agent loop consumes more than a short question.

**Payment.** Fees are payable in advance, in [CURRENCY], and are non-refundable except where §10.3
or your local law says otherwise. You authorise us to charge your payment method for the plan you
choose, including on renewal.

**Refunds.** [STATE YOUR POLICY. If you sell to consumers in the EU, UK, Brazil, South Korea,
Taiwan, or mainland China, a statutory cooling-off period may apply regardless of what you write
here — confirm with counsel.]

**Changes to pricing.** We will give at least [30] days' notice before a price increase takes effect
for an existing subscriber. You may cancel before it does.

**Unused allowance** does not carry over between periods unless the plan says it does.

**Taxes** are your responsibility, other than taxes on our income.

## 11. Availability, updates and change

We may change, suspend or discontinue any part of the Service. Where a change is material and
adverse to a paying subscriber, we will give reasonable notice and, where we cannot continue to
provide what you paid for, a pro-rata refund of the unused portion.

The desktop application checks for updates and can install them. Updates are cryptographically
signed. You may disable automatic updates, but we do not support versions we have superseded, and we
may require a minimum version to use the hosted Service.

We do not promise uninterrupted availability. Maintenance, provider outages and network faults
happen.

## 12. Security, secrets and your data

You are responsible for the credentials, API keys and secrets present in the environments you point
the Service at. The software will send file contents, command output and other context to our
gateway and onward to model providers in order to answer you; assume anything you put in front of it
may be transmitted.

Do not use the Service to process material you are not permitted to disclose to a third-party
processor — including classified material, and personal data you have no lawful basis to transfer.

## 13. Feedback

If you send us feedback, bug reports or suggestions, you grant us a perpetual, irrevocable,
worldwide, royalty-free licence to use them without obligation to you. We will not identify you as
the source without your permission.

## 14. Suspension and termination

**By you:** cancel at any time from the account console. Cancellation takes effect at the end of the
current billing period.

**By us:** we may suspend or terminate your access if you breach this Agreement, if we are required
to by law, if your payment fails, or if your use presents a security or legal risk to us or to other
users. Where circumstances allow, we will tell you first and give you a chance to put it right.
Breach of §8.2 or §9 may result in immediate termination without notice.

**On termination:** your licence ends and you must stop using the software. We will delete or
de-identify your data as described in the Privacy Policy. Sections 5, 6, 8, 9, 13, 15, 16, 17 and 19
survive.

## 15. Disclaimers

To the fullest extent the law allows, the Service is provided **"as is"** and **"as available"**,
without warranty of any kind — express, implied or statutory — including merchantability, fitness for
a particular purpose, title, non-infringement, accuracy, or uninterrupted or error-free operation.

Some jurisdictions do not allow the exclusion of implied warranties. Where that is so, the exclusions
above apply only so far as that law permits, and nothing here limits rights you have as a consumer
that cannot be waived.

## 16. Limitation of liability

To the fullest extent the law allows:

- Neither party is liable for indirect, incidental, special, consequential or punitive damages, or
  for lost profits, lost revenue, lost data, or business interruption, however caused.
- Our total aggregate liability arising out of or relating to the Service is limited to the greater
  of the amount you paid us in the **[6]** months before the event giving rise to the claim, or
  **[USD 100]**.

Nothing in this Agreement excludes liability for death or personal injury caused by negligence, for
fraud or fraudulent misrepresentation, or for anything else that cannot be excluded by law.

## 17. Indemnity

You will indemnify and hold us harmless against claims, damages, losses and reasonable legal costs
arising from: your breach of this Agreement; your Inputs; what you do with Outputs; actions the
software takes at your direction; and your infringement of anyone's rights. We will notify you of any
such claim and let you control its defence, provided any settlement releases us fully and imposes no
obligation on us.

## 18. Governing law and disputes

This Agreement is governed by the laws of [JURISDICTION], without regard to conflict-of-laws rules.
The courts of [VENUE] have exclusive jurisdiction, except that either party may seek injunctive
relief anywhere to protect its intellectual property or to stop a breach of §8.

Nothing here deprives a consumer of the protection of mandatory rules of the country where they
live.

> **Operator note:** if you sell into the EU, UK or China, a blanket choice of a foreign forum is
> often unenforceable against consumers, and China's PIPL and consumer-protection rules will apply
> to mainland users regardless. Get this section reviewed against your actual customer base.

## 19. General

**Whole agreement.** This Agreement and the Privacy Policy are the entire agreement between us on
this subject.

**Changes.** We may amend this Agreement. For material changes we will give at least [30] days'
notice by email or in the application, and the change takes effect for you when the notice period
ends. If you do not accept it, stop using the Service and cancel; we will refund the unused portion
of any prepaid period.

**Severability.** If a provision is unenforceable, the rest stands and the provision is narrowed to
the minimum extent needed to make it enforceable.

**No waiver.** Not enforcing a term once does not waive it.

**Assignment.** You may not assign this Agreement without our consent. We may assign it to an
affiliate or in connection with a merger or sale of assets.

**Export and sanctions.** You may not use the Service in breach of export controls or sanctions, and
you confirm you are not a restricted party.

**Contact.** [SUPPORT EMAIL] · [LEGAL EMAIL] · [POSTAL ADDRESS]
