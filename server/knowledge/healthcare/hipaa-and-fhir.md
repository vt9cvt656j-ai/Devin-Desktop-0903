# Healthcare: HIPAA Compliance & HL7 FHIR

## HIPAA Technical Safeguards Checklist (45 CFR 164.312)

### Access Controls (Required)
- Unique user IDs for every account (no shared logins)
- Role-based access: clinician sees full record; billing sees codes+payer only; front desk sees demographics
- Emergency access procedure (break-the-glass with mandatory post-access review)
- Auto-logoff after 15 minutes inactivity
- Encryption: AES-256 at rest, TLS 1.2+ in transit

### PHI (Protected Health Information) — 18 Identifiers to NEVER Expose
```
Names, Dates (except year), Phone, Fax, Email, SSN, MRN,
Health plan ID, Account numbers, Certificate/license numbers,
Vehicle IDs, Device IDs, URLs, IPs, Biometrics, Photos,
Any other unique identifying number
```

### Audit Logging (Required)
```sql
CREATE TABLE phi_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    patient_id UUID NOT NULL,
    resource_type TEXT NOT NULL,  -- 'Observation', 'MedicationRequest', etc.
    action TEXT NOT NULL,         -- 'read', 'create', 'update', 'delete'
    purpose TEXT NOT NULL,        -- 'treatment', 'payment', 'operations'
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT
);
-- Retain for 6 years minimum (45 CFR 164.530(j))
-- NEVER delete, only archive
```

### PHI in Code — Rules
```
NEVER: log PHI to application logs, stdout, or error tracking (Sentry, etc.)
NEVER: include PHI in URL parameters or query strings
NEVER: store PHI in localStorage/cookies
NEVER: use real patient data in dev/test (use synthetic data — Synthea)
ALWAYS: encrypt PHI columns with application-level encryption (pgcrypto)
ALWAYS: mask PHI in non-production environments
ALWAYS: sanitize error messages before returning to client
```

## HL7 FHIR R4 Core Patterns

### Essential Resources
```json
// Patient
{
  "resourceType": "Patient",
  "id": "example",
  "identifier": [{"system": "http://hospital.example/mrn", "value": "12345"}],
  "name": [{"family": "Smith", "given": ["John"]}],
  "gender": "male",
  "birthDate": "1990-01-15"
}

// Observation (e.g., blood pressure)
{
  "resourceType": "Observation",
  "status": "final",
  "category": [{"coding": [{"system": "http://terminology.hl7.org/CodeSystem/observation-category", "code": "vital-signs"}]}],
  "code": {"coding": [{"system": "http://loinc.org", "code": "85354-9", "display": "Blood pressure panel"}]},
  "subject": {"reference": "Patient/example"},
  "effectiveDateTime": "2024-01-15T10:30:00Z",
  "component": [
    {"code": {"coding": [{"system": "http://loinc.org", "code": "8480-6"}]}, "valueQuantity": {"value": 120, "unit": "mmHg"}},
    {"code": {"coding": [{"system": "http://loinc.org", "code": "8462-4"}]}, "valueQuantity": {"value": 80, "unit": "mmHg"}}
  ]
}
```

### FHIR Code Systems
| System URI | Use |
|-----------|-----|
| `http://loinc.org` | Lab tests, vital signs, documents |
| `http://snomed.info/sct` | Clinical terms, diagnoses, procedures |
| `http://www.nlm.nih.gov/research/umls/rxnorm` | Medications |
| `http://hl7.org/fhir/sid/icd-10-cm` | Diagnosis codes (billing) |
| `http://www.ama-assn.org/go/cpt` | Procedure codes (billing) |

### SMART on FHIR Auth
```
1. Register app with EHR (get client_id)
2. Authorization: GET /authorize?scope=patient/Observation.read+launch
3. Token exchange: POST /token (authorization_code + PKCE)
4. API calls: Authorization: Bearer {access_token}
5. Scopes: patient/{Resource}.{read|write} or user/{Resource}.{read|write}
```

### Common LLM Mistakes in FHIR
- Inventing code system URIs (use ONLY official URIs above)
- Using incorrect LOINC/SNOMED codes (always verify against terminology server)
- Missing `status` field (required on Observation, MedicationRequest, Condition)
- Wrong date format (must be YYYY-MM-DD or YYYY-MM-DDThh:mm:ssZ)
- Omitting `resourceType` (required on every FHIR resource)

## Clinical Data Patterns

### Lab Result Display
```javascript
function formatLabResult(observation) {
  const value = observation.valueQuantity?.value;
  const unit = observation.valueQuantity?.unit;
  const range = observation.referenceRange?.[0];

  let status = 'normal';
  if (range?.low && value < range.low.value) status = 'low';
  if (range?.high && value > range.high.value) status = 'high';

  return { value, unit, status, range: `${range?.low?.value}-${range?.high?.value}` };
}
// Display: green=normal, yellow=borderline (±10%), red=critical
```

### Medication Interaction Check
```
1. Get patient's active medications (MedicationRequest where status=active)
2. For each pair, check interaction database (RxNorm + DrugBank API)
3. Classify: contraindicated (block) | major (warn+confirm) | moderate (warn) | minor (info)
4. Display at prescribing time — BEFORE the order is signed
5. Log interaction overrides with clinician justification
```
