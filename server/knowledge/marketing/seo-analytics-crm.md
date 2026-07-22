# Marketing: SEO, Analytics, CRM & Growth Engineering

## Technical SEO

### Server-Side Rendering for SEO
```javascript
// Next.js: generateMetadata (App Router)
export async function generateMetadata({ params }) {
  const product = await getProduct(params.slug);
  return {
    title: `${product.name} | MyStore`,
    description: product.description.slice(0, 155),
    openGraph: {
      title: product.name,
      description: product.description.slice(0, 155),
      images: [{ url: product.image_url, width: 1200, height: 630, alt: product.name }],
      type: 'product',
    },
    alternates: {
      canonical: `https://mystore.com/products/${params.slug}`,
      languages: { 'zh-CN': `/zh/products/${params.slug}`, 'en-US': `/en/products/${params.slug}` },
    },
  };
}
```

### Structured Data (JSON-LD)
```javascript
function productJsonLd(product) {
  return {
    '@context': 'https://schema.org',
    '@type': 'Product',
    name: product.name,
    description: product.description,
    image: product.images,
    sku: product.sku,
    brand: { '@type': 'Brand', name: product.brand },
    offers: {
      '@type': 'Offer',
      url: `https://mystore.com/products/${product.slug}`,
      priceCurrency: product.currency,
      price: (product.price / 100).toFixed(2),
      availability: product.in_stock
        ? 'https://schema.org/InStock'
        : 'https://schema.org/OutOfStock',
      priceValidUntil: new Date(Date.now() + 30 * 86400000).toISOString().split('T')[0],
    },
    aggregateRating: product.rating_count > 0 ? {
      '@type': 'AggregateRating',
      ratingValue: product.avg_rating,
      reviewCount: product.rating_count,
    } : undefined,
  };
}

// Render: <script type="application/ld+json">{JSON.stringify(productJsonLd(product))}</script>
```

### Sitemap Generation
```javascript
async function generateSitemap() {
  const pages = await db.query(`
    SELECT slug, updated_at FROM products WHERE status = 'active'
    UNION ALL
    SELECT slug, updated_at FROM blog_posts WHERE status = 'published'
    UNION ALL
    SELECT slug, updated_at FROM categories
  `);

  const urls = pages.map(p => `
    <url>
      <loc>https://mystore.com/${p.slug}</loc>
      <lastmod>${p.updated_at.toISOString()}</lastmod>
      <changefreq>${p.slug.startsWith('blog') ? 'weekly' : 'daily'}</changefreq>
      <priority>${p.slug === '' ? '1.0' : '0.8'}</priority>
    </url>`).join('');

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>`;
}

// robots.txt
// User-agent: *
// Allow: /
// Disallow: /api/
// Disallow: /admin/
// Sitemap: https://mystore.com/sitemap.xml
```

### Core Web Vitals Optimization
```
LCP (Largest Contentful Paint) < 2.5s:
  - Preload hero image: <link rel="preload" as="image" href="hero.webp">
  - Use next/image or <img loading="eager"> for above-fold
  - Server-render critical content (no client-side data fetching for LCP element)

FID/INP (Interaction to Next Paint) < 200ms:
  - Break long tasks: yield to main thread with scheduler.yield() or setTimeout(0)
  - Defer non-critical JS: <script defer> or dynamic import()
  - Avoid layout thrashing: batch DOM reads then writes

CLS (Cumulative Layout Shift) < 0.1:
  - Set explicit width/height on images and videos
  - Reserve space for ads/embeds with aspect-ratio or min-height
  - Avoid inserting content above existing content (banners, cookie bars)
  - Use CSS containment: contain: layout style paint
```

## Analytics & Event Tracking

### Event Taxonomy
```javascript
// Use verb_noun naming: 'view_page', 'click_button', 'submit_form', 'purchase_complete'
const EVENT_SCHEMA = {
  page_view: {
    required: ['page_path', 'page_title'],
    optional: ['referrer', 'utm_source', 'utm_medium', 'utm_campaign'],
  },
  product_viewed: {
    required: ['product_id', 'product_name', 'price'],
    optional: ['category', 'variant', 'position'],
  },
  add_to_cart: {
    required: ['product_id', 'quantity', 'price'],
    optional: ['variant', 'cart_value'],
  },
  purchase_complete: {
    required: ['order_id', 'total', 'currency', 'items'],
    optional: ['coupon', 'shipping', 'tax', 'payment_method'],
  },
  signup_complete: {
    required: ['method'],  // 'email', 'google', 'github'
    optional: ['referral_code', 'utm_source'],
  },
};
```

### Server-Side Event Collection
```javascript
async function trackEvent(req, eventName, properties = {}) {
  const schema = EVENT_SCHEMA[eventName];
  if (schema) {
    for (const key of schema.required) {
      if (!(key in properties)) throw new Error(`Missing required property: ${key}`);
    }
  }

  const event = {
    event_name: eventName,
    user_id: req.userId || null,
    anonymous_id: req.cookies.anon_id || generateAnonId(),
    properties,
    context: {
      ip: req.ip,
      user_agent: req.headers['user-agent'],
      locale: req.headers['accept-language']?.split(',')[0],
      page_url: req.headers['referer'],
      utm: extractUtm(req),
    },
    timestamp: Date.now(),
  };

  // Write to Kafka / Redis stream for async processing
  await producer.send({ topic: 'analytics-events', messages: [{ value: JSON.stringify(event) }] });
}

function extractUtm(req) {
  const url = new URL(req.headers['referer'] || '', 'https://example.com');
  return {
    source: url.searchParams.get('utm_source'),
    medium: url.searchParams.get('utm_medium'),
    campaign: url.searchParams.get('utm_campaign'),
    term: url.searchParams.get('utm_term'),
    content: url.searchParams.get('utm_content'),
  };
}
```

### Funnel Analysis
```sql
-- Conversion funnel: visit → signup → activate → purchase
WITH funnel AS (
    SELECT
        user_id,
        MIN(CASE WHEN event_name = 'page_view' THEN created_at END) AS visited,
        MIN(CASE WHEN event_name = 'signup_complete' THEN created_at END) AS signed_up,
        MIN(CASE WHEN event_name = 'activation_complete' THEN created_at END) AS activated,
        MIN(CASE WHEN event_name = 'purchase_complete' THEN created_at END) AS purchased
    FROM analytics_events
    WHERE created_at > NOW() - interval '30 days'
    GROUP BY user_id
)
SELECT
    COUNT(*) AS visited,
    COUNT(signed_up) AS signed_up,
    COUNT(activated) AS activated,
    COUNT(purchased) AS purchased,
    ROUND(100.0 * COUNT(signed_up) / NULLIF(COUNT(*), 0), 1) AS visit_to_signup_pct,
    ROUND(100.0 * COUNT(purchased) / NULLIF(COUNT(signed_up), 0), 1) AS signup_to_purchase_pct
FROM funnel;
```

### Cohort Retention
```sql
WITH cohorts AS (
    SELECT
        user_id,
        DATE_TRUNC('week', MIN(created_at)) AS cohort_week
    FROM analytics_events
    WHERE event_name = 'signup_complete'
    GROUP BY user_id
),
activity AS (
    SELECT
        c.cohort_week,
        EXTRACT(WEEK FROM e.created_at - c.cohort_week)::int AS week_number,
        COUNT(DISTINCT e.user_id) AS active_users
    FROM analytics_events e
    JOIN cohorts c ON c.user_id = e.user_id
    GROUP BY c.cohort_week, week_number
)
SELECT
    cohort_week,
    week_number,
    active_users,
    ROUND(100.0 * active_users / FIRST_VALUE(active_users) OVER (
        PARTITION BY cohort_week ORDER BY week_number
    ), 1) AS retention_pct
FROM activity
ORDER BY cohort_week, week_number;
```

## Email Campaigns

### Transactional Email Architecture
```javascript
const EMAIL_TEMPLATES = {
  welcome: {
    subject: 'Welcome to {{app_name}}',
    delay: 0,
  },
  onboarding_day1: {
    subject: 'Getting started with {{app_name}}',
    delay: 86400000,
  },
  onboarding_day3: {
    subject: 'Have you tried {{feature_name}}?',
    delay: 259200000,
    condition: user => !user.has_used_feature,
  },
  churn_risk: {
    subject: 'We miss you, {{first_name}}',
    delay: 604800000,
    condition: user => user.days_inactive >= 7,
  },
};

async function sendDripEmail(userId, templateKey) {
  const template = EMAIL_TEMPLATES[templateKey];
  const user = await db.getUser(userId);

  // Suppression checks
  if (user.email_unsubscribed) return;
  if (await recentlySent(userId, templateKey, 86400000)) return;
  if (template.condition && !template.condition(user)) return;

  await emailService.send({
    to: user.email,
    template: templateKey,
    variables: { first_name: user.name.split(' ')[0], app_name: 'MyApp', ...user },
    headers: {
      'List-Unsubscribe': `<https://myapp.com/unsubscribe/${user.unsub_token}>`,
      'List-Unsubscribe-Post': 'List-Unsubscribe=One-Click',
    },
  });

  await db.insert('email_log', { user_id: userId, template: templateKey, sent_at: new Date() });
}
```

### Email Deliverability Checklist
```
1. SPF: TXT record — v=spf1 include:sendgrid.net ~all
2. DKIM: sign with 2048-bit key, rotate annually
3. DMARC: v=DMARC1; p=quarantine; rua=mailto:dmarc@myapp.com
4. List-Unsubscribe header: RFC 8058 one-click unsubscribe (required by Gmail/Yahoo 2024+)
5. Warm up new IPs: start with engaged users, ramp volume over 2-4 weeks
6. Bounce handling: remove hard bounces immediately, soft bounce after 3 attempts
7. Complaint rate < 0.1% (monitor via FBL — Feedback Loop)
```

## CRM Data Model

### Contacts & Deals Pipeline
```sql
CREATE TABLE contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE,
    first_name TEXT,
    last_name TEXT,
    company TEXT,
    title TEXT,
    phone TEXT,
    source TEXT,          -- 'organic','paid','referral','cold_outbound','event'
    lifecycle_stage TEXT DEFAULT 'lead',
    -- 'subscriber','lead','mql','sql','opportunity','customer','evangelist'
    owner_id UUID REFERENCES users(id),
    tags TEXT[],
    custom_fields JSONB DEFAULT '{}',
    last_activity_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE deals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contact_id UUID REFERENCES contacts(id),
    title TEXT NOT NULL,
    amount BIGINT,        -- cents
    currency CHAR(3) DEFAULT 'USD',
    stage TEXT NOT NULL DEFAULT 'prospecting',
    -- 'prospecting','qualification','proposal','negotiation','closed_won','closed_lost'
    probability SMALLINT, -- 0-100, auto-set by stage
    expected_close DATE,
    owner_id UUID REFERENCES users(id),
    lost_reason TEXT,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contact_id UUID REFERENCES contacts(id),
    deal_id UUID REFERENCES deals(id),
    activity_type TEXT NOT NULL,
    -- 'email_sent','email_opened','email_replied','call','meeting','note','task'
    subject TEXT,
    body TEXT,
    performed_by UUID REFERENCES users(id),
    performed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Pipeline value report
SELECT
    stage,
    COUNT(*) AS deal_count,
    SUM(amount) / 100.0 AS total_value,
    AVG(probability) AS avg_probability,
    SUM(amount * probability / 100) / 100.0 AS weighted_value
FROM deals
WHERE stage NOT IN ('closed_won', 'closed_lost')
GROUP BY stage
ORDER BY ARRAY_POSITION(
    ARRAY['prospecting','qualification','proposal','negotiation'], stage
);
```

### Lead Scoring
```javascript
const SCORING_RULES = [
  // Demographic scoring
  { condition: c => c.title?.match(/CTO|VP|Director|Head/i), points: 20, category: 'fit' },
  { condition: c => c.company_size > 100, points: 15, category: 'fit' },
  { condition: c => c.industry === 'technology', points: 10, category: 'fit' },

  // Behavioral scoring
  { condition: c => c.page_views > 10, points: 10, category: 'engagement' },
  { condition: c => c.visited_pricing, points: 20, category: 'intent' },
  { condition: c => c.downloaded_whitepaper, points: 15, category: 'engagement' },
  { condition: c => c.attended_demo, points: 30, category: 'intent' },
  { condition: c => c.days_inactive > 30, points: -20, category: 'decay' },
];

function scoreContact(contact) {
  let total = 0;
  const breakdown = {};
  for (const rule of SCORING_RULES) {
    if (rule.condition(contact)) {
      total += rule.points;
      breakdown[rule.category] = (breakdown[rule.category] || 0) + rule.points;
    }
  }
  return { total, breakdown, grade: total >= 80 ? 'A' : total >= 50 ? 'B' : total >= 20 ? 'C' : 'D' };
}

// MQL threshold: score >= 50 AND has_intent_signal → auto-assign to sales
```

## A/B Testing

### Experiment Framework
```javascript
function assignVariant(experimentId, userId, variants) {
  // Deterministic assignment: same user always gets same variant
  const hash = murmurhash3(`${experimentId}:${userId}`) % 100;
  let cumulative = 0;
  for (const variant of variants) {
    cumulative += variant.weight;  // e.g., {name: 'control', weight: 50}, {name: 'treatment', weight: 50}
    if (hash < cumulative) return variant.name;
  }
  return variants[0].name;
}

// Statistical significance check
function calculateSignificance(control, treatment) {
  // control/treatment: { visitors, conversions }
  const p1 = control.conversions / control.visitors;
  const p2 = treatment.conversions / treatment.visitors;
  const p = (control.conversions + treatment.conversions) / (control.visitors + treatment.visitors);
  const se = Math.sqrt(p * (1 - p) * (1 / control.visitors + 1 / treatment.visitors));
  const z = (p2 - p1) / se;
  // z > 1.96 → significant at p < 0.05 (two-tailed)
  return {
    control_rate: p1,
    treatment_rate: p2,
    lift: (p2 - p1) / p1,
    z_score: z,
    significant: Math.abs(z) > 1.96,
    p_value: 2 * (1 - normalCDF(Math.abs(z))),
  };
}

function normalCDF(x) {
  const t = 1 / (1 + 0.2316419 * Math.abs(x));
  const d = 0.3989422804 * Math.exp(-x * x / 2);
  const p = d * t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));
  return x > 0 ? 1 - p : p;
}
```

### Minimum Sample Size
```
For 80% power, 5% significance, baseline conversion 5%, minimum detectable effect 20% relative:
  n = 16 * p * (1-p) / (MDE * p)²
  n = 16 * 0.05 * 0.95 / (0.01)²
  n ≈ 7,600 per variant

Rule of thumb: don't peek at results before minimum sample size reached.
Running 50+ simultaneous tests? Use Bonferroni correction or FDR control.
```

## Internationalization (i18n)

### Translation Management
```javascript
// ICU MessageFormat — handles plurals, gender, select across languages
import { IntlMessageFormat } from 'intl-messageformat';

const messages = {
  en: {
    items_in_cart: '{count, plural, =0 {Cart is empty} one {# item in cart} other {# items in cart}}',
    greeting: '{gender, select, male {Mr.} female {Ms.} other {}} {name}',
    price: '{amount, number, ::currency/USD}',
  },
  zh: {
    items_in_cart: '{count, plural, =0 {购物车为空} other {购物车有 # 件商品}}',
    greeting: '{name}',
    price: '{amount, number, ::currency/CNY}',
  },
};

function t(key, locale, values) {
  const template = messages[locale]?.[key] || messages['en'][key];
  return new IntlMessageFormat(template, locale).format(values);
}

// t('items_in_cart', 'en', { count: 3 }) → "3 items in cart"
// t('items_in_cart', 'zh', { count: 3 }) → "购物车有 3 件商品"
```

### URL Strategy for Multi-Language
```
Subdirectory (recommended for SEO):
  myapp.com/en/products  myapp.com/zh/products

Subdomain:
  en.myapp.com/products  zh.myapp.com/products

hreflang tags (MUST include on every page):
  <link rel="alternate" hreflang="en" href="https://myapp.com/en/products" />
  <link rel="alternate" hreflang="zh" href="https://myapp.com/zh/products" />
  <link rel="alternate" hreflang="x-default" href="https://myapp.com/products" />
```

## Common LLM Mistakes in Marketing Tech
```
1. Client-side-only analytics (ad blockers block 30-40% of events)
2. Not deduplicating conversion events (webhook retries → inflated metrics)
3. Using Math.random() for A/B assignment (non-deterministic, user sees different variants)
4. Peeking at A/B test results before minimum sample size (inflated false positive rate)
5. Sending marketing emails without List-Unsubscribe header (deliverability killer since 2024)
6. Hardcoding strings instead of using i18n keys (impossible to translate later)
7. Not setting hreflang on multi-language pages (SEO penalty, wrong language in SERPs)
8. Storing PII in analytics events without consent (GDPR violation)
9. Not handling email bounce/complaint webhooks (IP reputation damage)
10. Missing UTM parameter tracking on landing pages (can't attribute conversions)
```
