use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

const NOMINATIM_SEARCH_URL: &str = "https://nominatim.openstreetmap.org/search";
const NOMINATIM_REVERSE_URL: &str = "https://nominatim.openstreetmap.org/reverse";
const ARCGIS_GEOCODE_URL: &str =
    "https://geocode.arcgis.com/arcgis/rest/services/World/GeocodeServer/findAddressCandidates";
const OVERPASS_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const OVERPASS_TOTAL_TIMEOUT: Duration = Duration::from_secs(25);
const OVERPASS_ENDPOINTS: [(&str, &str); 3] = [
    ("overpass.osm.ch", "https://overpass.osm.ch/api/interpreter"),
    (
        "overpass.openstreetmap.fr",
        "https://overpass.openstreetmap.fr/api/interpreter",
    ),
    ("overpass-api.de", "https://overpass-api.de/api/interpreter"),
];
const OPEN_METEO_URL: &str = "https://api.open-meteo.com/v1/forecast";
const SOURCE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_SOURCE_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const OVERPASS_MAXSIZE_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_RADIUS_M: u32 = 3_000;
const MIN_RADIUS_M: u32 = 100;
const MAX_RADIUS_M: u32 = 20_000;
const DEFAULT_LIMIT: u32 = 12;
const MAX_LIMIT: u32 = 30;
const ARCGIS_MIN_MATCH_SCORE: f64 = 90.0;

// ── 百度/高德 POI 富化（评分/人均/营业时间） ──────────────────────────────────
// AK/Key 通过环境变量注入。未设置时静默跳过，不影响 OSM 基础数据。
const BAIDU_PLACE_URL: &str = "https://api.map.baidu.com/place/v2/search";
const AMAP_PLACE_URL: &str = "https://restapi.amap.com/v3/place/around";

fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn baidu_ak() -> Option<String> {
    std::env::var("BAIDU_MAP_AK").ok().filter(|s| !s.is_empty())
}
fn amap_key() -> Option<String> {
    std::env::var("AMAP_KEY").ok().filter(|s| !s.is_empty())
}

static HTTP: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(60))
        .user_agent("Michael-IDE/1.0 (+https://github.com/fendoushaonian/Devin-Desktop)")
        .build()
        .unwrap_or_else(|_| Client::new())
});

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiscoveryCenter {
    pub label: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Where the coordinates came from. A reverse-geocoded label never changes
    /// supplied coordinates into a Nominatim-derived position.
    pub source: String,
    pub label_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiscoveryPlace {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    /// Great-circle distance, not road/walking distance.
    pub distance_m: u32,
    pub source: String,
    pub source_url: Option<String>,
    pub address: Option<String>,
    pub cuisine: Option<String>,
    /// Raw scheduled-hours text supplied by OpenStreetMap. It is not evaluated
    /// as an assertion that the place is currently open.
    pub opening_hours: Option<String>,
    /// These sources do not provide trustworthy normalized ratings or prices.
    pub rating: Option<f64>,
    pub price: Option<String>,
    /// Always unknown for this command: no source queried here is a live
    /// open/closed feed.
    pub open_now: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiscoveryWeather {
    pub temperature_c: Option<f64>,
    pub apparent_temperature_c: Option<f64>,
    pub precipitation_mm: Option<f64>,
    pub wind_speed_kmh: Option<f64>,
    pub weather_code: Option<i32>,
    pub condition: Option<String>,
    pub observed_at: Option<String>,
    pub timezone: Option<String>,
    pub source: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    Success,
    Empty,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceStatus {
    pub source: String,
    pub status: SourceState,
    pub result_count: usize,
    pub detail: String,
    /// Provider dataset/snapshot timestamp when the response exposes one. This
    /// is not necessarily the observation or verification time of an item.
    pub data_as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LocalDiscoveryResponse {
    pub center: Option<DiscoveryCenter>,
    /// Named OpenStreetMap POIs matching the requested local intent.
    pub places: Vec<DiscoveryPlace>,
    /// Nearby encyclopedia articles are contextual background, not POIs or
    /// recommendations. They are deliberately kept out of `places` ranking.
    pub nearby_context: Vec<DiscoveryPlace>,
    pub weather: Option<DiscoveryWeather>,
    pub source_statuses: Vec<SourceStatus>,
    pub limitations: Vec<String>,
    /// Unix timestamp in seconds, recorded after the source calls settle.
    pub retrieved_at: u64,
    pub radius_m: u32,
}

#[derive(Debug, Clone, PartialEq)]
enum LocationPlan {
    Coordinates(DiscoveryCenter),
    Geocode(String),
    Missing(String),
}

#[derive(Debug, Deserialize)]
struct NominatimHit {
    display_name: String,
    lat: String,
    lon: String,
    address: Option<NominatimAddress>,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    house_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NominatimReverse {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArcgisGeocodeResponse {
    #[serde(default)]
    candidates: Vec<ArcgisCandidate>,
    error: Option<ArcgisServiceError>,
}

#[derive(Debug, Deserialize)]
struct ArcgisCandidate {
    address: String,
    location: ArcgisLocation,
    score: f64,
    attributes: ArcgisAttributes,
}

#[derive(Debug, Deserialize)]
struct ArcgisLocation {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct ArcgisAttributes {
    #[serde(rename = "Match_addr")]
    match_address: Option<String>,
    #[serde(rename = "Addr_type")]
    address_type: Option<String>,
    #[serde(rename = "AddNum")]
    address_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArcgisServiceError {
    code: Option<i64>,
    message: Option<String>,
    #[serde(default)]
    details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ArcgisGeocodeMatch {
    center: DiscoveryCenter,
    score: f64,
    address_type: String,
}

#[derive(Debug, Deserialize)]
struct OverpassResponse {
    remark: Option<String>,
    osm3s: Option<OverpassMetadata>,
    #[serde(default)]
    elements: Vec<OverpassElement>,
}

#[derive(Debug, Deserialize)]
struct OverpassMetadata {
    timestamp_osm_base: Option<String>,
}

#[derive(Debug)]
struct OverpassPlacesResult {
    places: Vec<DiscoveryPlace>,
    data_as_of: Option<String>,
    warning: Option<String>,
    endpoint: String,
}

#[derive(Debug)]
struct OverpassCompletion {
    elements: Vec<OverpassElement>,
    warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OverpassElement {
    #[serde(rename = "type")]
    element_type: String,
    id: i64,
    lat: Option<f64>,
    lon: Option<f64>,
    center: Option<OverpassCenter>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct OverpassCenter {
    lat: f64,
    lon: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    timezone: Option<String>,
    current: Option<OpenMeteoCurrent>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    time: Option<String>,
    temperature_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    precipitation: Option<f64>,
    weather_code: Option<i32>,
    wind_speed_10m: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct WikipediaResponse {
    query: Option<WikipediaQuery>,
}

#[derive(Debug, Deserialize)]
struct WikipediaQuery {
    #[serde(default)]
    geosearch: Vec<WikipediaHit>,
}

#[derive(Debug, Deserialize)]
struct WikipediaHit {
    pageid: u64,
    title: String,
    lat: f64,
    lon: f64,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn is_current_location(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "current" | "current_location" | "my location" | "当前位置" | "我的位置"
    )
}

fn location_plan(
    near: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<LocationPlan, String> {
    match (latitude, longitude) {
        (Some(lat), Some(lon)) => {
            if !lat.is_finite() || !(-90.0..=90.0).contains(&lat) {
                return Err("latitude must be a finite value between -90 and 90".into());
            }
            if !lon.is_finite() || !(-180.0..=180.0).contains(&lon) {
                return Err("longitude must be a finite value between -180 and 180".into());
            }
            let supplied_label = near
                .map(str::trim)
                .filter(|value| !value.is_empty() && !is_current_location(value))
                .map(str::to_string);
            Ok(LocationPlan::Coordinates(DiscoveryCenter {
                label: supplied_label.unwrap_or_else(|| "Current location".into()),
                latitude: lat,
                longitude: lon,
                source: "provided_coordinates".into(),
                label_source: None,
            }))
        }
        (Some(_), None) | (None, Some(_)) => {
            Err("latitude and longitude must be supplied together".into())
        }
        (None, None) => {
            let value = near.unwrap_or_default().trim();
            if value.is_empty() || is_current_location(value) {
                Ok(LocationPlan::Missing(
                    "A current location was requested, but no coordinates were supplied. Ask for location permission or provide a city, address, or neighborhood; timezone is not a location.".into(),
                ))
            } else {
                Ok(LocationPlan::Geocode(value.to_string()))
            }
        }
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

async fn timed_with_timeout<T, F>(
    source: &'static str,
    timeout: Duration,
    future: F,
) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "{source} timed out after {} seconds",
            timeout.as_secs()
        )),
    }
}

async fn response_json<T: DeserializeOwned>(
    source: &str,
    request: reqwest::RequestBuilder,
) -> Result<T, String> {
    let mut response = request
        .send()
        .await
        .map_err(|error| format!("{source} request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{source} returned HTTP {status}"));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_SOURCE_RESPONSE_BYTES as u64)
    {
        return Err(format!(
            "{source} response exceeded the {} byte limit",
            MAX_SOURCE_RESPONSE_BYTES
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("{source} response read failed: {error}"))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_SOURCE_RESPONSE_BYTES {
            return Err(format!(
                "{source} response exceeded the {} byte limit",
                MAX_SOURCE_RESPONSE_BYTES
            ));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice::<T>(&body)
        .map_err(|error| format!("{source} returned invalid JSON: {error}"))
}

fn chinese_digit(character: char) -> Option<u64> {
    match character {
        '零' | '〇' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn is_chinese_number_character(character: char) -> bool {
    chinese_digit(character).is_some() || matches!(character, '十' | '百' | '千' | '万')
}

fn parse_chinese_number(value: &str) -> Option<u64> {
    if value.is_empty() || !value.chars().all(is_chinese_number_character) {
        return None;
    }
    if !value
        .chars()
        .any(|character| matches!(character, '十' | '百' | '千' | '万'))
    {
        return value.chars().try_fold(0_u64, |number, character| {
            number
                .checked_mul(10)?
                .checked_add(chinese_digit(character)?)
        });
    }

    let mut total = 0_u64;
    let mut section = 0_u64;
    let mut number = 0_u64;
    for character in value.chars() {
        if let Some(digit) = chinese_digit(character) {
            number = digit;
            continue;
        }
        let unit = match character {
            '十' => 10,
            '百' => 100,
            '千' => 1_000,
            '万' => 10_000,
            _ => return None,
        };
        if unit == 10_000 {
            section = section.checked_add(number)?;
            total = total.checked_add(section.checked_mul(unit)?)?;
            section = 0;
        } else {
            section = section.checked_add(number.max(1).checked_mul(unit)?)?;
        }
        number = 0;
    }
    total.checked_add(section)?.checked_add(number)
}

fn normalize_house_number(value: &str) -> Option<String> {
    let value = value
        .trim()
        .strip_suffix('号')
        .unwrap_or(value.trim())
        .trim();
    if let Some(number) = parse_chinese_number(value) {
        return Some(number.to_string());
    }

    let mut normalized = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_uppercase());
        } else if matches!(character, '-' | '之') {
            normalized.push('-');
        } else if character == '/' {
            normalized.push('/');
        } else if !character.is_whitespace() {
            return None;
        }
    }
    (!normalized.is_empty()
        && normalized
            .chars()
            .any(|character| character.is_ascii_digit()))
    .then_some(normalized)
}

#[derive(Debug)]
struct AddressToken<'a> {
    value: &'a str,
    start: usize,
    end: usize,
}

fn ascii_address_tokens(value: &str) -> Vec<AddressToken<'_>> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, character) in value.char_indices() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '/' | '之') {
            start.get_or_insert(index);
        } else if let Some(token_start) = start.take() {
            tokens.push(AddressToken {
                value: &value[token_start..index],
                start: token_start,
                end: index,
            });
        }
    }
    if let Some(token_start) = start {
        tokens.push(AddressToken {
            value: &value[token_start..],
            start: token_start,
            end: value.len(),
        });
    }
    tokens
}

fn is_ordinal_street_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["st", "nd", "rd", "th"].iter().any(|suffix| {
        lower
            .strip_suffix(suffix)
            .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
    })
}

fn requested_house_number(query: &str) -> Option<String> {
    let tokens = ascii_address_tokens(query);
    let numeric = |token: &AddressToken<'_>| {
        token
            .value
            .chars()
            .any(|character| character.is_ascii_digit())
            && !is_ordinal_street_token(token.value)
    };
    for marker in ["大道", "胡同", "路", "街", "巷", "弄"] {
        for (marker_start, _) in query.match_indices(marker) {
            let marker_end = marker_start + marker.len();
            if let Some(token) = tokens.iter().find(|token| {
                numeric(token)
                    && token.start >= marker_end
                    && query[marker_end..token.start].trim().is_empty()
                    && !query[token.end..].trim_start().starts_with("号线")
            }) {
                return normalize_house_number(token.value);
            }
        }
    }

    let lower_tokens: Vec<String> = tokens
        .iter()
        .map(|token| token.value.to_ascii_lowercase())
        .collect();
    let street_designators = [
        "street",
        "st",
        "road",
        "rd",
        "avenue",
        "ave",
        "boulevard",
        "blvd",
        "lane",
        "ln",
        "drive",
        "dr",
        "court",
        "ct",
        "place",
        "pl",
        "parkway",
        "pkwy",
        "terrace",
        "ter",
        "circle",
        "cir",
    ];
    if let Some(street_index) = lower_tokens
        .iter()
        .position(|token| street_designators.contains(&token.as_str()))
    {
        if tokens.first().is_some_and(numeric) {
            return normalize_house_number(tokens[0].value);
        }
        if let Some(token) = tokens.get(street_index + 1).filter(|token| numeric(token)) {
            return normalize_house_number(token.value);
        }
    }

    let characters: Vec<(usize, char)> = query.char_indices().collect();
    let mut explicit_numbers = Vec::new();
    for (position, (number_end, character)) in characters.iter().enumerate() {
        if *character != '号' {
            continue;
        }
        let suffix_start = characters
            .get(position + 1)
            .map(|(index, _)| *index)
            .unwrap_or(query.len());
        let suffix = query[suffix_start..].trim_start();
        if ["线", "出口", "航站楼", "航站", "站台", "口", "门"]
            .iter()
            .any(|excluded| suffix.starts_with(excluded))
        {
            continue;
        }
        let mut start = position;
        while start > 0 && {
            let previous = characters[start - 1].1;
            previous.is_ascii_alphanumeric()
                || is_chinese_number_character(previous)
                || matches!(previous, '-' | '/' | '之')
        } {
            start -= 1;
        }
        if start < position {
            let start_byte = characters[start].0;
            if let Some(number) = normalize_house_number(&query[start_byte..*number_end]) {
                explicit_numbers.push(number);
            }
        }
    }
    explicit_numbers.sort_unstable();
    explicit_numbers.dedup();
    (explicit_numbers.len() == 1).then(|| explicit_numbers.remove(0))
}

fn has_house_number_intent(query: &str) -> bool {
    requested_house_number(query).is_some()
}

fn house_number_matches(query: &str, candidate: &str) -> bool {
    requested_house_number(query)
        .zip(normalize_house_number(candidate))
        .is_some_and(|(requested, returned)| requested == returned)
}

async fn forward_geocode(query: &str, language: &str) -> Result<Option<DiscoveryCenter>, String> {
    let hits: Vec<NominatimHit> = response_json(
        "Nominatim",
        HTTP.get(NOMINATIM_SEARCH_URL).query(&[
            ("q", query),
            ("format", "jsonv2"),
            ("limit", "1"),
            ("addressdetails", "1"),
            ("accept-language", language),
        ]),
    )
    .await?;
    let Some(hit) = hits.into_iter().next() else {
        return Ok(None);
    };
    if has_house_number_intent(query)
        && !hit
            .address
            .as_ref()
            .and_then(|address| address.house_number.as_deref())
            .is_some_and(|house_number| house_number_matches(query, house_number))
    {
        // A road or locality centroid is not an exact address result. Treat it
        // as empty so the strict address fallback can run instead of silently
        // presenting a coarse coordinate as the requested house number.
        return Ok(None);
    }
    let latitude = hit
        .lat
        .parse::<f64>()
        .map_err(|_| "Nominatim returned an invalid latitude".to_string())?;
    let longitude = hit
        .lon
        .parse::<f64>()
        .map_err(|_| "Nominatim returned an invalid longitude".to_string())?;
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
    {
        return Err("Nominatim returned out-of-range coordinates".into());
    }
    Ok(Some(DiscoveryCenter {
        label: hit.display_name,
        latitude,
        longitude,
        source: "nominatim".into(),
        label_source: Some("nominatim".into()),
    }))
}

fn arcgis_match_from_response(
    query: &str,
    response: ArcgisGeocodeResponse,
) -> Result<Option<ArcgisGeocodeMatch>, String> {
    if let Some(error) = response.error {
        let code = error
            .code
            .map(|value| format!(" code {value}"))
            .unwrap_or_default();
        let message = error
            .message
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "unknown service error".into());
        let details = error
            .details
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "ArcGIS World Geocoding returned{code}: {message}{}",
            if details.is_empty() {
                String::new()
            } else {
                format!(" ({details})")
            }
        ));
    }

    let requires_house_precision = has_house_number_intent(query);
    let best = response
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            let address_type = candidate
                .attributes
                .address_type
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string();
            let precise_address_type = matches!(
                address_type.to_ascii_lowercase().as_str(),
                "subaddress" | "pointaddress" | "streetaddress"
            );
            let label = candidate
                .attributes
                .match_address
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| candidate.address.trim())
                .to_string();
            let requested_number_matches = candidate
                .attributes
                .address_number
                .as_deref()
                .is_some_and(|number| house_number_matches(query, number));
            if !candidate.score.is_finite()
                || candidate.score < ARCGIS_MIN_MATCH_SCORE
                || label.is_empty()
                || (requires_house_precision
                    && (!precise_address_type || !requested_number_matches))
                || !candidate.location.y.is_finite()
                || !candidate.location.x.is_finite()
                || !(-90.0..=90.0).contains(&candidate.location.y)
                || !(-180.0..=180.0).contains(&candidate.location.x)
            {
                return None;
            }
            Some((candidate, label, address_type))
        })
        .max_by(|(left, _, _), (right, _, _)| left.score.total_cmp(&right.score));

    Ok(
        best.map(|(candidate, label, address_type)| ArcgisGeocodeMatch {
            center: DiscoveryCenter {
                label,
                latitude: candidate.location.y,
                longitude: candidate.location.x,
                source: "arcgis_world_geocoding".into(),
                label_source: Some("arcgis_world_geocoding".into()),
            },
            score: candidate.score,
            address_type,
        }),
    )
}

async fn forward_geocode_arcgis(query: &str) -> Result<Option<ArcgisGeocodeMatch>, String> {
    let response: ArcgisGeocodeResponse = response_json(
        "ArcGIS World Geocoding",
        HTTP.get(ARCGIS_GEOCODE_URL).query(&[
            ("SingleLine", query),
            ("f", "json"),
            ("forStorage", "false"),
            ("maxLocations", "5"),
            ("outSR", "4326"),
            ("outFields", "Match_addr,Addr_type,AddNum"),
        ]),
    )
    .await?;
    arcgis_match_from_response(query, response)
}

async fn resolve_geocoded_center(
    query: &str,
    language: &str,
    statuses: &mut Vec<SourceStatus>,
) -> Option<DiscoveryCenter> {
    match timed("Nominatim", forward_geocode(query, language)).await {
        Ok(Some(center)) => {
            statuses.push(status(
                "nominatim",
                SourceState::Success,
                1,
                "Place text resolved to a coordinate.",
            ));
            statuses.push(status(
                "arcgis_world_geocoding",
                SourceState::Skipped,
                0,
                "Nominatim returned an acceptable result, so the ArcGIS fallback was not requested.",
            ));
            return Some(center);
        }
        Ok(None) => statuses.push(status(
            "nominatim",
            SourceState::Empty,
            0,
            "No result with the requested place/address precision was returned.",
        )),
        Err(error) => statuses.push(status("nominatim", SourceState::Failed, 0, error)),
    }

    match timed("ArcGIS World Geocoding", forward_geocode_arcgis(query)).await {
        Ok(Some(result)) => {
            statuses.push(status(
                "arcgis_world_geocoding",
                SourceState::Success,
                1,
                format!(
                    "Fallback accepted one {} candidate with provider match score {:.1}; the request used forStorage=false.",
                    result.address_type, result.score
                ),
            ));
            Some(result.center)
        }
        Ok(None) => {
            statuses.push(status(
                "arcgis_world_geocoding",
                SourceState::Empty,
                0,
                "No candidate met the confidence and requested address-precision requirements.",
            ));
            None
        }
        Err(error) => {
            statuses.push(status(
                "arcgis_world_geocoding",
                SourceState::Failed,
                0,
                error,
            ));
            None
        }
    }
}

async fn reverse_geocode(
    latitude: f64,
    longitude: f64,
    language: &str,
) -> Result<Option<String>, String> {
    let lat = format!("{latitude:.7}");
    let lon = format!("{longitude:.7}");
    let result: NominatimReverse = response_json(
        "Nominatim",
        HTTP.get(NOMINATIM_REVERSE_URL).query(&[
            ("lat", lat.as_str()),
            ("lon", lon.as_str()),
            ("format", "jsonv2"),
            ("accept-language", language),
        ]),
    )
    .await?;
    Ok(result
        .display_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn contains_any(query: &str, needles: &[&str]) -> bool {
    let query = query.to_lowercase();
    let words = query
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    needles.iter().any(|needle| {
        if needle.is_ascii() {
            let needle_words = needle
                .split_ascii_whitespace()
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>();
            !needle_words.is_empty()
                && words.windows(needle_words.len()).any(|window| {
                    window.iter().zip(&needle_words).all(|(word, needle)| {
                        *word == needle.as_str() || ascii_plural_matches(word, needle)
                    })
                })
        } else {
            query.contains(needle)
        }
    })
}

fn ascii_plural_matches(word: &str, singular: &str) -> bool {
    if singular.len() < 3 {
        return false;
    }
    if let Some(stem) = singular.strip_suffix('y') {
        return word == format!("{stem}ies");
    }
    if singular.ends_with('s')
        || singular.ends_with('x')
        || singular.ends_with('z')
        || singular.ends_with("ch")
        || singular.ends_with("sh")
    {
        word == format!("{singular}es")
    } else {
        word == format!("{singular}s")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PoiIntent {
    Bakery,
    Food,
    Lodging,
    FamilyActivity,
    Grocery,
    Healthcare,
    Entertainment,
    Nightlife,
    Fitness,
    Shopping,
    Sightseeing,
    Transport,
    PersonalServices,
    General,
}

fn poi_intent(query: &str) -> PoiIntent {
    if contains_any(
        query,
        &[
            "bakery",
            "pastry",
            "bread",
            "cake shop",
            "donut",
            "doughnut",
            "面包",
            "烘焙",
            "蛋糕店",
        ],
    ) {
        PoiIntent::Bakery
    } else if contains_any(
        query,
        &[
            "grocery",
            "supermarket",
            "convenience store",
            "farmers market",
            "food market",
            "买菜",
            "超市",
            "便利店",
            "菜市场",
            "生鲜",
        ],
    ) {
        PoiIntent::Grocery
    } else if contains_any(
        query,
        &[
            "food",
            "eat",
            "eating",
            "eatery",
            "restaurant",
            "cafe",
            "coffee",
            "breakfast",
            "lunch",
            "dinner",
            "sushi",
            "pizza",
            "burger",
            "vegan",
            "halal",
            "吃",
            "餐",
            "饭",
            "菜",
            "美食",
            "咖啡",
            "小吃",
            "火锅",
            "烧烤",
            "甜品",
        ],
    ) {
        PoiIntent::Food
    } else if contains_any(
        query,
        &[
            "hotel",
            "hostel",
            "motel",
            "lodging",
            "accommodation",
            "stay",
            "酒店",
            "旅馆",
            "住宿",
            "民宿",
            "青旅",
        ],
    ) {
        PoiIntent::Lodging
    } else if contains_any(
        query,
        &[
            "family activity",
            "family fun",
            "kids activity",
            "children activity",
            "playground",
            "aquarium",
            "zoo",
            "theme park",
            "water park",
            "amusement arcade",
            "bowling",
            "mini golf",
            "things to do",
            "activity",
            "亲子",
            "遛娃",
            "儿童",
            "小孩",
            "游乐场",
            "活动",
        ],
    ) {
        PoiIntent::FamilyActivity
    } else if contains_any(
        query,
        &[
            "pharmacy",
            "hospital",
            "clinic",
            "doctor",
            "dentist",
            "urgent care",
            "medical",
            "药店",
            "医院",
            "诊所",
            "医生",
            "牙医",
            "急诊",
        ],
    ) {
        PoiIntent::Healthcare
    } else if contains_any(
        query,
        &[
            "cinema",
            "movie",
            "theater",
            "theatre",
            "concert",
            "live music",
            "art gallery",
            "escape room",
            "电影院",
            "剧院",
            "演出",
            "音乐厅",
            "画廊",
            "密室",
        ],
    ) {
        PoiIntent::Entertainment
    } else if contains_any(
        query,
        &[
            "bar",
            "pub",
            "nightclub",
            "nightlife",
            "cocktail",
            "酒吧",
            "夜店",
            "夜生活",
        ],
    ) {
        PoiIntent::Nightlife
    } else if contains_any(
        query,
        &[
            "gym",
            "fitness",
            "workout",
            "swimming pool",
            "sports centre",
            "yoga",
            "健身",
            "游泳馆",
            "运动中心",
            "瑜伽",
        ],
    ) {
        PoiIntent::Fitness
    } else if contains_any(
        query,
        &[
            "shopping", "shop", "store", "mall", "boutique", "购物", "商场", "店铺",
        ],
    ) {
        PoiIntent::Shopping
    } else if contains_any(
        query,
        &[
            "travel",
            "tourism",
            "attraction",
            "museum",
            "landmark",
            "sightseeing",
            "sight",
            "park",
            "garden",
            "historic",
            "旅游",
            "景点",
            "博物馆",
            "公园",
            "地标",
            "古迹",
            "花园",
        ],
    ) {
        PoiIntent::Sightseeing
    } else if contains_any(
        query,
        &[
            "parking",
            "gas station",
            "fuel station",
            "charging station",
            "bus stop",
            "train station",
            "subway station",
            "transit",
            "停车",
            "加油站",
            "充电站",
            "公交站",
            "地铁站",
            "火车站",
        ],
    ) {
        PoiIntent::Transport
    } else if contains_any(
        query,
        &[
            "bank",
            "atm",
            "post office",
            "laundry",
            "dry cleaning",
            "hairdresser",
            "barber",
            "beauty salon",
            "spa",
            "银行",
            "取款机",
            "邮局",
            "洗衣",
            "理发",
            "美容",
            "水疗",
        ],
    ) {
        PoiIntent::PersonalServices
    } else {
        PoiIntent::General
    }
}

fn overpass_query(query: &str, center: &DiscoveryCenter, radius_m: u32) -> String {
    let around = format!(
        "around:{radius_m},{:.7},{:.7}",
        center.latitude, center.longitude
    );
    let selectors = match poi_intent(query) {
        PoiIntent::Bakery => vec![format!(
            "nwr({around})[\"shop\"~\"^(bakery|pastry|confectionery)$\"];"
        )],
        PoiIntent::Food => vec![format!(
            "nwr({around})[\"amenity\"~\"^(restaurant|cafe|fast_food|food_court|ice_cream|biergarten)$\"];"
        ), format!(
            "nwr({around})[\"shop\"~\"^(bakery|pastry|confectionery|deli|tea|coffee)$\"];"
        )],
        PoiIntent::Lodging => vec![format!(
            "nwr({around})[\"tourism\"~\"^(hotel|hostel|motel|guest_house|apartment|camp_site)$\"];"
        )],
        PoiIntent::FamilyActivity => vec![
            format!("nwr({around})[\"leisure\"~\"^(playground|water_park|miniature_golf|amusement_arcade|bowling_alley|escape_game|sports_centre|swimming_pool)$\"];"),
            format!("nwr({around})[\"tourism\"~\"^(zoo|aquarium|theme_park|museum|attraction)$\"];"),
            format!("nwr({around})[\"amenity\"~\"^(cinema|theatre|arts_centre|community_centre)$\"];"),
        ],
        PoiIntent::Grocery => vec![
            format!("nwr({around})[\"shop\"~\"^(supermarket|convenience|greengrocer|farm)$\"];"),
            format!("nwr({around})[\"amenity\"=\"marketplace\"];"),
        ],
        PoiIntent::Healthcare => vec![
            format!("nwr({around})[\"amenity\"~\"^(pharmacy|hospital|clinic|doctors|dentist)$\"];"),
            format!("nwr({around})[\"healthcare\"~\"^(hospital|clinic|doctor|dentist|pharmacy)$\"];"),
        ],
        PoiIntent::Entertainment => vec![
            format!("nwr({around})[\"amenity\"~\"^(cinema|theatre|arts_centre|music_venue)$\"];"),
            format!("nwr({around})[\"tourism\"=\"gallery\"];"),
            format!("nwr({around})[\"leisure\"~\"^(escape_game|dance)$\"];"),
        ],
        PoiIntent::Nightlife => vec![format!(
            "nwr({around})[\"amenity\"~\"^(bar|pub|nightclub)$\"];"
        )],
        PoiIntent::Fitness => vec![format!(
            "nwr({around})[\"leisure\"~\"^(fitness_centre|sports_centre|swimming_pool|fitness_station|pitch)$\"];"
        )],
        PoiIntent::Shopping => vec![format!("nwr({around})[\"shop\"];")],
        PoiIntent::Sightseeing => vec![
            format!("nwr({around})[\"tourism\"~\"^(attraction|museum|gallery|viewpoint|zoo|theme_park)$\"];") ,
            format!("nwr({around})[\"historic\"];") ,
            format!("nwr({around})[\"leisure\"~\"^(park|nature_reserve|garden)$\"];") ,
        ],
        PoiIntent::Transport => vec![
            format!("nwr({around})[\"amenity\"~\"^(parking|fuel|charging_station|bus_station)$\"];"),
            format!("nwr({around})[\"railway\"~\"^(station|halt|subway_entrance|tram_stop)$\"];"),
            format!("nwr({around})[\"highway\"=\"bus_stop\"];"),
        ],
        PoiIntent::PersonalServices => vec![
            format!("nwr({around})[\"amenity\"~\"^(bank|atm|post_office|parcel_locker)$\"];"),
            format!("nwr({around})[\"shop\"~\"^(laundry|dry_cleaning|hairdresser|beauty)$\"];"),
            format!("nwr({around})[\"leisure\"=\"spa\"];"),
        ],
        PoiIntent::General => vec![
            format!("nwr({around})[\"amenity\"~\"^(restaurant|cafe|fast_food|food_court|ice_cream)$\"];") ,
            format!("nwr({around})[\"tourism\"~\"^(attraction|museum|gallery|viewpoint|hotel|hostel|guest_house)$\"];") ,
            format!("nwr({around})[\"leisure\"~\"^(park|nature_reserve|garden)$\"];") ,
            format!("nwr({around})[\"historic\"];") ,
        ],
    };
    // Overpass does not order `out` rows by distance or text relevance. A row
    // count here would create an arbitrary sample that cannot be ranked
    // truthfully, so request the complete matching set under an explicit server
    // memory cap. The client also enforces MAX_SOURCE_RESPONSE_BYTES.
    format!(
        "[out:json][timeout:15][maxsize:{OVERPASS_MAXSIZE_BYTES}];({});out center;",
        selectors.join("")
    )
}

fn tag<'a>(tags: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    tags.get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn localized_name(tags: &HashMap<String, String>, language: &str) -> Option<String> {
    tag(tags, &format!("name:{language}"))
        .or_else(|| tag(tags, "name"))
        .or_else(|| tag(tags, "brand"))
        .map(str::to_string)
}

fn category_from_tags(tags: &HashMap<String, String>) -> Option<String> {
    [
        "amenity",
        "shop",
        "tourism",
        "leisure",
        "healthcare",
        "railway",
        "highway",
        "historic",
        "natural",
    ]
    .into_iter()
    .find_map(|key| tag(tags, key).map(|value| format!("{key}:{value}")))
}

fn address_from_tags(tags: &HashMap<String, String>) -> Option<String> {
    if let Some(full) = tag(tags, "addr:full") {
        return Some(full.to_string());
    }
    let street = tag(tags, "addr:street");
    let number = tag(tags, "addr:housenumber");
    let locality = tag(tags, "addr:city")
        .or_else(|| tag(tags, "addr:town"))
        .or_else(|| tag(tags, "addr:suburb"));
    let mut parts = Vec::new();
    if street.is_some() || number.is_some() {
        parts.push(
            [number, street]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    if let Some(locality) = locality {
        parts.push(locality.to_string());
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn overpass_element_to_place(
    element: OverpassElement,
    center: &DiscoveryCenter,
    language: &str,
    radius_m: u32,
) -> Option<DiscoveryPlace> {
    let name = localized_name(&element.tags, language)?;
    let coordinates = match (element.lat, element.lon, element.center) {
        (Some(lat), Some(lon), _) => Some((lat, lon)),
        (_, _, Some(center)) => Some((center.lat, center.lon)),
        _ => None,
    }?;
    if !coordinates.0.is_finite()
        || !coordinates.1.is_finite()
        || !(-90.0..=90.0).contains(&coordinates.0)
        || !(-180.0..=180.0).contains(&coordinates.1)
    {
        return None;
    }
    let source_url = format!(
        "https://www.openstreetmap.org/{}/{}",
        element.element_type, element.id
    );
    // For ways and relations Overpass's `center` is the bounding-box center,
    // not the closest point on the feature. Keep only reported coordinates
    // inside the requested radius so distance_m and radius_m remain comparable.
    let distance_m = haversine_m(
        center.latitude,
        center.longitude,
        coordinates.0,
        coordinates.1,
    );
    if distance_m > f64::from(radius_m) {
        return None;
    }
    Some(DiscoveryPlace {
        id: format!("osm:{}/{}", element.element_type, element.id),
        name,
        category: category_from_tags(&element.tags),
        latitude: coordinates.0,
        longitude: coordinates.1,
        distance_m: distance_m.round().max(0.0) as u32,
        source: "openstreetmap".into(),
        source_url: Some(source_url),
        address: address_from_tags(&element.tags),
        cuisine: tag(&element.tags, "cuisine").map(str::to_string),
        opening_hours: tag(&element.tags, "opening_hours").map(str::to_string),
        rating: None,
        price: None,
        open_now: None,
    })
}

fn overpass_data_as_of(response: &OverpassResponse) -> Option<String> {
    response
        .osm3s
        .as_ref()
        .and_then(|metadata| metadata.timestamp_osm_base.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn overpass_result_or_remember_empty(
    empty_result: &mut Option<OverpassPlacesResult>,
    partial_empty_result: &mut Option<OverpassPlacesResult>,
    result: OverpassPlacesResult,
) -> Option<OverpassPlacesResult> {
    if !result.places.is_empty() {
        return Some(result);
    }
    if result.warning.is_none() {
        if empty_result.is_none() {
            *empty_result = Some(result);
        }
    } else if partial_empty_result.is_none() {
        *partial_empty_result = Some(result);
    }
    None
}

async fn fetch_overpass_places(
    query: &str,
    center: &DiscoveryCenter,
    radius_m: u32,
    language: &str,
) -> Result<OverpassPlacesResult, String> {
    let statement = overpass_query(query, center, radius_m);
    let mut errors = Vec::new();
    let mut empty_result: Option<OverpassPlacesResult> = None;
    let mut partial_empty_result: Option<OverpassPlacesResult> = None;

    for (endpoint_name, endpoint_url) in OVERPASS_ENDPOINTS {
        let response: OverpassResponse = match response_json(
            "Overpass",
            HTTP.post(endpoint_url)
                .timeout(OVERPASS_REQUEST_TIMEOUT)
                .form(&[("data", statement.as_str())]),
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                errors.push(format!("{endpoint_name}: {error}"));
                continue;
            }
        };
        let data_as_of = overpass_data_as_of(&response);
        let completion = complete_overpass_elements(response);
        let mut places = completion
            .elements
            .into_iter()
            .filter_map(|element| overpass_element_to_place(element, center, language, radius_m))
            .collect::<Vec<_>>();
        // Keep the full bounded Overpass candidate set. Text relevance is applied
        // together with distance only after all sources have returned; truncating
        // here would discard a slightly farther exact cuisine/name match before it
        // ever receives a relevance score.
        places.sort_by_key(|place| place.distance_m);
        let result = OverpassPlacesResult {
            places,
            data_as_of,
            warning: completion.warning,
            endpoint: endpoint_name.to_string(),
        };
        if let Some(result) =
            overpass_result_or_remember_empty(&mut empty_result, &mut partial_empty_result, result)
        {
            return Ok(result);
        }
    }

    if let Some(result) = empty_result {
        return Ok(result);
    }
    if let Some(result) = partial_empty_result {
        return Ok(result);
    }

    Err(if errors.is_empty() {
        "Overpass returned no usable response".into()
    } else {
        errors.join("; ")
    })
}

fn complete_overpass_elements(response: OverpassResponse) -> OverpassCompletion {
    let warning = response
        .remark
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    OverpassCompletion {
        elements: response.elements,
        warning,
    }
}

fn weather_condition(code: i32) -> &'static str {
    match code {
        0 => "clear sky",
        1 | 2 => "mainly clear to partly cloudy",
        3 => "overcast",
        45 | 48 => "fog",
        51 | 53 | 55 | 56 | 57 => "drizzle",
        61 | 63 | 65 | 66 | 67 => "rain",
        71 | 73 | 75 | 77 => "snow",
        80..=82 => "rain showers",
        85 | 86 => "snow showers",
        95 | 96 | 99 => "thunderstorm",
        _ => "unknown WMO weather code",
    }
}

async fn fetch_weather(center: &DiscoveryCenter) -> Result<Option<DiscoveryWeather>, String> {
    let latitude = format!("{:.7}", center.latitude);
    let longitude = format!("{:.7}", center.longitude);
    let response: OpenMeteoResponse = response_json(
        "Open-Meteo",
        HTTP.get(OPEN_METEO_URL).query(&[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            (
                "current",
                "temperature_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m",
            ),
            ("temperature_unit", "celsius"),
            ("wind_speed_unit", "kmh"),
            ("precipitation_unit", "mm"),
            ("timezone", "auto"),
        ]),
    )
    .await?;
    let Some(current) = response.current else {
        return Ok(None);
    };
    Ok(Some(DiscoveryWeather {
        temperature_c: current.temperature_2m,
        apparent_temperature_c: current.apparent_temperature,
        precipitation_mm: current.precipitation,
        wind_speed_kmh: current.wind_speed_10m,
        weather_code: current.weather_code,
        condition: current
            .weather_code
            .map(weather_condition)
            .map(str::to_string),
        observed_at: current.time,
        timezone: response.timezone,
        source: "open_meteo".into(),
        source_url: "https://open-meteo.com/".into(),
    }))
}

fn wikipedia_language(language: Option<&str>, query: &str) -> String {
    let candidate = language
        .and_then(|value| value.split(['-', '_']).next())
        .map(str::trim)
        .filter(|value| (2..=3).contains(&value.len()))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_alphabetic()))
        .map(str::to_lowercase);
    candidate.unwrap_or_else(|| {
        if query
            .chars()
            .any(|character| ('\u{4e00}'..='\u{9fff}').contains(&character))
        {
            "zh".into()
        } else {
            "en".into()
        }
    })
}

// ── 百度地图 POI 搜索（scope=2 含评分/人均/营业时间/评论数） ─────────────────
async fn fetch_baidu_places(
    query: &str,
    center: &DiscoveryCenter,
    radius_m: u32,
    limit: u32,
) -> Result<Vec<DiscoveryPlace>, String> {
    let ak = match baidu_ak() {
        Some(k) => k,
        None => return Ok(Vec::new()),
    };
    let url = format!(
        "{}?query={}&location={},{}&radius={}&scope=2&page_size={}&output=json&ak={}",
        BAIDU_PLACE_URL,
        percent_encode_query(query),
        center.latitude,
        center.longitude,
        radius_m,
        limit.min(20),
        ak,
    );
    let resp: serde_json::Value = HTTP
        .get(&url)
        .timeout(SOURCE_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("baidu: {e}"))?
        .json()
        .await
        .map_err(|e| format!("baidu json: {e}"))?;
    if resp.get("status").and_then(|v| v.as_i64()) != Some(0) {
        let msg = resp
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("baidu api: {msg}"));
    }
    let results = resp
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut places = Vec::with_capacity(results.len());
    for r in &results {
        let name = r.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let loc = r.get("location");
        let lat = loc
            .and_then(|l| l.get("lat"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let lng = loc
            .and_then(|l| l.get("lng"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let detail = r.get("detail_info");
        let rating = detail.and_then(|d| d.get("overall_rating")).and_then(|v| {
            v.as_str().or_else(|| v.as_f64().map(|_| "")).and_then(|s| {
                if s.is_empty() {
                    v.as_f64()
                } else {
                    s.parse::<f64>().ok()
                }
            })
        });
        let price_str = detail
            .and_then(|d| d.get("price"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| format!("¥{s}/人"));
        let hours = detail
            .and_then(|d| d.get("shop_hours"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let address = r
            .get("address")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let phone = r
            .get("telephone")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let comment_num = detail.and_then(|d| d.get("comment_num")).and_then(|v| {
            v.as_str().or_else(|| v.as_u64().map(|_| "")).and_then(|s| {
                if s.is_empty() {
                    v.as_u64().map(|n| n.to_string())
                } else {
                    Some(s.to_string())
                }
            })
        });
        let tag = detail
            .and_then(|d| d.get("tag"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let dist_m = haversine_m(center.latitude, center.longitude, lat, lng) as u32;
        let mut src_parts = vec!["baidu_map".to_string()];
        if let Some(n) = &comment_num {
            src_parts.push(format!("{n}条评论"));
        }
        if let Some(p) = phone {
            src_parts.push(format!("tel:{p}"));
        }
        places.push(DiscoveryPlace {
            id: r
                .get("uid")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            name: name.to_string(),
            category: tag.clone(),
            latitude: lat,
            longitude: lng,
            distance_m: dist_m,
            source: src_parts.join(" | "),
            source_url: r
                .get("detail_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            address,
            cuisine: tag,
            opening_hours: hours,
            rating,
            price: price_str,
            open_now: None,
        });
    }
    Ok(places)
}

// ── 高德地图 POI 搜索（extensions=all 含评分/人均） ──────────────────────────
async fn fetch_amap_places(
    query: &str,
    center: &DiscoveryCenter,
    radius_m: u32,
    limit: u32,
) -> Result<Vec<DiscoveryPlace>, String> {
    let key = match amap_key() {
        Some(k) => k,
        None => return Ok(Vec::new()),
    };
    let url = format!(
        "{}?keywords={}&location={},{}&radius={}&offset={}&extensions=all&output=json&key={}",
        AMAP_PLACE_URL,
        percent_encode_query(query),
        center.longitude,
        center.latitude,
        radius_m,
        limit.min(25),
        key,
    );
    let resp: serde_json::Value = HTTP
        .get(&url)
        .timeout(SOURCE_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("amap: {e}"))?
        .json()
        .await
        .map_err(|e| format!("amap json: {e}"))?;
    if resp.get("status").and_then(|v| v.as_str()) != Some("1") {
        let msg = resp
            .get("info")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("amap api: {msg}"));
    }
    let pois = resp
        .get("pois")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut places = Vec::with_capacity(pois.len());
    for p in &pois {
        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let loc_str = p.get("location").and_then(|v| v.as_str()).unwrap_or("0,0");
        let mut parts = loc_str.split(',');
        let lng: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let lat: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let biz = p.get("biz_ext");
        let rating = biz
            .and_then(|b| b.get("rating"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|&r| r > 0.0);
        let cost = biz
            .and_then(|b| b.get("cost"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "[]")
            .map(|s| format!("¥{s}/人"));
        let category = p
            .get("type")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let address = p
            .get("address")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "[]")
            .map(|s| s.to_string());
        let phone = p
            .get("tel")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "[]");
        let dist_m = haversine_m(center.latitude, center.longitude, lat, lng) as u32;
        let mut src = "amap".to_string();
        if let Some(ph) = phone {
            src.push_str(&format!(" | tel:{ph}"));
        }
        places.push(DiscoveryPlace {
            id: p
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            name: name.to_string(),
            category: category.clone(),
            latitude: lat,
            longitude: lng,
            distance_m: dist_m,
            source: src,
            source_url: None,
            address,
            cuisine: category,
            opening_hours: None,
            price: cost,
            rating,
            open_now: None,
        });
    }
    Ok(places)
}

async fn fetch_wikipedia_places(
    center: &DiscoveryCenter,
    radius_m: u32,
    limit: u32,
    language: &str,
) -> Result<Vec<DiscoveryPlace>, String> {
    let url = format!("https://{language}.wikipedia.org/w/api.php");
    let coordinates = format!("{:.7}|{:.7}", center.latitude, center.longitude);
    let radius = radius_m.min(10_000).to_string();
    let limit_text = limit.to_string();
    let response: WikipediaResponse = response_json(
        "Wikipedia GeoSearch",
        HTTP.get(&url).query(&[
            ("action", "query"),
            ("list", "geosearch"),
            ("gscoord", coordinates.as_str()),
            ("gsradius", radius.as_str()),
            ("gslimit", limit_text.as_str()),
            ("format", "json"),
            ("origin", "*"),
        ]),
    )
    .await?;
    let hits = response
        .query
        .map(|query| query.geosearch)
        .unwrap_or_default();
    Ok(hits
        .into_iter()
        .filter(|hit| {
            hit.lat.is_finite()
                && hit.lon.is_finite()
                && (-90.0..=90.0).contains(&hit.lat)
                && (-180.0..=180.0).contains(&hit.lon)
        })
        .map(|hit| DiscoveryPlace {
            id: format!("wikipedia:{}:{}", language, hit.pageid),
            name: hit.title,
            category: Some("encyclopedia_article".into()),
            latitude: hit.lat,
            longitude: hit.lon,
            distance_m: haversine_m(center.latitude, center.longitude, hit.lat, hit.lon)
                .round()
                .max(0.0) as u32,
            source: "wikipedia".into(),
            source_url: Some(format!(
                "https://{language}.wikipedia.org/?curid={}",
                hit.pageid
            )),
            address: None,
            cuisine: None,
            opening_hours: None,
            rating: None,
            price: None,
            open_now: None,
        })
        .collect())
}

fn normalized_terms(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut terms = Vec::new();
    for segment in lower.split(|character: char| !character.is_alphanumeric()) {
        let characters = segment.chars().collect::<Vec<_>>();
        if characters.len() < 2 {
            continue;
        }
        if characters.iter().all(|character| character.is_ascii()) {
            terms.push(segment.to_string());
        } else {
            // CJK queries usually have no spaces. Bigrams let “附近川菜” match
            // a Chinese venue name containing “川菜” without treating the whole
            // sentence as one impossible token.
            for pair in characters.windows(2) {
                terms.push(pair.iter().collect());
            }
        }
    }
    for (needle, aliases) in [
        ("川菜", &["sichuan", "chinese"][..]),
        ("寿司", &["sushi", "japanese"][..]),
        ("日料", &["japanese"][..]),
        ("咖啡", &["cafe", "coffee"][..]),
        ("火锅", &["hot_pot", "chinese"][..]),
        ("烧烤", &["barbecue", "bbq"][..]),
        ("素食", &["vegan", "vegetarian"][..]),
        ("清真", &["halal"][..]),
        ("博物馆", &["museum"][..]),
        ("公园", &["park"][..]),
    ] {
        if lower.contains(needle) {
            terms.extend(aliases.iter().map(|alias| (*alias).to_string()));
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn relevance_score(query: &str, place: &DiscoveryPlace) -> usize {
    let terms = normalized_terms(query);
    if terms.is_empty() {
        return 0;
    }
    let haystack = format!(
        "{} {} {}",
        place.name,
        place.category.as_deref().unwrap_or_default(),
        place.cuisine.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    terms.iter().filter(|term| haystack.contains(*term)).count()
}

fn sort_and_limit_places(query: &str, places: &mut Vec<DiscoveryPlace>, limit: usize) {
    places.sort_by(|left, right| {
        relevance_score(query, right)
            .cmp(&relevance_score(query, left))
            .then_with(|| left.distance_m.cmp(&right.distance_m))
            .then_with(|| left.name.cmp(&right.name))
    });
    places.truncate(limit);
}

fn sort_and_limit_context(context: &mut Vec<DiscoveryPlace>, limit: usize) {
    context.sort_by(|left, right| {
        left.distance_m
            .cmp(&right.distance_m)
            .then_with(|| left.name.cmp(&right.name))
    });
    context.truncate(limit);
}

fn finalize_places_and_context(
    query: &str,
    mut places: Vec<DiscoveryPlace>,
    mut nearby_context: Vec<DiscoveryPlace>,
    limit: usize,
) -> (Vec<DiscoveryPlace>, Vec<DiscoveryPlace>) {
    sort_and_limit_places(query, &mut places, limit);
    sort_and_limit_context(&mut nearby_context, limit);
    (places, nearby_context)
}

fn status(
    source: &str,
    state: SourceState,
    count: usize,
    detail: impl Into<String>,
) -> SourceStatus {
    SourceStatus {
        source: source.into(),
        status: state,
        result_count: count,
        detail: detail.into(),
        data_as_of: None,
    }
}

fn status_as_of(
    source: &str,
    state: SourceState,
    count: usize,
    detail: impl Into<String>,
    data_as_of: Option<String>,
) -> SourceStatus {
    SourceStatus {
        source: source.into(),
        status: state,
        result_count: count,
        detail: detail.into(),
        data_as_of,
    }
}

fn append_spatial_skips(statuses: &mut Vec<SourceStatus>, reason: &str) {
    for source in ["overpass", "open_meteo", "wikipedia"] {
        statuses.push(status(source, SourceState::Skipped, 0, reason));
    }
}

fn base_limitations(radius_m: u32) -> Vec<String> {
    let mut limitations = vec![
        "Distances are Haversine straight-line estimates, not walking, driving, transit, traffic, or accessibility routes.".into(),
        "No queried source supplies normalized ratings or prices, so rating and price remain null.".into(),
        "OpenStreetMap opening_hours is returned only as source text; open_now remains null because this command has no live open/closed feed.".into(),
        "Activity queries return mapped venues and POIs, not a live event schedule, ticket inventory, or proof that an event is happening today.".into(),
        "OpenStreetMap and Wikipedia coverage can be incomplete or stale; verify consequential details with the venue or an official source.".into(),
        "retrieved_at is when this command finished retrieving its responses, not when any map record or venue detail was surveyed or updated.".into(),
        "A source_statuses success means that provider returned a usable response; it does not prove address accuracy, freshness, current business state, or independent corroboration.".into(),
        "Overpass data_as_of is the OSM base snapshot timestamp exposed by that response, not an on-site verification or per-venue update time; map databases are not real-time feeds.".into(),
        "ArcGIS World Geocoding is queried only as a strict fallback with forStorage=false; its returned coordinate is not averaged with or inferred from another source.".into(),
        "Wikipedia GeoSearch is returned separately in nearby_context as background; it is never ranked as a POI, recommendation, endorsement, or popularity signal.".into(),
        "Open-Meteo current conditions are provider estimates for the reported timestamp, not a guarantee at a specific venue.".into(),
        "The public Nominatim, ArcGIS World Geocoding, Overpass, Open-Meteo, and Wikipedia endpoints have independent rate limits and no application SLA; source_statuses reports each requested source separately.".into(),
    ];
    if radius_m > 10_000 {
        limitations.push(
            "Wikipedia GeoSearch is capped at 10 km even though the requested POI radius is larger."
                .into(),
        );
    }
    limitations
}

fn empty_response(
    radius_m: u32,
    limitation: String,
    statuses: Vec<SourceStatus>,
) -> LocalDiscoveryResponse {
    let mut limitations = base_limitations(radius_m);
    limitations.insert(0, limitation);
    LocalDiscoveryResponse {
        center: None,
        places: Vec::new(),
        nearby_context: Vec::new(),
        weather: None,
        source_statuses: statuses,
        limitations,
        retrieved_at: unix_now(),
        radius_m,
    }
}

/// Discover nearby POIs and contextual data without pretending that generic
/// public datasets contain live ratings, prices, routes, or open/closed state.
///
/// `near="current"` is only a location intent. The caller must obtain consent
/// and pass both coordinates; this command never infers a position from timezone
/// or IP address. Place text uses Nominatim first and a strict ArcGIS World
/// Geocoding fallback; returned coordinates are never averaged or guessed.
#[tauri::command]
pub async fn local_discovery(
    query: String,
    near: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_m: Option<u32>,
    limit: Option<u32>,
    language: Option<String>,
) -> Result<LocalDiscoveryResponse, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("local_discovery requires a non-empty query".into());
    }
    let radius_m = radius_m
        .unwrap_or(DEFAULT_RADIUS_M)
        .clamp(MIN_RADIUS_M, MAX_RADIUS_M);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let language = wikipedia_language(language.as_deref(), &query);
    let plan = location_plan(near.as_deref(), latitude, longitude)?;
    let mut statuses = Vec::with_capacity(5);

    let (mut center, reverse_needed) = match plan {
        LocationPlan::Missing(reason) => {
            statuses.push(status(
                "nominatim",
                SourceState::Skipped,
                0,
                "No place text was supplied for geocoding.",
            ));
            statuses.push(status(
                "arcgis_world_geocoding",
                SourceState::Skipped,
                0,
                "No place text was supplied for geocoding.",
            ));
            append_spatial_skips(&mut statuses, "No resolved center is available.");
            return Ok(empty_response(radius_m, reason, statuses));
        }
        LocationPlan::Geocode(place) => {
            match resolve_geocoded_center(&place, &language, &mut statuses).await {
                Some(center) => (center, false),
                None => {
                    append_spatial_skips(&mut statuses, "No resolved center is available.");
                    return Ok(empty_response(
                    radius_m,
                    format!(
                        "Neither Nominatim nor the strict ArcGIS fallback could resolve '{place}' at the requested precision. Provide a more specific city, address, neighborhood, or coordinates; no coordinate was guessed."
                    ),
                    statuses,
                ));
                }
            }
        }
        LocationPlan::Coordinates(center) => (center, true),
    };

    let reverse = async {
        if reverse_needed {
            timed(
                "Nominatim",
                reverse_geocode(center.latitude, center.longitude, &language),
            )
            .await
        } else {
            Ok(None)
        }
    };
    let overpass = timed_with_timeout(
        "Overpass",
        OVERPASS_TOTAL_TIMEOUT,
        fetch_overpass_places(&query, &center, radius_m, &language),
    );
    let weather = timed("Open-Meteo", fetch_weather(&center));
    let wikipedia = timed(
        "Wikipedia GeoSearch",
        fetch_wikipedia_places(&center, radius_m, limit, &language),
    );
    let baidu = timed(
        "Baidu Place",
        fetch_baidu_places(&query, &center, radius_m, limit),
    );
    let amap = timed(
        "Amap Place",
        fetch_amap_places(&query, &center, radius_m, limit),
    );
    let (
        reverse_result,
        overpass_result,
        weather_result,
        wikipedia_result,
        baidu_result,
        amap_result,
    ) = tokio::join!(reverse, overpass, weather, wikipedia, baidu, amap);

    if reverse_needed {
        match reverse_result {
            Ok(Some(label)) => {
                center.label = label;
                center.label_source = Some("nominatim".into());
                statuses.push(status(
                    "nominatim",
                    SourceState::Success,
                    1,
                    "Supplied coordinates were reverse-geocoded for a display label.",
                ));
            }
            Ok(None) => statuses.push(status(
                "nominatim",
                SourceState::Empty,
                0,
                "No reverse-geocoded display label was returned; supplied coordinates remain usable.",
            )),
            Err(error) => statuses.push(status(
                "nominatim",
                SourceState::Failed,
                0,
                format!(
                    "Reverse geocoding failed, but supplied coordinates remain usable: {error}"
                ),
            )),
        }
        statuses.push(status(
            "arcgis_world_geocoding",
            SourceState::Skipped,
            0,
            "Coordinates were supplied directly, so the ArcGIS forward-geocoding fallback was not requested.",
        ));
    }

    let mut places = Vec::new();
    match overpass_result {
        Ok(result) if result.places.is_empty() => {
            let detail = result.warning.as_deref().map_or_else(
                || {
                    format!(
                        "No named OpenStreetMap POIs matched this category and radius via {}.",
                        result.endpoint
                    )
                },
                |warning| {
                    format!(
                        "Overpass via {} returned a partial result set: {warning}; no named OpenStreetMap POIs survived filtering.",
                        result.endpoint
                    )
                },
            );
            statuses.push(status_as_of(
                "overpass",
                SourceState::Empty,
                0,
                detail,
                result.data_as_of,
            ));
        }
        Ok(result) => {
            let count = result.places.len();
            let detail = result.warning.as_deref().map_or_else(
                || {
                    format!(
                        "Named OpenStreetMap POIs returned via {}. data_as_of is the Overpass OSM base snapshot, not a per-place verification time.",
                        result.endpoint
                    )
                },
                |warning| {
                    format!(
                        "Overpass via {} returned a partial result set: {warning}; named OpenStreetMap POIs were salvaged from the usable portion. data_as_of is the OSM base snapshot, not a per-place verification time.",
                        result.endpoint
                    )
                },
            );
            places.extend(result.places);
            statuses.push(status_as_of(
                "overpass",
                SourceState::Success,
                count,
                detail,
                result.data_as_of,
            ));
        }
        Err(error) => statuses.push(status("overpass", SourceState::Failed, 0, error)),
    }

    // ── 百度/高德 POI 富化：与 OSM 结果合并，补充评分/价格/营业时间 ──────────
    let mut commercial_places: Vec<DiscoveryPlace> = Vec::new();
    match baidu_result {
        Ok(bp) if !bp.is_empty() => {
            let count = bp.len();
            commercial_places.extend(bp);
            statuses.push(status(
                "baidu_map",
                SourceState::Success,
                count,
                "Baidu Map POI search returned commercial data (rating/price/hours).",
            ));
        }
        Ok(_) => {
            if baidu_ak().is_some() {
                statuses.push(status(
                    "baidu_map",
                    SourceState::Empty,
                    0,
                    "No Baidu POI results for this query.",
                ));
            } else {
                statuses.push(status(
                    "baidu_map",
                    SourceState::Skipped,
                    0,
                    "BAIDU_MAP_AK not set.",
                ));
            }
        }
        Err(error) => statuses.push(status("baidu_map", SourceState::Failed, 0, error)),
    }
    match amap_result {
        Ok(ap) if !ap.is_empty() => {
            let count = ap.len();
            commercial_places.extend(ap);
            statuses.push(status(
                "amap",
                SourceState::Success,
                count,
                "Amap POI search returned commercial data (rating/price).",
            ));
        }
        Ok(_) => {
            if amap_key().is_some() {
                statuses.push(status(
                    "amap",
                    SourceState::Empty,
                    0,
                    "No Amap POI results for this query.",
                ));
            } else {
                statuses.push(status("amap", SourceState::Skipped, 0, "AMAP_KEY not set."));
            }
        }
        Err(error) => statuses.push(status("amap", SourceState::Failed, 0, error)),
    }
    // Merge: enrich OSM places with commercial data by name proximity, then
    // append any commercial-only places not found in OSM.
    if !commercial_places.is_empty() {
        for cp in &commercial_places {
            let cp_name = cp.name.to_lowercase();
            let matched = places.iter_mut().find(|p| {
                let pn = p.name.to_lowercase();
                pn == cp_name || pn.contains(&cp_name) || cp_name.contains(&pn)
            });
            if let Some(existing) = matched {
                if existing.rating.is_none() && cp.rating.is_some() {
                    existing.rating = cp.rating;
                }
                if existing.price.is_none() && cp.price.is_some() {
                    existing.price = cp.price.clone();
                }
                if existing.opening_hours.is_none() && cp.opening_hours.is_some() {
                    existing.opening_hours = cp.opening_hours.clone();
                }
                if existing.address.is_none() && cp.address.is_some() {
                    existing.address = cp.address.clone();
                }
                if cp.source_url.is_some() {
                    existing.source_url = cp.source_url.clone();
                }
                existing.source = format!("{} + {}", existing.source, cp.source);
            } else {
                places.push(cp.clone());
            }
        }
    }

    let weather = match weather_result {
        Ok(Some(weather)) => {
            statuses.push(status(
                "open_meteo",
                SourceState::Success,
                1,
                "Current weather fields returned for the resolved center.",
            ));
            Some(weather)
        }
        Ok(None) => {
            statuses.push(status(
                "open_meteo",
                SourceState::Empty,
                0,
                "The provider returned no current weather object.",
            ));
            None
        }
        Err(error) => {
            statuses.push(status("open_meteo", SourceState::Failed, 0, error));
            None
        }
    };

    let mut nearby_context = Vec::new();
    match wikipedia_result {
        Ok(items) if items.is_empty() => statuses.push(status(
            "wikipedia",
            SourceState::Empty,
            0,
            "No nearby geotagged Wikipedia articles were returned.",
        )),
        Ok(items) => {
            let count = items.len();
            nearby_context.extend(items);
            statuses.push(status(
                "wikipedia",
                SourceState::Success,
                count,
                "Nearby geotagged encyclopedia articles returned separately as context; they are not POIs or recommendations.",
            ));
        }
        Err(error) => statuses.push(status("wikipedia", SourceState::Failed, 0, error)),
    }

    let (places, nearby_context) =
        finalize_places_and_context(&query, places, nearby_context, limit as usize);
    Ok(LocalDiscoveryResponse {
        center: Some(center),
        places,
        nearby_context,
        weather,
        source_statuses: statuses,
        limitations: base_limitations(radius_m),
        retrieved_at: unix_now(),
        radius_m,
    })
}

/// Great-circle distance over a spherical Earth. This deliberately does not
/// claim road, walking, transit, traffic, or accessibility distance.
pub fn haversine_m(lat_a: f64, lon_a: f64, lat_b: f64, lon_b: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_008.8;
    let lat_a = lat_a.to_radians();
    let lat_b = lat_b.to_radians();
    let delta_lat = lat_b - lat_a;
    let delta_lon = (lon_b - lon_a).to_radians();
    let hav = (delta_lat / 2.0).sin().powi(2)
        + lat_a.cos() * lat_b.cos() * (delta_lon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * hav.clamp(0.0, 1.0).sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center() -> DiscoveryCenter {
        DiscoveryCenter {
            label: "Test center".into(),
            latitude: 34.0522,
            longitude: -118.2437,
            source: "provided_coordinates".into(),
            label_source: None,
        }
    }

    #[test]
    fn current_location_without_coordinates_is_not_inferred() {
        let plan = location_plan(Some("current"), None, None).unwrap();
        match plan {
            LocationPlan::Missing(reason) => {
                assert!(reason.contains("no coordinates"));
                assert!(reason.contains("timezone is not a location"));
            }
            other => panic!("expected missing location, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn command_returns_a_truthful_location_required_response_without_network() {
        let response = local_discovery(
            "nearby food".into(),
            Some("current".into()),
            None,
            None,
            None,
            None,
            Some("en".into()),
        )
        .await
        .unwrap();
        assert_eq!(response.center, None);
        assert!(response.places.is_empty());
        assert_eq!(response.source_statuses.len(), 5);
        assert!(response
            .source_statuses
            .iter()
            .all(|source| source.status == SourceState::Skipped));
        assert!(response.limitations[0].contains("no coordinates"));
        assert!(response.limitations[0].contains("timezone is not a location"));
    }

    #[tokio::test]
    #[ignore = "calls live public geocoding, POI, weather, and encyclopedia endpoints"]
    async fn live_local_discovery_reports_each_public_source() {
        let response = local_discovery(
            "coffee".into(),
            Some("Pasadena, California".into()),
            None,
            None,
            Some(2_000),
            Some(5),
            Some("en".into()),
        )
        .await
        .unwrap();
        assert!(response.center.is_some());
        assert_eq!(response.source_statuses.len(), 5);
        assert!(response
            .source_statuses
            .iter()
            .any(|source| source.status == SourceState::Success));
        eprintln!("{}", serde_json::to_string_pretty(&response).unwrap());
    }

    #[tokio::test]
    #[ignore = "calls the live public ArcGIS World Geocoding endpoint"]
    async fn live_arcgis_resolves_the_shanghai_street_address_at_address_precision() {
        let result = forward_geocode_arcgis("上海市静安区胶州路282号")
            .await
            .unwrap()
            .expect("the live endpoint should return a strict address match");
        assert_eq!(result.center.source, "arcgis_world_geocoding");
        assert!(result.center.label.contains("胶州路282号"));
        assert!(result.score >= ARCGIS_MIN_MATCH_SCORE);
        assert!(matches!(
            result.address_type.as_str(),
            "Subaddress" | "PointAddress" | "StreetAddress"
        ));
        assert!((31.22..31.24).contains(&result.center.latitude));
        assert!((121.42..121.45).contains(&result.center.longitude));
    }

    #[tokio::test]
    #[ignore = "calls live public geocoding, POI, weather, and encyclopedia endpoints"]
    async fn live_shanghai_address_uses_arcgis_fallback_in_the_full_discovery_flow() {
        let response = local_discovery(
            "餐厅".into(),
            Some("上海市胶州路282号".into()),
            None,
            None,
            Some(3_000),
            Some(5),
            Some("zh".into()),
        )
        .await
        .unwrap();
        let center = response
            .center
            .as_ref()
            .expect("the exact address should resolve");
        assert_eq!(center.source, "arcgis_world_geocoding");
        assert!(center.label.contains("胶州路282号"));
        assert!(response.source_statuses.iter().any(|status| {
            status.source == "arcgis_world_geocoding" && status.status == SourceState::Success
        }));
        if let Some(overpass) = response
            .source_statuses
            .iter()
            .find(|status| status.source == "overpass")
        {
            if matches!(overpass.status, SourceState::Success | SourceState::Empty) {
                assert!(overpass.data_as_of.is_some());
            }
        }
        eprintln!("{}", serde_json::to_string_pretty(&response).unwrap());
    }

    #[test]
    fn house_number_intent_requires_address_context() {
        for place in [
            "District 7, Ho Chi Minh City",
            "90210",
            "Terminal 2, Singapore Changi Airport",
            "Route 66, California",
            "5th Avenue, New York",
            "上海市第2街",
            "上海市66路",
        ] {
            assert!(
                !has_house_number_intent(place),
                "{place:?} must remain a general place query"
            );
        }
        for address in [
            "上海市静安区胶州路282号",
            "上海市静安区胶州路十二号",
            "1600 Amphitheatre Pkwy, Mountain View, CA 94043",
            "Main St 12A",
        ] {
            assert!(
                has_house_number_intent(address),
                "{address:?} must request house-level precision"
            );
        }
    }

    #[test]
    fn house_number_matching_is_exact_after_limited_normalization() {
        assert!(house_number_matches("上海市胶州路282号", "282"));
        assert!(house_number_matches("上海市胶州路十二号", "12"));
        assert!(house_number_matches("Main St 12A", "12a"));
        assert!(house_number_matches("胶州路12之1号", "12-1"));
        assert!(!house_number_matches("Main St 12A", "12B"));
        assert!(!house_number_matches("Main St 12A/12B", "12A"));
        assert!(house_number_matches("Main St 12A/12B", "12A/12B"));
        assert!(!house_number_matches("上海市胶州路282号", "999"));
        assert!(!house_number_matches("上海市胶州路282号", "282-1"));
        assert!(!house_number_matches("District 7, Ho Chi Minh City", "7"));
        assert!(house_number_matches("地铁2号出口 上海市胶州路282号", "282"));
        assert!(!house_number_matches("地铁2号出口 上海市胶州路282号", "2"));
    }

    #[test]
    fn arcgis_fixture_accepts_only_a_high_confidence_precise_house_match() {
        let fixture = r#"{
          "candidates": [
            {
              "address": "上海市静安区曹家渡街道胶州路282号",
              "location": {"x": 121.43875479716, "y": 31.230038216008},
              "score": 100,
              "attributes": {
                "Match_addr": "上海市静安区曹家渡街道胶州路282号",
                "Addr_type": "StreetAddress",
                "AddNum": "282"
              }
            },
            {
              "address": "上海静安区",
              "location": {"x": 121.41667, "y": 31.23333},
              "score": 77.68,
              "attributes": {"Match_addr": "上海静安区", "Addr_type": "Locality", "AddNum": null}
            }
          ]
        }"#;
        let response: ArcgisGeocodeResponse = serde_json::from_str(fixture).unwrap();
        let result = arcgis_match_from_response("上海市静安区胶州路282号", response)
            .unwrap()
            .unwrap();

        assert_eq!(result.center.source, "arcgis_world_geocoding");
        assert_eq!(
            result.center.label_source.as_deref(),
            Some("arcgis_world_geocoding")
        );
        assert_eq!(result.address_type, "StreetAddress");
        assert_eq!(result.score, 100.0);
        assert_eq!(result.center.latitude, 31.230038216008);
        assert_eq!(result.center.longitude, 121.43875479716);
    }

    #[test]
    fn arcgis_fixture_rejects_locality_low_score_and_wrong_house_candidates() {
        let fixture = r#"{
          "candidates": [
            {
              "address": "上海市静安区胶州路282号",
              "location": {"x": 121.42, "y": 31.23},
              "score": 100,
              "attributes": {"Match_addr": "上海市静安区胶州路282号", "Addr_type": "Locality", "AddNum": "282"}
            },
            {
              "address": "上海市静安区胶州路282号",
              "location": {"x": 121.43, "y": 31.23},
              "score": 89.99,
              "attributes": {"Match_addr": "上海市静安区胶州路282号", "Addr_type": "PointAddress", "AddNum": "282"}
            },
            {
              "address": "上海市静安区胶州路999号",
              "location": {"x": 121.44, "y": 31.23},
              "score": 99,
              "attributes": {"Match_addr": "上海市静安区胶州路999号", "Addr_type": "StreetAddress", "AddNum": "999"}
            }
          ]
        }"#;
        let response: ArcgisGeocodeResponse = serde_json::from_str(fixture).unwrap();
        assert!(
            arcgis_match_from_response("上海市静安区胶州路282号", response)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn coordinate_pairs_are_validated_and_take_precedence() {
        let plan = location_plan(Some("current"), Some(34.0), Some(-118.0)).unwrap();
        assert!(matches!(plan, LocationPlan::Coordinates(_)));
        assert!(location_plan(None, Some(34.0), None).is_err());
        assert!(location_plan(None, Some(91.0), Some(0.0)).is_err());
        assert!(location_plan(None, Some(0.0), Some(181.0)).is_err());
    }

    #[test]
    fn place_text_is_geocoded_without_mixing_in_the_search_query() {
        assert_eq!(
            location_plan(Some("Pasadena, CA"), None, None).unwrap(),
            LocationPlan::Geocode("Pasadena, CA".into())
        );
    }

    #[test]
    fn haversine_is_zero_and_matches_a_known_city_distance() {
        assert!(haversine_m(34.0522, -118.2437, 34.0522, -118.2437) < 0.001);
        let los_angeles_to_san_francisco = haversine_m(34.0522, -118.2437, 37.7749, -122.4194);
        assert!((555_000.0..565_000.0).contains(&los_angeles_to_san_francisco));
        assert!(haversine_m(0.0, 0.0, 0.0, 180.0).is_finite());
    }

    #[test]
    fn food_query_uses_bounded_category_selectors_not_raw_user_ql() {
        let statement = overpass_query("附近有什么好吃的 \" ); out body; //", &center(), 3_000);
        assert!(statement.contains("restaurant|cafe|fast_food"));
        assert!(statement.contains("bakery|pastry|confectionery|deli|tea|coffee"));
        assert!(!statement.contains("out body"));
        assert!(statement.contains(&format!("[maxsize:{OVERPASS_MAXSIZE_BYTES}]")));
        assert!(statement.ends_with("out center;"));
        assert!(!statement.contains("out center 96"));
    }

    #[test]
    fn overpass_empty_mirror_does_not_block_later_places() {
        fn result(endpoint: &str, places: Vec<DiscoveryPlace>) -> OverpassPlacesResult {
            OverpassPlacesResult {
                places,
                data_as_of: Some("2026-07-14T11:45:15Z".into()),
                warning: None,
                endpoint: endpoint.into(),
            }
        }
        let mut empty_result = None;
        let mut partial_empty_result = None;
        assert!(overpass_result_or_remember_empty(
            &mut empty_result,
            &mut partial_empty_result,
            result("overpass.osm.ch", Vec::new()),
        )
        .is_none());
        let selected = overpass_result_or_remember_empty(
            &mut empty_result,
            &mut partial_empty_result,
            result(
                "overpass-api.de",
                vec![DiscoveryPlace {
                    id: "osm:node/1".into(),
                    name: "家味螺蛳粉".into(),
                    category: Some("amenity:fast_food".into()),
                    latitude: 31.241,
                    longitude: 121.432,
                    distance_m: 238,
                    source: "openstreetmap".into(),
                    source_url: None,
                    address: None,
                    cuisine: Some("chinese".into()),
                    opening_hours: None,
                    rating: None,
                    price: None,
                    open_now: None,
                }],
            ),
        )
        .expect("later populated mirror must win over an earlier clean empty response");
        assert_eq!(selected.endpoint, "overpass-api.de");
        assert_eq!(selected.places.len(), 1);
        assert!(empty_result.is_some());
        assert!(partial_empty_result.is_none());
    }

    #[test]
    fn category_detection_does_not_treat_theater_as_eat() {
        let statement = overpass_query("nearby theater", &center(), 3_000);
        assert_eq!(poi_intent("nearby theater"), PoiIntent::Entertainment);
        assert!(statement.contains("cinema|theatre|arts_centre"));
        assert!(!statement.contains("restaurant|cafe|fast_food"));
    }

    #[test]
    fn bakery_and_family_queries_use_real_osm_categories() {
        let bakery = overpass_query("nearby bakery", &center(), 3_000);
        assert_eq!(poi_intent("nearby bakeries"), PoiIntent::Bakery);
        assert!(bakery.contains("[\"shop\"~\"^(bakery|pastry|confectionery)$\"]"));
        assert!(!bakery.contains("restaurant|cafe|fast_food"));

        let family = overpass_query("family activity", &center(), 3_000);
        assert_eq!(poi_intent("family activities"), PoiIntent::FamilyActivity);
        assert!(family.contains("playground|water_park|miniature_golf"));
        assert!(family.contains("zoo|aquarium|theme_park|museum|attraction"));
        assert!(family.contains("cinema|theatre|arts_centre|community_centre"));
    }

    #[test]
    fn common_local_intents_are_specific_and_word_bounded() {
        assert_eq!(poi_intent("need a pharmacy"), PoiIntent::Healthcare);
        assert_eq!(poi_intent("nearby supermarkets"), PoiIntent::Grocery);
        assert_eq!(poi_intent("nearby parking"), PoiIntent::Transport);
        assert_eq!(poi_intent("附近理发店"), PoiIntent::PersonalServices);
        assert_eq!(poi_intent("nearest gyms"), PoiIntent::Fitness);
        assert_eq!(poi_intent("sparkling water"), PoiIntent::General);
    }

    #[test]
    fn market_queries_take_priority_over_generic_food() {
        assert_eq!(poi_intent("nearby food market"), PoiIntent::Grocery);
        assert_eq!(poi_intent("local farmers market"), PoiIntent::Grocery);
        assert_eq!(poi_intent("bakery near a food market"), PoiIntent::Bakery);

        let statement = overpass_query("food market", &center(), 3_000);
        assert!(statement.contains("supermarket|convenience|greengrocer|farm"));
        assert!(statement.contains("[\"amenity\"=\"marketplace\"]"));
        assert!(!statement.contains("restaurant|cafe|fast_food"));
    }

    #[test]
    fn relevance_is_applied_before_the_final_limit() {
        let mut places = (0..20)
            .map(|index| DiscoveryPlace {
                id: format!("near:{index}"),
                name: format!("Generic restaurant {index}"),
                category: Some("amenity:restaurant".into()),
                latitude: 34.0,
                longitude: -118.0,
                distance_m: index,
                source: "openstreetmap".into(),
                source_url: None,
                address: None,
                cuisine: None,
                opening_hours: None,
                rating: None,
                price: None,
                open_now: None,
            })
            .collect::<Vec<_>>();
        places.push(DiscoveryPlace {
            id: "exact".into(),
            name: "Neighborhood Sushi".into(),
            category: Some("amenity:restaurant".into()),
            latitude: 34.0,
            longitude: -118.0,
            distance_m: 900,
            source: "openstreetmap".into(),
            source_url: None,
            address: None,
            cuisine: Some("sushi".into()),
            opening_hours: None,
            rating: None,
            price: None,
            open_now: None,
        });

        sort_and_limit_places("sushi", &mut places, 5);
        assert_eq!(places[0].id, "exact");
        assert_eq!(places.len(), 5);
    }

    #[test]
    fn wikipedia_context_cannot_displace_osm_pois() {
        let places = (0..4)
            .map(|index| DiscoveryPlace {
                id: format!("osm:{index}"),
                name: format!("Cafe {index}"),
                category: Some("amenity:cafe".into()),
                latitude: 34.0,
                longitude: -118.0,
                distance_m: 100 + index,
                source: "openstreetmap".into(),
                source_url: None,
                address: None,
                cuisine: None,
                opening_hours: None,
                rating: None,
                price: None,
                open_now: None,
            })
            .collect::<Vec<_>>();
        let context = vec![DiscoveryPlace {
            id: "wikipedia:en:1".into(),
            name: "Cafe history".into(),
            category: Some("encyclopedia_article".into()),
            latitude: 34.0,
            longitude: -118.0,
            distance_m: 1,
            source: "wikipedia".into(),
            source_url: None,
            address: None,
            cuisine: None,
            opening_hours: None,
            rating: None,
            price: None,
            open_now: None,
        }];

        let (places, context) = finalize_places_and_context("cafe", places, context, 3);

        assert_eq!(places.len(), 3);
        assert!(places.iter().all(|place| place.source == "openstreetmap"));
        assert_eq!(context.len(), 1);
        assert_eq!(context[0].source, "wikipedia");
    }

    #[test]
    fn chinese_food_queries_match_structured_cuisine_tags() {
        let place = DiscoveryPlace {
            id: "sichuan".into(),
            name: "Neighborhood Kitchen".into(),
            category: Some("amenity:restaurant".into()),
            latitude: 34.0,
            longitude: -118.0,
            distance_m: 500,
            source: "openstreetmap".into(),
            source_url: None,
            address: None,
            cuisine: Some("sichuan".into()),
            opening_hours: None,
            rating: None,
            price: None,
            open_now: None,
        };
        assert!(relevance_score("附近川菜", &place) > 0);
    }

    #[test]
    fn overpass_places_never_invent_commercial_or_live_fields() {
        let element = OverpassElement {
            element_type: "node".into(),
            id: 42,
            lat: Some(34.053),
            lon: Some(-118.244),
            center: None,
            tags: HashMap::from([
                ("name".into(), "Actual Cafe".into()),
                ("amenity".into(), "cafe".into()),
                ("opening_hours".into(), "Mo-Fr 08:00-17:00".into()),
            ]),
        };
        let place = overpass_element_to_place(element, &center(), "en", 3_000).unwrap();
        assert_eq!(place.opening_hours.as_deref(), Some("Mo-Fr 08:00-17:00"));
        assert_eq!(place.rating, None);
        assert_eq!(place.price, None);
        assert_eq!(place.open_now, None);
        assert!(place.distance_m > 0);
    }

    #[test]
    fn overpass_feature_centers_outside_the_requested_radius_are_excluded() {
        let element = OverpassElement {
            element_type: "way".into(),
            id: 43,
            lat: None,
            lon: None,
            center: Some(OverpassCenter {
                lat: center().latitude + 0.002,
                lon: center().longitude,
            }),
            tags: HashMap::from([
                ("name".into(), "Large Park".into()),
                ("leisure".into(), "park".into()),
            ]),
        };

        assert!(overpass_element_to_place(element, &center(), "en", 100).is_none());
    }

    #[test]
    fn overpass_remarks_preserve_partial_elements_for_fallbacks() {
        let response = OverpassResponse {
            remark: Some("runtime error: Query ran out of memory".into()),
            osm3s: None,
            elements: vec![OverpassElement {
                element_type: "node".into(),
                id: 44,
                lat: Some(34.053),
                lon: Some(-118.244),
                center: None,
                tags: HashMap::from([
                    ("name".into(), "Partial Cafe".into()),
                    ("amenity".into(), "cafe".into()),
                ]),
            }],
        };

        let completion = complete_overpass_elements(response);
        assert_eq!(completion.elements.len(), 1);
        assert_eq!(
            completion.warning.as_deref(),
            Some("runtime error: Query ran out of memory")
        );
    }

    #[test]
    fn overpass_remarks_without_elements_are_kept_as_partial_empty() {
        let response = OverpassResponse {
            remark: Some("runtime error: Query timed out in \"query\"".into()),
            osm3s: None,
            elements: Vec::new(),
        };

        let completion = complete_overpass_elements(response);
        assert!(completion.elements.is_empty());
        assert_eq!(
            completion.warning.as_deref(),
            Some("runtime error: Query timed out in \"query\"")
        );
    }

    #[test]
    fn overpass_fixture_exposes_the_osm_base_snapshot_time() {
        let response: OverpassResponse = serde_json::from_str(
            r#"{
              "osm3s": {"timestamp_osm_base": "2026-07-12T09:57:14Z"},
              "elements": []
            }"#,
        )
        .unwrap();
        assert_eq!(
            overpass_data_as_of(&response).as_deref(),
            Some("2026-07-12T09:57:14Z")
        );
        let source = status_as_of(
            "overpass",
            SourceState::Success,
            0,
            "fixture",
            overpass_data_as_of(&response),
        );
        assert_eq!(source.data_as_of.as_deref(), Some("2026-07-12T09:57:14Z"));
    }

    #[test]
    fn wikipedia_language_is_safe_and_has_a_chinese_default() {
        assert_eq!(wikipedia_language(Some("zh-CN"), "food"), "zh");
        assert_eq!(wikipedia_language(Some("../../evil"), "附近景点"), "zh");
        assert_eq!(wikipedia_language(None, "nearby museum"), "en");
    }

    #[test]
    fn limitations_are_explicit_about_distance_hours_and_missing_commercial_data() {
        let limitations = base_limitations(12_000).join(" ");
        assert!(limitations.contains("straight-line"));
        assert!(limitations.contains("open_now remains null"));
        assert!(limitations.contains("rating and price remain null"));
        assert!(limitations.contains("not a live event schedule"));
        assert!(limitations.contains("retrieved_at"));
        assert!(limitations.contains("OSM base snapshot"));
        assert!(limitations.contains("map databases are not real-time"));
        assert!(limitations.contains("capped at 10 km"));
    }
}
