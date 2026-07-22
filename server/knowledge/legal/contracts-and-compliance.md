# Legal: Contracts & Compliance Systems

## Contract Document Structure

### Clause Data Model
```sql
CREATE TABLE contracts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    contract_type TEXT NOT NULL,  -- 'NDA', 'MSA', 'SaaS', 'employment', 'lease'
    status TEXT NOT NULL DEFAULT 'draft',
    -- CHECK (status IN ('draft','review','pending_signature','executed','expired','terminated'))
    effective_date DATE,
    expiration_date DATE,
    auto_renew BOOLEAN DEFAULT false,
    renewal_term_days INT,
    governing_law TEXT,  -- 'DE', 'NY', 'CA', 'England_Wales'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE clauses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID REFERENCES contracts(id),
    clause_type TEXT NOT NULL,
    -- 'definitions','term','payment','ip','confidentiality','liability','termination','indemnity','force_majeure','dispute_resolution','governing_law'
    position INT NOT NULL,  -- ordering within contract
    title TEXT NOT NULL,
    body TEXT NOT NULL,     -- clause text with {{variable}} placeholders
    is_negotiable BOOLEAN DEFAULT true,
    risk_level TEXT DEFAULT 'low',  -- 'low','medium','high','critical'
    parent_clause_id UUID REFERENCES clauses(id)  -- for sub-clauses
);

CREATE TABLE defined_terms (
    id UUID PRIMARY KEY,
    contract_id UUID REFERENCES contracts(id),
    term TEXT NOT NULL,       -- e.g., "Confidential Information"
    definition TEXT NOT NULL,
    first_use_clause_id UUID REFERENCES clauses(id)
);
```

### Variable Substitution (Template Engine)
```javascript
function renderClause(template, variables) {
  return template.replace(/\{\{(\w+)(\|([^}]+))?\}\}/g, (match, key, _, fallback) => {
    const value = variables[key];
    if (value === undefined || value === null) {
      if (fallback) return fallback;
      throw new Error(`Missing required variable: ${key}`);
    }
    return value;
  });
}

// Contract variables — type-safe
const CONTRACT_VARS = {
  party_a_name: { type: 'string', required: true },
  party_b_name: { type: 'string', required: true },
  effective_date: { type: 'date', format: 'MMMM D, YYYY' },
  term_months: { type: 'number', min: 1 },
  payment_amount: { type: 'money', currency: 'USD' },
  governing_state: { type: 'enum', values: ['Delaware','New York','California'] },
};
```

### Cross-Reference System
```javascript
function resolveReferences(clauses) {
  const byNumber = new Map(clauses.map(c => [c.position, c]));
  return clauses.map(c => ({
    ...c,
    body: c.body.replace(/Section (\d+(\.\d+)*)/g, (match, ref) => {
      const target = byNumber.get(parseFloat(ref));
      if (!target) console.warn(`Dangling reference: ${match}`);
      return match; // preserve text, flag broken refs
    })
  }));
}
```

## GDPR Technical Implementation

### Lawful Basis Tracking
```sql
CREATE TABLE consent_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    purpose TEXT NOT NULL,         -- 'marketing_email', 'analytics', 'third_party_sharing'
    lawful_basis TEXT NOT NULL,    -- 'consent', 'contract', 'legal_obligation', 'legitimate_interest', 'vital_interest', 'public_task'
    granted_at TIMESTAMPTZ,
    withdrawn_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    evidence TEXT,                 -- how consent was collected (checkbox text, timestamp, IP)
    version TEXT NOT NULL          -- privacy policy version at time of consent
);

-- Index for fast lookup during data processing
CREATE INDEX idx_consent_active ON consent_records(user_id, purpose)
    WHERE withdrawn_at IS NULL AND (expires_at IS NULL OR expires_at > NOW());
```

### DSAR (Data Subject Access Request) Pipeline
```python
class DSARProcessor:
    """GDPR Art. 15-22: respond within 30 days (extendable to 90)"""

    DEADLINE_DAYS = 30

    def process_request(self, user_id, request_type):
        # request_type: 'access', 'rectification', 'erasure', 'portability', 'restriction', 'objection'

        if request_type == 'access':
            return self._collect_all_data(user_id)
        elif request_type == 'erasure':
            return self._right_to_erasure(user_id)
        elif request_type == 'portability':
            return self._export_machine_readable(user_id)

    def _right_to_erasure(self, user_id):
        """Art. 17: Right to erasure ('right to be forgotten')"""
        # Check exemptions FIRST
        exemptions = self._check_exemptions(user_id)
        if exemptions:
            return {'status': 'partially_denied', 'exemptions': exemptions}

        # Erasure must be complete across ALL systems
        systems = ['primary_db', 'analytics', 'backups', 'logs', 'third_parties']
        results = {}
        for system in systems:
            results[system] = self._erase_from(system, user_id)

        return {'status': 'completed', 'systems': results}

    def _check_exemptions(self, user_id):
        """Art. 17(3): Erasure does NOT apply if processing is necessary for:"""
        exemptions = []
        # (a) freedom of expression
        # (b) legal obligation (tax records: 7 years)
        if self._has_financial_records(user_id):
            exemptions.append('legal_obligation_tax_7yr')
        # (c) public health
        # (d) archiving/research/statistics
        # (e) legal claims
        if self._has_active_dispute(user_id):
            exemptions.append('legal_claims')
        return exemptions

    def _export_machine_readable(self, user_id):
        """Art. 20: Data portability — JSON or CSV, machine-readable"""
        data = self._collect_all_data(user_id)
        return {
            'format': 'application/json',
            'data': data,
            'schema_version': '1.0',
            'exported_at': datetime.utcnow().isoformat()
        }
```

### Data Retention Policy Engine
```javascript
const RETENTION_RULES = {
  'user_account':      { days: null, trigger: 'account_deletion' },
  'transaction_logs':  { days: 2555, trigger: 'creation' },  // 7 years (tax)
  'analytics_events':  { days: 730, trigger: 'creation' },   // 2 years
  'session_data':      { days: 30, trigger: 'creation' },
  'support_tickets':   { days: 1095, trigger: 'resolution' }, // 3 years
  'consent_records':   { days: 2555, trigger: 'withdrawal' }, // keep proof 7 years
  'audit_logs':        { days: 2555, trigger: 'creation' },
};

async function enforceRetention() {
  for (const [category, rule] of Object.entries(RETENTION_RULES)) {
    if (!rule.days) continue;
    const cutoff = new Date(Date.now() - rule.days * 86400000);
    const deleted = await db.query(
      `DELETE FROM ${category} WHERE ${rule.trigger}_at < $1 RETURNING id`,
      [cutoff]
    );
    auditLog(`retention_sweep: deleted ${deleted.rowCount} from ${category}`);
  }
}
```

## E-Signature (ESIGN Act / eIDAS Compliance)

### Signature Workflow
```
1. Document finalized → generate hash (SHA-256)
2. Send signing invitation with unique token (expires in 7 days)
3. Signer authenticates (email link + PIN / SMS OTP / ID verification)
4. Capture signing ceremony evidence:
   - Timestamp (RFC 3161 if available)
   - IP address, user agent
   - Authentication method used
   - Document hash at time of signing
   - Click/tap coordinates on signature field
5. Apply signature → seal document (prevent further edits)
6. Generate audit trail certificate
7. Distribute executed copies to all parties
```

### Audit Trail Requirements
```sql
CREATE TABLE signature_events (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL,
    signer_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    -- 'sent','viewed','signed','declined','voided','expired'
    ip_address INET,
    user_agent TEXT,
    geo_location TEXT,
    document_hash TEXT NOT NULL,  -- SHA-256 at event time
    auth_method TEXT,  -- 'email_link', 'sms_otp', 'id_verification'
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Immutable: no UPDATE/DELETE allowed (use triggers to enforce)
```

## Legal Citation Format (Bluebook)

### Common Patterns
```
Case:       Brown v. Board of Education, 347 U.S. 483 (1954)
            [Party1] v. [Party2], [Volume] [Reporter] [Page] ([Court] [Year])

Statute:    42 U.S.C. § 1983 (2018)
            [Title] [Code] § [Section] ([Year])

Regulation: 17 C.F.R. § 240.10b-5 (2023)
            [Title] [C.F.R.] § [Section] ([Year])
```

### LLM Mistakes to Avoid
- NEVER hallucinate case citations (fabricated volume/page numbers)
- NEVER invent statute sections that don't exist
- Always verify citations against official sources before inclusion
- Use "cf." (compare) and "see also" only when the authority is analogous, not directly on point
- Pinpoint citations (specific page) preferred over general citations

## Compliance Monitoring Patterns

### SOC 2 Control Evidence Collection
```python
CONTROLS = {
    'CC6.1': {
        'description': 'Logical and physical access controls',
        'evidence': [
            {'type': 'automated', 'source': 'iam_audit_logs', 'frequency': 'continuous'},
            {'type': 'automated', 'source': 'mfa_enrollment_report', 'frequency': 'daily'},
            {'type': 'manual', 'source': 'access_review_spreadsheet', 'frequency': 'quarterly'},
        ]
    },
    'CC7.2': {
        'description': 'System monitoring and anomaly detection',
        'evidence': [
            {'type': 'automated', 'source': 'alerting_system_logs', 'frequency': 'continuous'},
            {'type': 'automated', 'source': 'incident_response_tickets', 'frequency': 'per_event'},
        ]
    },
}
```

### Regulatory Calendar
```javascript
const COMPLIANCE_DEADLINES = {
  GDPR: {
    dsar_response: 30,       // days from request
    breach_notification: 72, // hours to DPA
    dpia_before_processing: true,
  },
  SOX: {
    quarterly_certification: 'Q+45days',
    annual_report: 'FY+60days',
    material_weakness_disclosure: 'immediate',
  },
  PCI_DSS: {
    quarterly_scan: true,
    annual_assessment: true,
    incident_report: 72, // hours
  },
};
```
