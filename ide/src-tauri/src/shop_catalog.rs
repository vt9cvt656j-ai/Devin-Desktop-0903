use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_LIMIT: usize = 12;
const MAX_LIMIT: usize = 50;

static HTTP: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(60))
        .user_agent("Michael-IDE/1.0 (+https://github.com/fendoushaonian/Devin-Desktop)")
        .build()
        .unwrap_or_else(|_| Client::new())
});

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShopSourceState {
    Success,
    Empty,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShopSourceStatus {
    pub source: String,
    pub status: ShopSourceState,
    pub result_count: usize,
    pub detail: String,
    pub data_as_of: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShopCatalogItem {
    pub name: String,
    pub brand: Option<String>,
    pub sku: Option<String>,
    pub price: Option<String>,
    pub compare_at_price: Option<String>,
    pub currency: Option<String>,
    pub availability: Option<String>,
    pub source: String,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub data_as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShopCatalogResponse {
    pub query: String,
    pub store_url: Option<String>,
    pub items: Vec<ShopCatalogItem>,
    pub source_statuses: Vec<ShopSourceStatus>,
    pub limitations: Vec<String>,
    /// Unix timestamp in seconds when Michael IDE finished this request. It is
    /// not a merchant inventory update timestamp.
    pub retrieved_at: u64,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn bounded_limit(limit: Option<u32>) -> usize {
    limit
        .map(|value| value.clamp(1, MAX_LIMIT as u32) as usize)
        .unwrap_or(DEFAULT_LIMIT)
}

fn status(
    source: &str,
    state: ShopSourceState,
    count: usize,
    detail: impl Into<String>,
    data_as_of: Option<String>,
    source_url: Option<String>,
) -> ShopSourceStatus {
    ShopSourceStatus {
        source: source.into(),
        status: state,
        result_count: count,
        detail: detail.into(),
        data_as_of,
        source_url,
    }
}

fn response(
    query: String,
    store_url: Option<String>,
    items: Vec<ShopCatalogItem>,
    source_statuses: Vec<ShopSourceStatus>,
    mut limitations: Vec<String>,
) -> ShopCatalogResponse {
    limitations.push("retrieved_at is when Michael IDE completed this request; it is not the merchant's product, price, or inventory update time.".into());
    limitations.push("Only public, unauthenticated sources are used. Missing price, currency, stock, rating, shipping, or discount fields must remain unknown instead of being guessed.".into());
    limitations.push("A source status of success means the endpoint/page was reachable and parseable in this request; it does not prove catalog completeness, merchant accuracy, or checkout availability.".into());
    ShopCatalogResponse {
        query,
        store_url,
        items,
        source_statuses,
        limitations,
        retrieved_at: unix_now(),
    }
}

async fn timed<T, F>(source: &'static str, future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    match tokio::time::timeout(SOURCE_TIMEOUT, future).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "{source} timed out after {} seconds",
            SOURCE_TIMEOUT.as_secs()
        )),
    }
}

async fn response_bytes(source: &str, request: reqwest::RequestBuilder) -> Result<Vec<u8>, String> {
    let mut response = request
        .send()
        .await
        .map_err(|error| format!("{source} request failed: {}", error.without_url()))?;
    let http_status = response.status();
    if !http_status.is_success() {
        return Err(format!("{source} returned HTTP {http_status}"));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(format!("{source} response exceeded the byte limit"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("{source} response read failed: {}", error.without_url()))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(format!("{source} response exceeded the byte limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn response_text(source: &str, request: reqwest::RequestBuilder) -> Result<String, String> {
    let bytes = response_bytes(source, request).await?;
    String::from_utf8(bytes).map_err(|_| format!("{source} response was not valid UTF-8"))
}

async fn response_json(source: &str, request: reqwest::RequestBuilder) -> Result<Value, String> {
    let bytes = response_bytes(source, request).await?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{source} returned invalid JSON: {error}"))
}

fn clean_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    })
}

fn first_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(value) = clean_string(Some(value)) {
        return Some(value);
    }
    if let Some(values) = value.as_array() {
        return values.iter().find_map(|item| first_string(Some(item)));
    }
    if let Some(object) = value.as_object() {
        for key in ["url", "src", "contentUrl", "thumbnailUrl", "name"] {
            if let Some(value) = first_string(object.get(key)) {
                return Some(value);
            }
        }
    }
    None
}

fn type_matches(value: &Value, expected: &str) -> bool {
    let Some(kind) = value.get("@type").or_else(|| value.get("type")) else {
        return false;
    };
    match kind {
        Value::String(kind) => kind.eq_ignore_ascii_case(expected),
        Value::Array(values) => values.iter().any(|item| {
            item.as_str()
                .is_some_and(|kind| kind.eq_ignore_ascii_case(expected))
        }),
        _ => false,
    }
}

fn brand_name(value: &Value) -> Option<String> {
    clean_string(value.get("brand")).or_else(|| {
        value
            .get("brand")
            .and_then(|brand| clean_string(brand.get("name")))
    })
}

fn normalize_availability(value: Option<String>) -> Option<String> {
    value.map(|value| {
        let lower = value.to_lowercase();
        lower
            .rsplit(['/', '#'])
            .next()
            .filter(|tail| !tail.is_empty())
            .unwrap_or(value.as_str())
            .to_string()
    })
}

fn price_from_offer(offer: &Value) -> Option<String> {
    clean_string(offer.get("price")).or_else(|| {
        offer
            .get("priceSpecification")
            .and_then(|spec| clean_string(spec.get("price")))
    })
}

fn currency_from_offer(offer: &Value) -> Option<String> {
    clean_string(offer.get("priceCurrency")).or_else(|| {
        offer
            .get("priceSpecification")
            .and_then(|spec| clean_string(spec.get("priceCurrency")))
    })
}

fn source_url(base: &Url, value: Option<String>) -> Option<String> {
    value.and_then(|url| {
        Url::parse(&url)
            .or_else(|_| base.join(&url))
            .map(|url| url.to_string())
            .ok()
    })
}

fn image_url(base: &Url, value: Option<String>) -> Option<String> {
    source_url(base, value)
}

fn json_ld_items_from_product(product: &Value, base: &Url, source: &str) -> Vec<ShopCatalogItem> {
    let name = clean_string(product.get("name")).unwrap_or_else(|| "Unnamed product".into());
    let brand = brand_name(product);
    let sku = clean_string(product.get("sku")).or_else(|| clean_string(product.get("mpn")));
    let image = image_url(base, first_string(product.get("image")));
    let product_url =
        source_url(base, clean_string(product.get("url"))).or_else(|| Some(base.to_string()));
    let data_as_of = clean_string(product.get("dateModified"))
        .or_else(|| clean_string(product.get("datePublished")));
    let offers = product.get("offers");
    let offer_values: Vec<&Value> = match offers {
        Some(Value::Array(values)) => values.iter().collect(),
        Some(value) => vec![value],
        None => Vec::new(),
    };

    if offer_values.is_empty() {
        return vec![ShopCatalogItem {
            name,
            brand,
            sku,
            price: None,
            compare_at_price: None,
            currency: None,
            availability: None,
            source: source.into(),
            source_url: product_url,
            image_url: image,
            data_as_of,
        }];
    }

    offer_values
        .into_iter()
        .map(|offer| ShopCatalogItem {
            name: clean_string(offer.get("name")).unwrap_or_else(|| name.clone()),
            brand: brand.clone(),
            sku: clean_string(offer.get("sku")).or_else(|| sku.clone()),
            price: price_from_offer(offer),
            compare_at_price: None,
            currency: currency_from_offer(offer),
            availability: normalize_availability(clean_string(offer.get("availability"))),
            source: source.into(),
            source_url: source_url(base, clean_string(offer.get("url")))
                .or_else(|| product_url.clone()),
            image_url: image.clone(),
            data_as_of: clean_string(offer.get("priceValidUntil"))
                .or_else(|| clean_string(offer.get("availabilityStarts")))
                .or_else(|| data_as_of.clone()),
        })
        .collect()
}

fn json_ld_items_from_offer(offer: &Value, base: &Url, source: &str) -> Option<ShopCatalogItem> {
    let item = offer.get("itemOffered").unwrap_or(offer);
    let name = clean_string(item.get("name")).or_else(|| clean_string(offer.get("name")))?;
    Some(ShopCatalogItem {
        name,
        brand: brand_name(item),
        sku: clean_string(item.get("sku")).or_else(|| clean_string(offer.get("sku"))),
        price: price_from_offer(offer),
        compare_at_price: None,
        currency: currency_from_offer(offer),
        availability: normalize_availability(clean_string(offer.get("availability"))),
        source: source.into(),
        source_url: source_url(base, clean_string(offer.get("url")))
            .or_else(|| source_url(base, clean_string(item.get("url"))))
            .or_else(|| Some(base.to_string())),
        image_url: image_url(base, first_string(item.get("image"))),
        data_as_of: clean_string(offer.get("priceValidUntil")),
    })
}

fn walk_json_ld(value: &Value, base: &Url, items: &mut Vec<ShopCatalogItem>) {
    if type_matches(value, "Product") {
        items.extend(json_ld_items_from_product(value, base, "json_ld_product"));
    } else if type_matches(value, "Offer") || type_matches(value, "AggregateOffer") {
        if let Some(item) = json_ld_items_from_offer(value, base, "json_ld_offer") {
            items.push(item);
        }
    }

    if let Some(graph) = value.get("@graph").and_then(Value::as_array) {
        for item in graph {
            walk_json_ld(item, base, items);
        }
    }
    if let Some(list) = value.get("itemListElement").and_then(Value::as_array) {
        for entry in list {
            if let Some(item) = entry.get("item") {
                walk_json_ld(item, base, items);
            } else {
                walk_json_ld(entry, base, items);
            }
        }
    }
    if let Some(values) = value.as_array() {
        for item in values {
            walk_json_ld(item, base, items);
        }
    }
}

fn parse_json_ld_catalog(html: &str, base: &Url, limit: usize) -> Vec<ShopCatalogItem> {
    let document = Html::parse_document(html);
    let selector = match Selector::parse(r#"script[type="application/ld+json"]"#) {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };
    let mut items = Vec::new();
    for script in document.select(&selector) {
        let text = script.text().collect::<Vec<_>>().join("");
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            walk_json_ld(&value, base, &mut items);
        }
        if items.len() >= limit {
            break;
        }
    }
    dedupe_items(items, limit)
}

fn shopify_product_url(
    base: &Url,
    handle: Option<&str>,
    variant_id: Option<String>,
) -> Option<String> {
    let handle = handle?;
    let mut url = base.join(&format!("/products/{handle}")).ok()?;
    if let Some(variant_id) = variant_id.filter(|value| !value.trim().is_empty()) {
        url.query_pairs_mut().append_pair("variant", &variant_id);
    }
    Some(url.to_string())
}

fn parse_shopify_catalog(payload: &Value, base: &Url, limit: usize) -> Vec<ShopCatalogItem> {
    let mut items = Vec::new();
    let Some(products) = payload.get("products").and_then(Value::as_array) else {
        return items;
    };
    for product in products {
        let product_name = clean_string(product.get("title"))
            .or_else(|| clean_string(product.get("name")))
            .unwrap_or_else(|| "Unnamed product".into());
        let brand = clean_string(product.get("vendor"));
        let handle = clean_string(product.get("handle"));
        let product_image = product
            .get("images")
            .and_then(Value::as_array)
            .and_then(|images| images.first())
            .and_then(|image| first_string(Some(image)))
            .or_else(|| first_string(product.get("featured_image")));
        let image = image_url(base, product_image);
        let variants = product.get("variants").and_then(Value::as_array);
        if let Some(variants) = variants.filter(|variants| !variants.is_empty()) {
            for variant in variants {
                let variant_title = clean_string(variant.get("title")).unwrap_or_default();
                let name = if variant_title.is_empty()
                    || variant_title.eq_ignore_ascii_case("default title")
                {
                    product_name.clone()
                } else {
                    format!("{product_name} - {variant_title}")
                };
                let availability = match variant.get("available").and_then(Value::as_bool) {
                    Some(true) => Some("available".into()),
                    Some(false) => Some("unavailable".into()),
                    None => None,
                };
                items.push(ShopCatalogItem {
                    name,
                    brand: brand.clone(),
                    sku: clean_string(variant.get("sku")),
                    price: clean_string(variant.get("price")),
                    compare_at_price: clean_string(variant.get("compare_at_price")),
                    currency: None,
                    availability,
                    source: "shopify_products_json".into(),
                    source_url: shopify_product_url(
                        base,
                        handle.as_deref(),
                        clean_string(variant.get("id")),
                    ),
                    image_url: image.clone(),
                    data_as_of: clean_string(product.get("updated_at")),
                });
                if items.len() >= limit {
                    return dedupe_items(items, limit);
                }
            }
        } else {
            items.push(ShopCatalogItem {
                name: product_name,
                brand,
                sku: None,
                price: None,
                compare_at_price: None,
                currency: None,
                availability: None,
                source: "shopify_products_json".into(),
                source_url: shopify_product_url(base, handle.as_deref(), None),
                image_url: image,
                data_as_of: clean_string(product.get("updated_at")),
            });
        }
        if items.len() >= limit {
            break;
        }
    }
    dedupe_items(items, limit)
}

fn dedupe_items(items: Vec<ShopCatalogItem>, limit: usize) -> Vec<ShopCatalogItem> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for item in items {
        let key = format!(
            "{}|{}|{}|{}",
            item.name.to_lowercase(),
            item.sku.clone().unwrap_or_default().to_lowercase(),
            item.price.clone().unwrap_or_default(),
            item.source_url.clone().unwrap_or_default()
        );
        if seen.insert(key) {
            unique.push(item);
            if unique.len() >= limit {
                break;
            }
        }
    }
    unique
}

fn extract_url_candidate(query: &str, explicit: Option<String>) -> Option<String> {
    explicit
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            query
                .split_whitespace()
                .find(|part| part.starts_with("http://") || part.starts_with("https://"))
                .map(|part| {
                    part.trim_matches(|ch| matches!(ch, ',' | '，' | ')' | '）' | '"' | '\''))
                        .to_string()
                })
        })
        .or_else(|| {
            let trimmed = query.trim();
            let looks_like_domain = trimmed.contains('.')
                && !trimmed.contains(' ')
                && !trimmed.contains('/')
                && trimmed
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'));
            looks_like_domain.then(|| trimmed.to_string())
        })
}

fn normalize_store_url(candidate: &str) -> Result<Url, String> {
    let with_scheme = if candidate.starts_with("http://") || candidate.starts_with("https://") {
        candidate.to_string()
    } else {
        format!("https://{candidate}")
    };
    let url = Url::parse(&with_scheme).map_err(|error| format!("invalid store url: {error}"))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err("store url must use http or https".into()),
    }
}

fn shopify_products_url(base: &Url, limit: usize) -> Url {
    let mut url = base.clone();
    url.set_path("/products.json");
    url.set_query(Some(&format!("limit={}", limit.min(250))));
    url
}

#[tauri::command]
pub async fn shop_catalog(
    query: String,
    url: Option<String>,
    limit: Option<u32>,
) -> Result<ShopCatalogResponse, String> {
    let limit = bounded_limit(limit);
    let Some(candidate) = extract_url_candidate(&query, url) else {
        return Ok(response(
            query,
            None,
            Vec::new(),
            vec![status(
                "store_url",
                ShopSourceState::Skipped,
                0,
                "No store URL was supplied. Use a known merchant/product URL, or first find a public official website with local_discovery/web_search, then call shop_catalog on that URL.",
                None,
                None,
            )],
            vec!["No URL means Michael IDE cannot truthfully load a merchant catalog or current prices; no search-result snippets were treated as prices.".into()],
        ));
    };
    let base = normalize_store_url(&candidate)?;
    let mut all_items = Vec::new();
    let mut statuses = Vec::new();

    let shopify_url = shopify_products_url(&base, limit);
    match timed(
        "shopify_products_json",
        response_json(
            "shopify_products_json",
            HTTP.get(shopify_url.clone())
                .header("accept", "application/json,text/plain,*/*"),
        ),
    )
    .await
    {
        Ok(payload) => {
            let items = parse_shopify_catalog(&payload, &base, limit);
            let count = items.len();
            statuses.push(status(
                "shopify_products_json",
                if count == 0 {
                    ShopSourceState::Empty
                } else {
                    ShopSourceState::Success
                },
                count,
                if count == 0 {
                    "The Shopify public products.json endpoint responded but exposed no products in this request."
                } else {
                    "The Shopify public products.json endpoint returned parseable product/variant data."
                },
                items.iter().filter_map(|item| item.data_as_of.clone()).max(),
                Some(shopify_url.to_string()),
            ));
            all_items.extend(items);
        }
        Err(error) => statuses.push(status(
            "shopify_products_json",
            ShopSourceState::Failed,
            0,
            error,
            None,
            Some(shopify_url.to_string()),
        )),
    }

    if all_items.len() < limit {
        match timed(
            "html_json_ld",
            response_text(
                "html_json_ld",
                HTTP.get(base.clone())
                    .header("accept", "text/html,application/xhtml+xml,*/*"),
            ),
        )
        .await
        {
            Ok(html) => {
                let items = parse_json_ld_catalog(&html, &base, limit - all_items.len());
                let count = items.len();
                statuses.push(status(
                    "html_json_ld",
                    if count == 0 {
                        ShopSourceState::Empty
                    } else {
                        ShopSourceState::Success
                    },
                    count,
                    if count == 0 {
                        "The public page was reachable, but no schema.org Product/Offer JSON-LD was found."
                    } else {
                        "The public page exposed parseable schema.org Product/Offer JSON-LD."
                    },
                    items.iter().filter_map(|item| item.data_as_of.clone()).max(),
                    Some(base.to_string()),
                ));
                all_items.extend(items);
            }
            Err(error) => statuses.push(status(
                "html_json_ld",
                ShopSourceState::Failed,
                0,
                error,
                None,
                Some(base.to_string()),
            )),
        }
    } else {
        statuses.push(status(
            "html_json_ld",
            ShopSourceState::Skipped,
            0,
            "Skipped because the public products endpoint already filled the requested limit.",
            None,
            Some(base.to_string()),
        ));
    }

    let items = dedupe_items(all_items, limit);
    Ok(response(
        query,
        Some(base.to_string()),
        items,
        statuses,
        vec![
            "Shopify products.json often lacks currency; when currency is absent it must not be inferred from locale or domain.".into(),
            "JSON-LD offers can be incomplete or stale; priceValidUntil/dateModified are preserved when present but not invented.".into(),
            "This tool does not log in, bypass CAPTCHA, scrape private APIs, or claim checkout-level stock/shipping/tax accuracy.".into(),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_shopify_products_without_inventing_currency() {
        let base = Url::parse("https://example-shop.test").unwrap();
        let payload = json!({
            "products": [{
                "title": "Heavy Hoodie",
                "vendor": "Real Brand",
                "handle": "heavy-hoodie",
                "updated_at": "2026-07-14T10:00:00-07:00",
                "images": [{ "src": "https://cdn.example/hoodie.jpg" }],
                "variants": [{
                    "id": 123,
                    "title": "Black / L",
                    "sku": "HH-BLK-L",
                    "price": "79.00",
                    "compare_at_price": "99.00",
                    "available": true
                }]
            }]
        });
        let items = parse_shopify_catalog(&payload, &base, 10);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Heavy Hoodie - Black / L");
        assert_eq!(items[0].price.as_deref(), Some("79.00"));
        assert_eq!(items[0].compare_at_price.as_deref(), Some("99.00"));
        assert_eq!(items[0].currency, None);
        assert_eq!(items[0].availability.as_deref(), Some("available"));
        assert!(items[0]
            .source_url
            .as_deref()
            .unwrap()
            .contains("/products/heavy-hoodie?variant=123"));
    }

    #[test]
    fn parses_json_ld_product_offers() {
        let base = Url::parse("https://store.test/products/tea").unwrap();
        let html = r#"
          <html><head>
            <script type="application/ld+json">
              {
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Jasmine Tea",
                "brand": {"@type":"Brand","name":"Garden"},
                "sku": "TEA-JAS",
                "image": ["/tea.jpg"],
                "offers": [{
                  "@type": "Offer",
                  "price": "12.50",
                  "priceCurrency": "USD",
                  "availability": "https://schema.org/InStock",
                  "url": "/products/tea"
                }]
              }
            </script>
          </head></html>
        "#;
        let items = parse_json_ld_catalog(html, &base, 10);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Jasmine Tea");
        assert_eq!(items[0].brand.as_deref(), Some("Garden"));
        assert_eq!(items[0].sku.as_deref(), Some("TEA-JAS"));
        assert_eq!(items[0].price.as_deref(), Some("12.50"));
        assert_eq!(items[0].currency.as_deref(), Some("USD"));
        assert_eq!(items[0].availability.as_deref(), Some("instock"));
        assert_eq!(
            items[0].image_url.as_deref(),
            Some("https://store.test/tea.jpg")
        );
    }

    #[test]
    fn no_url_returns_skipped_status_instead_of_fake_prices() {
        assert!(extract_url_candidate("附近奶茶店价格", None).is_none());
        let candidate =
            extract_url_candidate("查 https://shop.example/products/a 价格", None).unwrap();
        assert_eq!(candidate, "https://shop.example/products/a");
    }
}
