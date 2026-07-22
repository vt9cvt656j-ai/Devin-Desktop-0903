use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz;
use quick_xml::{events::Event as XmlEvent, Reader as XmlReader};
use reqwest::{header, Client};
use scraper::{ElementRef, Html, Selector};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::future::Future;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECORDS: usize = 50;
const FINTRAFFIC_DIRECTORY_CACHE_TTL: Duration = Duration::from_secs(60);
const NORWAY_DIRECTORY_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
const DIGITRAFFIC_USER: &str = "Michael-IDE/0.3 github.com/fendoushaonian/Devin-Desktop";
const PROVIDER_FUTURE_TOLERANCE_MINUTES: i64 = 5;
const DELAYED_AFTER_MINUTES: i64 = 15;
const WINNIPEG_OBSERVATION_MAX_AGE_MINUTES: i64 = 60;
const NYC_OBSERVATION_MAX_AGE_MINUTES: i64 = 60;
const CHICAGO_FLOW_OBSERVATION_MAX_AGE_MINUTES: i64 = 30;
const FINTRAFFIC_OBSERVATION_MAX_AGE_MINUTES: i64 = 15;
const NORWAY_OBSERVATION_MAX_AGE_MINUTES: i64 = 8 * 60;
const CALTRANS_CHP_REPRESENTATION_MAX_AGE_MINUTES: i64 = 15;
const CALTRANS_CHP_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const CALTRANS_CHP_KML_URL: &str = "https://quickmap.dot.ca.gov/data/v2_chp-only.kml";

static HTTP: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(60))
        .user_agent("Michael-IDE/1.0 (+https://github.com/fendoushaonian/Devin-Desktop)")
        .build()
        .unwrap_or_else(|_| Client::new())
});

struct CachedDirectory {
    value: Value,
    stored_at: Instant,
}

static FINTRAFFIC_STATION_DIRECTORY: LazyLock<RwLock<Option<CachedDirectory>>> =
    LazyLock::new(|| RwLock::new(None));
static NORWAY_STATION_DIRECTORY: LazyLock<RwLock<Option<CachedDirectory>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveSourceState {
    Success,
    Delayed,
    Empty,
    Stale,
    NoCoverage,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LiveSourceStatus {
    pub source: String,
    pub status: LiveSourceState,
    pub result_count: usize,
    pub detail: String,
    pub data_as_of: Option<String>,
    /// Meaning of `data_as_of`; road sources use a concrete provider-time
    /// semantic instead of treating observation, feed, and event times alike.
    pub data_as_of_kind: Option<String>,
    /// Derived only from the provider timestamp. It does not describe endpoint
    /// completeness or prove that an incident is still present on the road.
    pub freshness: Option<String>,
    pub provider_time_age_seconds: Option<u64>,
    /// Dynamic observations are not cached. Directory-prefixed states refer
    /// only to a provider's station catalog; the observation was fetched live.
    pub cache_state: Option<String>,
    pub cache_age_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveDataResponse {
    pub topic: String,
    pub records: Vec<Value>,
    pub source_statuses: Vec<LiveSourceStatus>,
    pub limitations: Vec<String>,
    /// Unix seconds when the IDE finished this request. It is not the provider's
    /// observation, publication, exchange, or carrier event time.
    pub retrieved_at: u64,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn status(
    source: &str,
    state: LiveSourceState,
    count: usize,
    detail: impl Into<String>,
    data_as_of: Option<String>,
) -> LiveSourceStatus {
    LiveSourceStatus {
        source: source.into(),
        status: state,
        result_count: count,
        detail: detail.into(),
        data_as_of,
        data_as_of_kind: None,
        freshness: None,
        provider_time_age_seconds: None,
        cache_state: None,
        cache_age_seconds: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservationWindowState {
    NearRealTime,
    Delayed,
    Stale,
    Future,
    MissingOrInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservationWindow {
    state: ObservationWindowState,
    age_seconds: Option<u64>,
}

fn observation_window_at(
    timestamp: Option<&str>,
    timezone: Tz,
    max_age_minutes: i64,
    now: DateTime<Utc>,
) -> ObservationWindow {
    let Some(parsed) = timestamp.and_then(|value| parse_provider_time(value, timezone)) else {
        return ObservationWindow {
            state: ObservationWindowState::MissingOrInvalid,
            age_seconds: None,
        };
    };
    if parsed > now + ChronoDuration::minutes(PROVIDER_FUTURE_TOLERANCE_MINUTES) {
        return ObservationWindow {
            state: ObservationWindowState::Future,
            age_seconds: None,
        };
    }
    let age = now.signed_duration_since(parsed);
    let state = if age > ChronoDuration::minutes(max_age_minutes) {
        ObservationWindowState::Stale
    } else if age > ChronoDuration::minutes(DELAYED_AFTER_MINUTES) {
        ObservationWindowState::Delayed
    } else {
        ObservationWindowState::NearRealTime
    };
    ObservationWindow {
        state,
        age_seconds: Some(age.num_seconds().max(0) as u64),
    }
}

fn observation_window(
    timestamp: Option<&str>,
    timezone: Tz,
    max_age_minutes: i64,
) -> ObservationWindow {
    observation_window_at(timestamp, timezone, max_age_minutes, Utc::now())
}

fn observation_is_usable(timestamp: Option<&str>, timezone: Tz, max_age_minutes: i64) -> bool {
    matches!(
        observation_window(timestamp, timezone, max_age_minutes).state,
        ObservationWindowState::NearRealTime | ObservationWindowState::Delayed
    )
}

fn timestamp_freshness(
    timestamp: Option<&str>,
    timezone: Tz,
    max_age_minutes: i64,
) -> (Option<String>, Option<u64>) {
    let window = observation_window(timestamp, timezone, max_age_minutes);
    let freshness = match window.state {
        ObservationWindowState::NearRealTime => "near_real_time",
        ObservationWindowState::Delayed => "delayed",
        ObservationWindowState::Stale => "stale",
        ObservationWindowState::Future | ObservationWindowState::MissingOrInvalid => "unknown",
    };
    (Some(freshness.into()), window.age_seconds)
}

#[allow(clippy::too_many_arguments)]
fn evidenced_status(
    source: &str,
    state: LiveSourceState,
    count: usize,
    detail: impl Into<String>,
    data_as_of: Option<String>,
    data_as_of_kind: Option<String>,
    timezone: Tz,
    max_age_minutes: Option<i64>,
    cache_state: Option<String>,
    cache_age_seconds: Option<u64>,
) -> LiveSourceStatus {
    let (freshness, provider_time_age_seconds) = timestamp_freshness(
        data_as_of.as_deref(),
        timezone,
        max_age_minutes.unwrap_or(24 * 60),
    );
    LiveSourceStatus {
        source: source.into(),
        status: state,
        result_count: count,
        detail: detail.into(),
        data_as_of,
        data_as_of_kind,
        freshness,
        provider_time_age_seconds,
        cache_state,
        cache_age_seconds,
    }
}

fn response(
    topic: &str,
    records: Vec<Value>,
    source_statuses: Vec<LiveSourceStatus>,
    mut limitations: Vec<String>,
) -> LiveDataResponse {
    limitations.push(
        "retrieved_at is when Michael IDE completed this request, not the provider's observation, publication, road event, sensor interval, market, flight, or shipment event time."
            .into(),
    );
    limitations.push(
        "A source status of success means the public endpoint returned parseable data in this request; it does not prove completeness, correctness, freshness, or independent corroboration."
            .into(),
    );
    LiveDataResponse {
        topic: topic.into(),
        records,
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

async fn response_json<T: DeserializeOwned>(
    source: &str,
    request: reqwest::RequestBuilder,
) -> Result<T, String> {
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
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{source} returned invalid JSON: {error}"))
}

fn coordinates(latitude: Option<f64>, longitude: Option<f64>) -> Result<(f64, f64), String> {
    match (latitude, longitude) {
        (Some(latitude), Some(longitude))
            if latitude.is_finite()
                && longitude.is_finite()
                && (-90.0..=90.0).contains(&latitude)
                && (-180.0..=180.0).contains(&longitude) =>
        {
            Ok((latitude, longitude))
        }
        (Some(_), Some(_)) => Err("latitude/longitude must be finite valid coordinates".into()),
        _ => Err("latitude and longitude are both required for this topic".into()),
    }
}

fn bounded_limit(limit: Option<u32>, default: usize) -> usize {
    limit
        .map(|value| value.clamp(1, MAX_RECORDS as u32) as usize)
        .unwrap_or(default)
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    })
}

fn value_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    })
}

fn value_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| match value {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    })
}

fn update_latest(current: &mut Option<String>, candidate: Option<String>) {
    if let Some(candidate) = candidate {
        if current.as_ref().is_none_or(|value| candidate > *value) {
            *current = Some(candidate);
        }
    }
}

fn parse_provider_time(value: &str, timezone: Tz) -> Option<DateTime<Utc>> {
    if let Ok(value) = DateTime::parse_from_rfc3339(value) {
        return Some(value.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(value) = NaiveDateTime::parse_from_str(value, format) {
            return timezone
                .from_local_datetime(&value)
                .earliest()
                .map(|value| value.with_timezone(&Utc));
        }
    }
    None
}

fn within_lookback_at(
    value: Option<&str>,
    timezone: Tz,
    lookback_hours: u32,
    now: DateTime<Utc>,
) -> bool {
    value
        .and_then(|value| parse_provider_time(value, timezone))
        .is_some_and(|value| {
            value >= now - ChronoDuration::hours(lookback_hours as i64)
                && value <= now + ChronoDuration::minutes(PROVIDER_FUTURE_TOLERANCE_MINUTES)
        })
}

fn within_lookback(value: Option<&str>, timezone: Tz, lookback_hours: u32) -> bool {
    within_lookback_at(value, timezone, lookback_hours, Utc::now())
}

async fn open_meteo_environment(
    kind: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(Vec<Value>, Option<String>), String> {
    let (source, url, query): (&str, &str, Vec<(&str, String)>) = match kind {
        "weather" => (
            "open_meteo_weather",
            "https://api.open-meteo.com/v1/forecast",
            vec![
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
                ("timezone", "auto".into()),
                ("forecast_days", "3".into()),
                ("current", "temperature_2m,apparent_temperature,precipitation,rain,weather_code,cloud_cover,wind_speed_10m,wind_direction_10m".into()),
                ("daily", "temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,weather_code,sunrise,sunset".into()),
            ],
        ),
        "air_quality" => (
            "open_meteo_air_quality",
            "https://air-quality-api.open-meteo.com/v1/air-quality",
            vec![
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
                ("timezone", "auto".into()),
                ("current", "pm10,pm2_5,carbon_monoxide,nitrogen_dioxide,sulphur_dioxide,ozone,aerosol_optical_depth,dust,uv_index,us_aqi,european_aqi".into()),
            ],
        ),
        "marine" => (
            "open_meteo_marine",
            "https://marine-api.open-meteo.com/v1/marine",
            vec![
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
                ("timezone", "auto".into()),
                ("forecast_days", "3".into()),
                ("current", "wave_height,wave_direction,wave_period,wind_wave_height,swell_wave_height,swell_wave_direction,sea_surface_temperature,ocean_current_velocity,ocean_current_direction".into()),
                ("daily", "wave_height_max,wave_direction_dominant,wave_period_max,wind_wave_height_max,swell_wave_height_max".into()),
            ],
        ),
        _ => return Err(format!("unsupported Open-Meteo environment kind: {kind}")),
    };
    let payload: Value = timed(source, response_json(source, HTTP.get(url).query(&query))).await?;
    let valid_at = payload
        .get("current")
        .and_then(|current| value_string(current.get("time")));
    let record = json!({
        "source": source,
        "fact_kind": "modeled_estimate",
        "latitude": payload.get("latitude"),
        "longitude": payload.get("longitude"),
        "timezone": payload.get("timezone"),
        "elevation_m": payload.get("elevation"),
        "valid_at": valid_at,
        "current_units": payload.get("current_units"),
        "current": payload.get("current"),
        "daily_units": payload.get("daily_units"),
        "daily": payload.get("daily"),
    });
    Ok((vec![record], valid_at))
}

fn haversine_km(latitude_a: f64, longitude_a: f64, latitude_b: f64, longitude_b: f64) -> f64 {
    let radius = 6_371.008_8_f64;
    let d_latitude = (latitude_b - latitude_a).to_radians();
    let d_longitude = (longitude_b - longitude_a).to_radians();
    let latitude_a = latitude_a.to_radians();
    let latitude_b = latitude_b.to_radians();
    let a = (d_latitude / 2.0).sin().powi(2)
        + latitude_a.cos() * latitude_b.cos() * (d_longitude / 2.0).sin().powi(2);
    radius * 2.0 * a.sqrt().asin()
}

fn earthquake_feed(window: Option<&str>, minimum_magnitude: f64) -> (&'static str, &'static str) {
    let window = match window.unwrap_or("day").trim().to_lowercase().as_str() {
        "hour" | "1h" => "hour",
        "week" | "7d" => "week",
        "month" | "30d" => "month",
        _ => "day",
    };
    let bucket = if minimum_magnitude >= 4.5 {
        "4.5"
    } else if minimum_magnitude >= 2.5 {
        "2.5"
    } else if minimum_magnitude >= 1.0 {
        "1.0"
    } else {
        "all"
    };
    let url = match (bucket, window) {
        ("4.5", "hour") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_hour.geojson"
        }
        ("4.5", "week") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_week.geojson"
        }
        ("4.5", "month") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_month.geojson"
        }
        ("4.5", _) => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_day.geojson",
        ("2.5", "hour") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_hour.geojson"
        }
        ("2.5", "week") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_week.geojson"
        }
        ("2.5", "month") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_month.geojson"
        }
        ("2.5", _) => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_day.geojson",
        ("1.0", "hour") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_hour.geojson"
        }
        ("1.0", "week") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_week.geojson"
        }
        ("1.0", "month") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_month.geojson"
        }
        ("1.0", _) => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_day.geojson",
        (_, "hour") => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_hour.geojson",
        (_, "week") => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_week.geojson",
        (_, "month") => {
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_month.geojson"
        }
        _ => "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_day.geojson",
    };
    (url, window)
}

fn parse_earthquakes(
    payload: &Value,
    minimum_magnitude: f64,
    center: Option<(f64, f64, f64)>,
    limit: usize,
) -> Vec<Value> {
    payload
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|feature| {
            let properties = feature.get("properties")?;
            let coordinates = feature.pointer("/geometry/coordinates")?.as_array()?;
            let longitude = coordinates.first()?.as_f64()?;
            let latitude = coordinates.get(1)?.as_f64()?;
            let depth_km = coordinates.get(2).and_then(Value::as_f64);
            let magnitude = properties.get("mag").and_then(Value::as_f64)?;
            if magnitude < minimum_magnitude {
                return None;
            }
            let distance_km = center.map(|(center_latitude, center_longitude, radius_km)| {
                (
                    haversine_km(center_latitude, center_longitude, latitude, longitude),
                    radius_km,
                )
            });
            if distance_km.is_some_and(|(distance, radius)| distance > radius) {
                return None;
            }
            Some(json!({
                "source": "usgs_earthquake_hazards_program",
                "id": feature.get("id"),
                "title": properties.get("title"),
                "place": properties.get("place"),
                "magnitude": magnitude,
                "magnitude_type": properties.get("magType"),
                "event_time_unix_ms": properties.get("time"),
                "updated_at_unix_ms": properties.get("updated"),
                "review_status": properties.get("status"),
                "tsunami_flag": properties.get("tsunami"),
                "significance": properties.get("sig"),
                "latitude": latitude,
                "longitude": longitude,
                "depth_km": depth_km,
                "distance_km": distance_km.map(|(distance, _)| (distance * 10.0).round() / 10.0),
                "source_url": properties.get("url"),
            }))
        })
        .take(limit)
        .collect()
}

async fn earthquake_data(
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_km: Option<u32>,
    window: Option<&str>,
    minimum_magnitude: Option<f64>,
    limit: usize,
) -> Result<(Vec<Value>, Option<String>, String), String> {
    let minimum_magnitude = minimum_magnitude
        .filter(|value| value.is_finite())
        .unwrap_or(2.5)
        .clamp(-1.0, 10.0);
    let center = match (latitude, longitude) {
        (None, None) => None,
        _ => {
            let (latitude, longitude) = coordinates(latitude, longitude)?;
            Some((
                latitude,
                longitude,
                radius_km.unwrap_or(500).clamp(1, 20_000) as f64,
            ))
        }
    };
    let (url, normalized_window) = earthquake_feed(window, minimum_magnitude);
    let payload: Value = timed(
        "USGS Earthquake Hazards Program",
        response_json("USGS Earthquake Hazards Program", HTTP.get(url)),
    )
    .await?;
    let data_as_of = value_string(payload.pointer("/metadata/generated"))
        .map(|value| format!("unix_ms:{value}"));
    Ok((
        parse_earthquakes(&payload, minimum_magnitude, center, limit),
        data_as_of,
        normalized_window.into(),
    ))
}

async fn natural_hazards(category: Option<&str>, limit: usize) -> Result<Vec<Value>, String> {
    let mut query = vec![("status", "open".to_string()), ("limit", limit.to_string())];
    let category = category
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    if let Some(category) = category {
        query.push(("category", category.to_string()));
    }
    let payload: Value = timed(
        "NASA EONET",
        response_json(
            "NASA EONET",
            HTTP.get("https://eonet.gsfc.nasa.gov/api/v3/events")
                .query(&query),
        ),
    )
    .await?;
    Ok(payload
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(limit)
        .map(|event| {
            let latest_geometry = event
                .get("geometry")
                .and_then(Value::as_array)
                .and_then(|items| items.last());
            json!({
                "source": "nasa_eonet",
                "id": event.get("id"),
                "title": event.get("title"),
                "description": event.get("description"),
                "categories": event.get("categories"),
                "closed_at": event.get("closed"),
                "latest_observation": latest_geometry,
                "upstream_sources": event.get("sources"),
                "source_url": event.get("link"),
            })
        })
        .collect())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named IPC fields are clearer than a positional blob.
pub async fn live_environment(
    kind: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius_km: Option<u32>,
    window: Option<String>,
    minimum_magnitude: Option<f64>,
    category: Option<String>,
    limit: Option<u32>,
) -> Result<LiveDataResponse, String> {
    let kind = kind.trim().to_lowercase();
    let limit = bounded_limit(limit, 12);
    match kind.as_str() {
        "weather" | "air_quality" | "marine" => {
            let (latitude, longitude) = coordinates(latitude, longitude)?;
            let source = match kind.as_str() {
                "weather" => "open_meteo_weather",
                "air_quality" => "open_meteo_air_quality",
                _ => "open_meteo_marine",
            };
            match open_meteo_environment(&kind, latitude, longitude).await {
                Ok((records, valid_at)) => {
                    let count = records.len();
                    Ok(response(
                        &kind,
                        records,
                        vec![status(
                            source,
                            if count == 0 {
                                LiveSourceState::Empty
                            } else {
                                LiveSourceState::Success
                            },
                            count,
                            format!(
                                "The keyless public Open-Meteo endpoint returned structured model fields{}.",
                                valid_at
                                    .as_deref()
                                    .map(|value| format!(" valid at {value}"))
                                    .unwrap_or_default()
                            ),
                            None,
                        )],
                        vec![
                            "Open-Meteo values are modelled grid estimates for the returned coordinate and timestamp, not an on-site sensor guarantee.".into(),
                            "Null marine fields commonly mean the coordinate is inland or the marine grid has no usable value; null must not be replaced with a guess.".into(),
                        ],
                    ))
                }
                Err(error) => Ok(response(
                    &kind,
                    Vec::new(),
                    vec![status(source, LiveSourceState::Failed, 0, error, None)],
                    vec!["No fallback value was invented when the public endpoint failed.".into()],
                )),
            }
        }
        "earthquakes" => match earthquake_data(
            latitude,
            longitude,
            radius_km,
            window.as_deref(),
            minimum_magnitude,
            limit,
        )
        .await
        {
            Ok((records, data_as_of, normalized_window)) => {
                let count = records.len();
                Ok(response(
                    "earthquakes",
                    records,
                    vec![status(
                        "usgs_earthquake_hazards_program",
                        if count == 0 {
                            LiveSourceState::Empty
                        } else {
                            LiveSourceState::Success
                        },
                        count,
                        format!("USGS summary feed window: {normalized_window}."),
                        data_as_of,
                    )],
                    vec![
                        "USGS events can be revised after automatic or analyst review; event_time and updated_at are separate fields.".into(),
                        "distance_km, when present, is a Haversine surface distance to the epicenter, not shaking intensity or travel distance.".into(),
                    ],
                ))
            }
            Err(error) => Ok(response(
                "earthquakes",
                Vec::new(),
                vec![status(
                    "usgs_earthquake_hazards_program",
                    LiveSourceState::Failed,
                    0,
                    error,
                    None,
                )],
                vec!["No earthquake event was inferred when USGS was unavailable.".into()],
            )),
        },
        "natural_hazards" => match natural_hazards(category.as_deref(), limit).await {
            Ok(records) => {
                let count = records.len();
                Ok(response(
                    "natural_hazards",
                    records,
                    vec![status(
                        "nasa_eonet",
                        if count == 0 {
                            LiveSourceState::Empty
                        } else {
                            LiveSourceState::Success
                        },
                        count,
                        "NASA EONET open-event records returned.",
                        None,
                    )],
                    vec![
                        "EONET aggregates reports from upstream sources; an open record is not a complete global incident list or a local emergency instruction.".into(),
                        "Use official local emergency authorities for safety decisions.".into(),
                    ],
                ))
            }
            Err(error) => Ok(response(
                "natural_hazards",
                Vec::new(),
                vec![status(
                    "nasa_eonet",
                    LiveSourceState::Failed,
                    0,
                    error,
                    None,
                )],
                vec!["No natural-hazard event was inferred when EONET was unavailable.".into()],
            )),
        },
        _ => {
            Err("kind must be weather, air_quality, marine, earthquakes, or natural_hazards".into())
        }
    }
}

fn currency_code(value: Option<&str>, field: &str) -> Result<String, String> {
    let code = value.unwrap_or_default().trim().to_uppercase();
    if code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        Ok(code)
    } else {
        Err(format!("{field} must be a three-letter currency code"))
    }
}

fn asset_code(value: Option<&str>, field: &str) -> Result<String, String> {
    let code = value.unwrap_or_default().trim().to_uppercase();
    if (2..=10).contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Ok(code)
    } else {
        Err(format!(
            "{field} must be a 2-10 character asset/currency code"
        ))
    }
}

async fn exchange_rate(base: &str, quote: &str) -> Result<(Vec<Value>, Option<String>), String> {
    let payload: Value = timed(
        "Frankfurter/ECB reference rates",
        response_json(
            "Frankfurter/ECB reference rates",
            HTTP.get("https://api.frankfurter.app/latest")
                .query(&[("from", base), ("to", quote)]),
        ),
    )
    .await?;
    let date = value_string(payload.get("date"));
    let rate = payload
        .pointer(&format!("/rates/{quote}"))
        .and_then(Value::as_f64);
    Ok((
        rate.map(|rate| {
            vec![json!({
                "source": "frankfurter_ecb_reference_rates",
                "base": base,
                "quote": quote,
                "rate": rate,
                "rate_date": date,
            })]
        })
        .unwrap_or_default(),
        date,
    ))
}

async fn coinbase_spot(asset: &str, quote: &str) -> Result<Vec<Value>, String> {
    let pair = format!("{asset}-{quote}");
    let url = format!("https://api.coinbase.com/v2/prices/{pair}/spot");
    let payload: Value = response_json("Coinbase spot", HTTP.get(url)).await?;
    let amount = value_string(payload.pointer("/data/amount"));
    Ok(amount
        .map(|amount| {
            vec![json!({
                "source": "coinbase_spot",
                "base": payload.pointer("/data/base"),
                "quote": payload.pointer("/data/currency"),
                "price": amount,
                "provider_observed_at": null,
            })]
        })
        .unwrap_or_default())
}

fn kraken_asset(code: &str) -> &str {
    match code {
        "BTC" => "XBT",
        "DOGE" => "XDG",
        other => other,
    }
}

async fn kraken_ticker(asset: &str, quote: &str) -> Result<Vec<Value>, String> {
    let pair = format!("{}{}", kraken_asset(asset), quote);
    let payload: Value = response_json(
        "Kraken ticker",
        HTTP.get("https://api.kraken.com/0/public/Ticker")
            .query(&[("pair", pair)]),
    )
    .await?;
    if payload
        .get("error")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
    {
        return Err(format!(
            "Kraken returned errors: {}",
            payload.get("error").unwrap_or(&Value::Null)
        ));
    }
    let Some(ticker) = payload
        .get("result")
        .and_then(Value::as_object)
        .and_then(|result| result.values().next())
    else {
        return Ok(Vec::new());
    };
    Ok(vec![json!({
        "source": "kraken_ticker",
        "base": asset,
        "quote": quote,
        "last_trade_price": ticker.pointer("/c/0"),
        "best_ask": ticker.pointer("/a/0"),
        "best_bid": ticker.pointer("/b/0"),
        "volume_today": ticker.pointer("/v/0"),
        "volume_24h": ticker.pointer("/v/1"),
        "low_today": ticker.pointer("/l/0"),
        "high_today": ticker.pointer("/h/0"),
        "provider_observed_at": null,
    })])
}

#[tauri::command]
pub async fn live_markets(
    kind: String,
    base: Option<String>,
    quote: Option<String>,
) -> Result<LiveDataResponse, String> {
    let kind = kind.trim().to_lowercase();
    match kind.as_str() {
        "exchange_rate" | "fx" => {
            let base = currency_code(base.as_deref(), "base")?;
            let quote = currency_code(quote.as_deref(), "quote")?;
            match exchange_rate(&base, &quote).await {
                Ok((records, date)) => {
                    let count = records.len();
                    Ok(response(
                        "exchange_rate",
                        records,
                        vec![status(
                            "frankfurter_ecb_reference_rates",
                            if count == 0 {
                                LiveSourceState::Empty
                            } else {
                                LiveSourceState::Success
                            },
                            count,
                            "Latest published reference rate returned.",
                            date,
                        )],
                        vec![
                            "Frankfurter exposes daily reference rates based on central-bank data; this is not an intraday executable FX quote.".into(),
                            "Weekends and holidays can return the most recent publication date rather than today's date.".into(),
                        ],
                    ))
                }
                Err(error) => Ok(response(
                    "exchange_rate",
                    Vec::new(),
                    vec![status(
                        "frankfurter_ecb_reference_rates",
                        LiveSourceState::Failed,
                        0,
                        error,
                        None,
                    )],
                    vec!["No exchange rate was inferred or copied from a search result.".into()],
                )),
            }
        }
        "crypto" => {
            let base = asset_code(base.as_deref(), "base")?;
            let quote = asset_code(quote.as_deref(), "quote")?;
            let coinbase = timed("Coinbase spot", coinbase_spot(&base, &quote));
            let kraken = timed("Kraken ticker", kraken_ticker(&base, &quote));
            let (coinbase, kraken) = tokio::join!(coinbase, kraken);
            let mut records = Vec::new();
            let mut statuses = Vec::new();
            for (source, result) in [("coinbase_spot", coinbase), ("kraken_ticker", kraken)] {
                match result {
                    Ok(items) if items.is_empty() => statuses.push(status(
                        source,
                        LiveSourceState::Empty,
                        0,
                        "The public endpoint returned no quote for this pair.",
                        None,
                    )),
                    Ok(items) => {
                        statuses.push(status(
                            source,
                            LiveSourceState::Success,
                            items.len(),
                            "An exchange-specific public quote was returned.",
                            None,
                        ));
                        records.extend(items);
                    }
                    Err(error) => {
                        statuses.push(status(source, LiveSourceState::Failed, 0, error, None))
                    }
                }
            }
            Ok(response(
                "crypto",
                records,
                statuses,
                vec![
                    "Coinbase and Kraken are exchange-specific quotes, not a universal market price; provider disagreement must remain visible and must not be silently averaged.".into(),
                    "These public responses do not include a trustworthy provider quote timestamp, so retrieved_at only states when the IDE received them.".into(),
                    "Quotes are informational and are not proof of executable price, liquidity, fees, or investment suitability.".into(),
                ],
            ))
        }
        _ => Err("kind must be exchange_rate or crypto".into()),
    }
}

fn flight_record(state: &[Value], feed_time: Option<i64>) -> Option<Value> {
    let longitude = state.get(5).and_then(Value::as_f64)?;
    let latitude = state.get(6).and_then(Value::as_f64)?;
    Some(json!({
        "source": "opensky_network",
        "feed_time_unix": feed_time,
        "icao24": state.first(),
        "callsign": state.get(1).and_then(Value::as_str).map(str::trim),
        "origin_country": state.get(2),
        "time_position_unix": state.get(3),
        "last_contact_unix": state.get(4),
        "longitude": longitude,
        "latitude": latitude,
        "barometric_altitude_m": state.get(7),
        "on_ground": state.get(8),
        "velocity_mps": state.get(9),
        "true_track_degrees": state.get(10),
        "vertical_rate_mps": state.get(11),
        "geo_altitude_m": state.get(13),
        "squawk": state.get(14),
        "special_purpose_indicator": state.get(15),
        "position_source": state.get(16),
        "aircraft_category": state.get(17),
    }))
}

#[tauri::command]
pub async fn live_flights(
    latitude: f64,
    longitude: f64,
    radius_km: Option<u32>,
    limit: Option<u32>,
) -> Result<LiveDataResponse, String> {
    let (latitude, longitude) = coordinates(Some(latitude), Some(longitude))?;
    let radius_km = radius_km.unwrap_or(100).clamp(1, 500) as f64;
    let latitude_delta = radius_km / 111.0;
    let longitude_scale = latitude.to_radians().cos().abs().max(0.1);
    let longitude_delta = radius_km / (111.0 * longitude_scale);
    let lamin = (latitude - latitude_delta).max(-90.0);
    let lamax = (latitude + latitude_delta).min(90.0);
    let lomin = (longitude - longitude_delta).max(-180.0);
    let lomax = (longitude + longitude_delta).min(180.0);
    let query = [
        ("lamin", lamin.to_string()),
        ("lamax", lamax.to_string()),
        ("lomin", lomin.to_string()),
        ("lomax", lomax.to_string()),
    ];
    let result: Result<Value, String> = timed(
        "OpenSky Network",
        response_json(
            "OpenSky Network",
            HTTP.get("https://opensky-network.org/api/states/all")
                .query(&query),
        ),
    )
    .await;
    match result {
        Ok(payload) => {
            let feed_time = payload.get("time").and_then(Value::as_i64);
            let limit = bounded_limit(limit, 20);
            let records: Vec<Value> = payload
                .get("states")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_array)
                .filter_map(|state| flight_record(state, feed_time))
                .take(limit)
                .collect();
            let count = records.len();
            Ok(response(
                "flights",
                records,
                vec![status(
                    "opensky_network",
                    if count == 0 {
                        LiveSourceState::Empty
                    } else {
                        LiveSourceState::Success
                    },
                    count,
                    "Anonymous OpenSky state vectors returned for the computed bounding box.",
                    feed_time.map(|value| format!("unix:{value}")),
                )],
                vec![
                    "OpenSky anonymous access is rate-limited and coverage depends on contributing receivers; missing aircraft are expected.".into(),
                    "State vectors are observations, not schedules, ticket status, safety confirmation, or proof of a flight's final route.".into(),
                    "The radius is converted to an approximate latitude/longitude bounding box; results are not route-distance filtered.".into(),
                ],
            ))
        }
        Err(error) => Ok(response(
            "flights",
            Vec::new(),
            vec![status(
                "opensky_network",
                LiveSourceState::Failed,
                0,
                error,
                None,
            )],
            vec!["No aircraft state was inferred when OpenSky was unavailable.".into()],
        )),
    }
}

#[derive(Clone, Copy)]
struct RoadCoverage {
    min_latitude: f64,
    max_latitude: f64,
    min_longitude: f64,
    max_longitude: f64,
}

const WINNIPEG_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 49.70,
    max_latitude: 50.10,
    min_longitude: -97.45,
    max_longitude: -96.80,
};
const NEW_YORK_CITY_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 40.45,
    max_latitude: 40.95,
    min_longitude: -74.30,
    max_longitude: -73.65,
};
const CHICAGO_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 41.60,
    max_latitude: 42.10,
    min_longitude: -87.95,
    max_longitude: -87.50,
};
const AUSTIN_TRAVIS_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 29.80,
    max_latitude: 30.75,
    min_longitude: -98.35,
    max_longitude: -97.20,
};
const CALGARY_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 50.75,
    max_latitude: 51.35,
    min_longitude: -114.40,
    max_longitude: -113.70,
};
const LONDON_COVERAGE: RoadCoverage = RoadCoverage {
    min_latitude: 51.28,
    max_latitude: 51.70,
    min_longitude: -0.55,
    max_longitude: 0.35,
};

// Coarse offline gate for the statewide CHP feed. It prevents irrelevant
// network calls; it is not a claim that CHP publishes every road in California.
const CALIFORNIA_CHP_ROUGH_GATE: &[(f64, f64)] = &[
    (42.01, -124.41),
    (42.01, -120.00),
    (39.00, -120.00),
    (35.00, -114.63),
    (34.50, -114.38),
    (34.08, -114.44),
    (33.41, -114.73),
    (32.72, -114.72),
    (32.53, -117.13),
    (33.00, -117.30),
    (33.70, -118.50),
    (34.40, -120.50),
    (35.80, -121.50),
    (36.80, -122.00),
    (37.80, -122.50),
    (38.90, -123.80),
    (40.00, -124.40),
    (41.50, -124.20),
];

const FINLAND_MAINLAND_GATE: &[(f64, f64)] = &[
    (59.75, 22.50),
    (59.85, 24.00),
    (60.20, 26.50),
    (60.50, 27.80),
    (61.50, 29.50),
    (62.90, 31.60),
    (64.00, 30.70),
    (65.70, 29.70),
    (66.90, 29.40),
    (68.00, 28.80),
    (69.00, 29.30),
    (70.10, 27.90),
    (69.90, 25.50),
    (69.10, 21.00),
    (67.00, 23.50),
    (65.00, 24.10),
    (64.00, 22.80),
    (63.10, 21.20),
    (62.00, 21.00),
    (61.00, 21.00),
    (60.00, 21.00),
];

const NORWAY_MAINLAND_GATE: &[(f64, f64)] = &[
    (57.70, 7.00),
    (58.00, 11.80),
    (59.20, 11.80),
    (60.50, 12.60),
    (62.00, 12.30),
    (63.00, 12.20),
    (64.00, 14.20),
    (65.00, 14.50),
    (66.00, 16.00),
    (67.00, 16.50),
    (68.00, 18.00),
    (68.80, 19.50),
    (69.00, 21.00),
    (69.80, 24.00),
    (70.00, 28.00),
    (69.70, 30.90),
    (70.40, 31.10),
    (71.20, 28.00),
    (71.30, 25.00),
    (70.80, 20.00),
    (69.50, 17.00),
    (68.00, 13.00),
    (66.00, 11.00),
    (64.00, 9.00),
    (62.00, 5.00),
    (60.00, 4.50),
    (58.00, 6.00),
];

const BC_BOUNDARY_SIMPLIFICATION_MARGIN_KM: f64 = 10.0;

// Statistics Canada 2021 Digital Boundary, PRUID=59, generalized by the
// official ArcGIS service with maxAllowableOffset=0.05 degrees. The 10 km
// coverage margin below is conservative relative to that simplification.
// Source: https://geo.statcan.gc.ca/geo_wa/rest/services/2021/Digital_boundary_files/MapServer/0
// Contains information licensed under the Open Government Licence - Canada.
const BRITISH_COLUMBIA_DIGITAL_BOUNDARY_GATE: &[(f64, f64)] = &[
    (60.00006, -135.40002),
    (60.00004, -139.05222),
    (59.90624, -138.70581),
    (59.75769, -138.60545),
    (59.24073, -137.60209),
    (58.9066, -137.52675),
    (59.15981, -136.82469),
    (59.16555, -136.58201),
    (59.28457, -136.46752),
    (59.4642, -136.47627),
    (59.52672, -136.23631),
    (59.60068, -136.35573),
    (59.79862, -135.47903),
    (59.69595, -135.23473),
    (59.56392, -135.02773),
    (59.42701, -135.09755),
    (59.38788, -134.98927),
    (59.34836, -135.03022),
    (59.28109, -134.95896),
    (59.13339, -134.4848),
    (58.85908, -134.25645),
    (58.73015, -133.84135),
    (58.43256, -133.38182),
    (58.38988, -133.46129),
    (57.21202, -132.24464),
    (57.09143, -132.36933),
    (57.04405, -132.04595),
    (56.86691, -132.12117),
    (56.80625, -131.87078),
    (56.59878, -131.83365),
    (56.61238, -131.58177),
    (56.24148, -130.46513),
    (56.13888, -130.41974),
    (56.12203, -130.10372),
    (55.91156, -130.00275),
    (55.70479, -130.1717),
    (55.28493, -129.97403),
    (54.96634, -130.28375),
    (54.76395, -130.65908),
    (54.7085, -130.6168),
    (54.64592, -133.24398),
    (53.7201, -133.23379),
    (52.99427, -132.95691),
    (52.75001, -132.75829),
    (52.75, -132.50002),
    (52.56015, -132.50002),
    (52.22335, -132.06718),
    (51.89069, -131.07119),
    (51.9691, -130.45698),
    (52.89735, -131.04001),
    (53.08427, -131.00718),
    (53.36644, -130.69981),
    (53.13611, -130.37632),
    (52.98353, -129.94538),
    (52.46889, -129.44734),
    (52.36352, -128.80439),
    (52.19326, -128.7493),
    (52.00007, -128.82222),
    (52.00006, -129.06814),
    (51.8211, -129.00002),
    (51.20053, -129.00002),
    (50.75502, -129.10194),
    (50.27509, -128.57054),
    (49.99654, -127.9847),
    (49.55263, -127.45209),
    (49.54045, -126.61447),
    (49.348, -126.59401),
    (49.25103, -126.25149),
    (48.92565, -125.79979),
    (48.59695, -125.09714),
    (48.50001, -124.4995),
    (48.25284, -123.75569),
    (48.25001, -123.25002),
    (48.42312, -123.11382),
    (48.69357, -123.26838),
    (48.76711, -123.00851),
    (48.83122, -123.0085),
    (49.00007, -123.31857),
    (49.02431, -114.05463),
    (49.39611, -114.5906),
    (49.55747, -114.57362),
    (49.57639, -114.7326),
    (49.73037, -114.63366),
    (50.06491, -114.66248),
    (50.12036, -114.73601),
    (50.35879, -114.76864),
    (50.5824, -115.01513),
    (50.52783, -115.20815),
    (50.72393, -115.31231),
    (50.83797, -115.63937),
    (50.89426, -115.5621),
    (50.98088, -115.62829),
    (51.31194, -116.26803),
    (51.4565, -116.28402),
    (51.66184, -116.59503),
    (51.80519, -116.65908),
    (51.70981, -116.92058),
    (51.88969, -117.01079),
    (52.07432, -117.30508),
    (52.19405, -117.31732),
    (52.14427, -117.61128),
    (52.22636, -117.81833),
    (52.36535, -117.70581),
    (52.50024, -117.988),
    (52.39847, -118.04418),
    (52.37418, -118.21935),
    (52.44954, -118.25535),
    (52.47783, -118.19337),
    (52.61065, -118.35252),
    (52.6777, -118.29016),
    (52.77579, -118.42243),
    (52.84713, -118.387),
    (52.88373, -118.61385),
    (53.0348, -118.65536),
    (53.04536, -118.77622),
    (53.11548, -118.72954),
    (53.15923, -118.78654),
    (53.24166, -118.97521),
    (53.12672, -119.02458),
    (53.17663, -119.25713),
    (53.36081, -119.39075),
    (53.36783, -119.66892),
    (53.51913, -119.89949),
    (53.61454, -119.92591),
    (53.61459, -119.71342),
    (53.80623, -120.00002),
    (60.00001, -120.00002),
    (60.00006, -135.40002),
];

impl RoadCoverage {
    fn may_cover(self, latitude: f64, longitude: f64, radius_km: f64) -> bool {
        let latitude_margin = radius_km / 111.0;
        let longitude_margin = radius_km / (111.0 * latitude.to_radians().cos().abs().max(0.1));
        latitude >= self.min_latitude - latitude_margin
            && latitude <= self.max_latitude + latitude_margin
            && longitude >= self.min_longitude - longitude_margin
            && longitude <= self.max_longitude + longitude_margin
    }
}

fn point_in_polygon(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let crosses = (current.0 > point.0) != (previous.0 > point.0)
            && point.1
                < (previous.1 - current.1) * (point.0 - current.0) / (previous.0 - current.0)
                    + current.1;
        if crosses {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn polygon_may_cover(point: (f64, f64), radius_km: f64, polygon: &[(f64, f64)]) -> bool {
    if point_in_polygon(point, polygon) {
        return true;
    }
    if polygon.is_empty() {
        return false;
    }
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let vertex_distance = haversine_km(point.0, point.1, current.0, current.1);
        if vertex_distance <= radius_km {
            return true;
        }
        if let Some((latitude, longitude)) =
            nearest_interior_segment_point(point, previous, current)
        {
            if haversine_km(point.0, point.1, latitude, longitude) <= radius_km {
                return true;
            }
        }
        previous = current;
    }
    false
}

fn drivebc_may_cover(center: (f64, f64), radius_km: f64) -> bool {
    polygon_may_cover(
        center,
        radius_km + BC_BOUNDARY_SIMPLIFICATION_MARGIN_KM,
        BRITISH_COLUMBIA_DIGITAL_BOUNDARY_GATE,
    )
}

struct RoadProviderData {
    records: Vec<Value>,
    data_as_of: Option<String>,
    data_as_of_kind: Option<String>,
    stale: bool,
    delayed: bool,
    stale_after_minutes: Option<i64>,
    cache_state: Option<String>,
    cache_age_seconds: Option<u64>,
    timezone: Tz,
}

fn road_provider_data(
    records: Vec<Value>,
    data_as_of: Option<String>,
    data_as_of_kind: &'static str,
    stale: bool,
    timezone: Tz,
) -> RoadProviderData {
    RoadProviderData {
        records,
        data_as_of,
        data_as_of_kind: Some(data_as_of_kind.into()),
        stale,
        delayed: false,
        stale_after_minutes: None,
        cache_state: Some("bypassed".into()),
        cache_age_seconds: None,
        timezone,
    }
}

fn time_sensitive_road_provider_data(
    records: Vec<Value>,
    data_as_of: Option<String>,
    data_as_of_kind: &'static str,
    timezone: Tz,
    stale_after_minutes: i64,
    observation_candidate_present: bool,
) -> RoadProviderData {
    time_sensitive_road_provider_data_at(
        records,
        data_as_of,
        data_as_of_kind,
        timezone,
        stale_after_minutes,
        observation_candidate_present,
        Utc::now(),
    )
}

#[allow(clippy::too_many_arguments)]
fn time_sensitive_road_provider_data_at(
    mut records: Vec<Value>,
    data_as_of: Option<String>,
    data_as_of_kind: &'static str,
    timezone: Tz,
    stale_after_minutes: i64,
    observation_candidate_present: bool,
    now: DateTime<Utc>,
) -> RoadProviderData {
    let window = observation_window_at(data_as_of.as_deref(), timezone, stale_after_minutes, now);
    let stale = match window.state {
        ObservationWindowState::Stale | ObservationWindowState::Future => true,
        ObservationWindowState::MissingOrInvalid => {
            observation_candidate_present || !records.is_empty() || data_as_of.is_some()
        }
        ObservationWindowState::NearRealTime | ObservationWindowState::Delayed => false,
    };
    if stale {
        records.clear();
    }
    let delayed = !records.is_empty() && window.state == ObservationWindowState::Delayed;
    RoadProviderData {
        records,
        data_as_of,
        data_as_of_kind: Some(data_as_of_kind.into()),
        stale,
        delayed,
        stale_after_minutes: Some(stale_after_minutes),
        cache_state: Some("bypassed".into()),
        cache_age_seconds: None,
        timezone,
    }
}

async fn cached_road_directory<F>(
    cache: &'static RwLock<Option<CachedDirectory>>,
    ttl: Duration,
    fetch: F,
) -> Result<(Value, &'static str, u64), String>
where
    F: Future<Output = Result<Value, String>>,
{
    let cached = {
        let guard = cache.read().await;
        guard
            .as_ref()
            .map(|entry| (entry.value.clone(), entry.stored_at.elapsed()))
    };
    if let Some((value, age)) = cached.as_ref() {
        if *age <= ttl {
            return Ok((
                value.clone(),
                "directory_hit_dynamic_bypassed",
                age.as_secs(),
            ));
        }
    }

    // Only one caller refreshes an expired directory. Recheck after taking the
    // write lock because another caller may have completed the fetch meanwhile.
    let mut guard = cache.write().await;
    if let Some(entry) = guard.as_ref() {
        let age = entry.stored_at.elapsed();
        if age <= ttl {
            return Ok((
                entry.value.clone(),
                "directory_hit_dynamic_bypassed",
                age.as_secs(),
            ));
        }
    }
    match fetch.await {
        Ok(value) => {
            *guard = Some(CachedDirectory {
                value: value.clone(),
                stored_at: Instant::now(),
            });
            Ok((value, "directory_miss_dynamic_bypassed", 0))
        }
        Err(error) => Err(error),
    }
}

fn rounded_distance_km(distance: f64) -> f64 {
    (distance * 100.0).round() / 100.0
}

fn geometry_coordinate(value: &Value) -> Option<(f64, f64)> {
    let values = value.as_array()?;
    let longitude = values.first()?.as_f64()?;
    let latitude = values.get(1)?.as_f64()?;
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude))
        .then_some((latitude, longitude))
}

fn collect_geometry_paths(value: &Value, paths: &mut Vec<Vec<(f64, f64)>>) {
    let Some(values) = value.as_array() else {
        return;
    };
    let path = values
        .iter()
        .map(geometry_coordinate)
        .collect::<Option<Vec<_>>>();
    if let Some(path) = path.filter(|path| !path.is_empty()) {
        paths.push(path);
        return;
    }
    for value in values {
        collect_geometry_paths(value, paths);
    }
}

fn geometry_ring(value: &Value) -> Option<Vec<(f64, f64)>> {
    let ring = value
        .as_array()?
        .iter()
        .map(geometry_coordinate)
        .collect::<Option<Vec<_>>>()?;
    (ring.len() >= 3).then_some(ring)
}

fn polygon_coordinates_contain(coordinates: &Value, point: (f64, f64)) -> bool {
    let Some(rings) = coordinates
        .as_array()
        .and_then(|rings| rings.iter().map(geometry_ring).collect::<Option<Vec<_>>>())
    else {
        return false;
    };
    let Some(exterior) = rings.first() else {
        return false;
    };
    point_in_polygon(point, exterior)
        && !rings
            .iter()
            .skip(1)
            .any(|hole| point_in_polygon(point, hole))
}

fn reported_polygon_contains(
    geometry_type: Option<&str>,
    coordinates: &Value,
    point: (f64, f64),
) -> bool {
    match geometry_type {
        Some("Polygon") => polygon_coordinates_contain(coordinates, point),
        Some("MultiPolygon") => coordinates.as_array().is_some_and(|polygons| {
            polygons
                .iter()
                .any(|polygon| polygon_coordinates_contain(polygon, point))
        }),
        _ => false,
    }
}

fn nearest_interior_segment_point(
    center: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
) -> Option<(f64, f64)> {
    let longitude_scale = center.0.to_radians().cos().abs().max(0.1);
    let start_x = (start.1 - center.1) * longitude_scale;
    let start_y = start.0 - center.0;
    let end_x = (end.1 - center.1) * longitude_scale;
    let end_y = end.0 - center.0;
    let delta_x = end_x - start_x;
    let delta_y = end_y - start_y;
    let denominator = delta_x * delta_x + delta_y * delta_y;
    if denominator <= f64::EPSILON {
        return None;
    }
    let projection = (-(start_x * delta_x + start_y * delta_y) / denominator).clamp(0.0, 1.0);
    if projection <= f64::EPSILON || projection >= 1.0 - f64::EPSILON {
        return None;
    }
    Some((
        start.0 + projection * (end.0 - start.0),
        start.1 + projection * (end.1 - start.1),
    ))
}

fn geometry_distance(
    geometry: Option<&Value>,
    center: (f64, f64),
) -> Option<(f64, f64, f64, &'static str)> {
    let geometry = geometry?;
    let geometry_type = geometry.get("type").and_then(Value::as_str);
    if geometry_type == Some("Point") {
        let (latitude, longitude) = geometry_coordinate(geometry.get("coordinates")?)?;
        return Some((
            haversine_km(center.0, center.1, latitude, longitude),
            latitude,
            longitude,
            "point",
        ));
    }
    let mut paths = Vec::new();
    let coordinates = geometry.get("coordinates")?;
    if reported_polygon_contains(geometry_type, coordinates, center) {
        return Some((0.0, center.0, center.1, "inside_reported_polygon"));
    }
    if geometry_type == Some("MultiPoint") {
        for value in coordinates.as_array()? {
            if let Some(point) = geometry_coordinate(value) {
                paths.push(vec![point]);
            }
        }
    } else {
        collect_geometry_paths(coordinates, &mut paths);
    }
    let mut nearest: Option<(f64, f64, f64, &'static str)> = None;
    let mut consider = |latitude: f64, longitude: f64, basis: &'static str| {
        let candidate = (
            haversine_km(center.0, center.1, latitude, longitude),
            latitude,
            longitude,
            basis,
        );
        if nearest
            .as_ref()
            .is_none_or(|current| candidate.0 < current.0)
        {
            nearest = Some(candidate);
        }
    };
    for path in paths {
        for &(latitude, longitude) in &path {
            consider(latitude, longitude, "nearest_geometry_vertex");
        }
        for segment in path.windows(2) {
            if let Some((latitude, longitude)) =
                nearest_interior_segment_point(center, segment[0], segment[1])
            {
                consider(latitude, longitude, "nearest_geometry_segment");
            }
        }
    }
    nearest
}

fn nyc_polyline_distance(points: &str, center: (f64, f64)) -> Option<(f64, f64, f64)> {
    let points = points
        .split_whitespace()
        .filter_map(|point| {
            let (latitude, longitude) = point.split_once(',')?;
            let latitude = latitude.trim().parse::<f64>().ok()?;
            let longitude = longitude.trim().parse::<f64>().ok()?;
            (latitude.is_finite()
                && longitude.is_finite()
                && NEW_YORK_CITY_COVERAGE.may_cover(latitude, longitude, 5.0))
            .then_some((latitude, longitude))
        })
        .collect::<Vec<_>>();
    if points.len() < 2 {
        return None;
    }
    points
        .iter()
        .copied()
        .filter(|point| {
            points.iter().copied().any(|other| {
                point != &other && haversine_km(point.0, point.1, other.0, other.1) <= 20.0
            })
        })
        .map(|(latitude, longitude)| {
            (
                haversine_km(center.0, center.1, latitude, longitude),
                latitude,
                longitude,
            )
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
}

fn parse_winnipeg_counts(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    let mut returned_data_as_of = None;
    let mut rejected_data_as_of = None;
    let mut seen_sites = HashSet::new();
    for item in payload {
        let measurement_time = value_string(item.get("timestamp"));
        if !observation_is_usable(
            measurement_time.as_deref(),
            chrono_tz::America::Winnipeg,
            WINNIPEG_OBSERVATION_MAX_AGE_MINUTES,
        ) {
            update_latest(&mut rejected_data_as_of, measurement_time);
            continue;
        }
        update_latest(&mut data_as_of, measurement_time.clone());
        let Some(site) = value_string(item.get("site")) else {
            continue;
        };
        if seen_sites.contains(&site) {
            continue;
        }
        let Some(latitude) = value_f64(item.get("latitude")) else {
            continue;
        };
        let Some(longitude) = value_f64(item.get("longitude")) else {
            continue;
        };
        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
        if distance_km > radius_km {
            continue;
        }
        let Some(vehicle_count) = value_i64(item.get("total")) else {
            continue;
        };
        let (record_freshness, record_time_age_seconds) = timestamp_freshness(
            measurement_time.as_deref(),
            chrono_tz::America::Winnipeg,
            WINNIPEG_OBSERVATION_MAX_AGE_MINUTES,
        );
        seen_sites.insert(site.clone());
        update_latest(&mut returned_data_as_of, measurement_time.clone());
        records.push(json!({
            "record_type": "vehicle_count_observation",
            "source": "winnipeg_permanent_count_stations",
            "fact_kind": "sensor_observation",
            "station_name": site,
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "station_point_haversine",
            "measurement_time_local": measurement_time,
            "provider_timezone": "America/Winnipeg",
            "record_freshness": record_freshness,
            "record_time_age_seconds": record_time_age_seconds,
            "vehicle_count": vehicle_count,
            "count_unit": "provider_vehicle_count_for_timestamp_interval",
            "is_simultaneous_nearby_vehicle_count": false,
            "count_interval_duration": null,
            "count_interval_boundary": "not_documented_in_provider_schema",
            "directional_counts": {
                "northbound": value_i64(item.get("northbound")),
                "southbound": value_i64(item.get("southbound")),
                "eastbound": value_i64(item.get("eastbound")),
                "westbound": value_i64(item.get("westbound")),
                "left_sensor": value_i64(item.get("left")),
                "right_sensor": value_i64(item.get("right")),
            },
        }));
        if records.len() >= limit {
            break;
        }
    }
    let source_data_as_of = if records.is_empty() {
        data_as_of.or(rejected_data_as_of)
    } else {
        returned_data_as_of
    };
    time_sensitive_road_provider_data(
        records,
        source_data_as_of,
        "observation_time",
        chrono_tz::America::Winnipeg,
        WINNIPEG_OBSERVATION_MAX_AGE_MINUTES,
        !payload.is_empty(),
    )
}

async fn winnipeg_vehicle_counts(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = vec![
        ("$select", "timestamp,site,`right`,`left`,northbound,southbound,eastbound,westbound,total,latitude,longitude".into()),
        ("$where", format!("within_circle(location,{:.5},{:.5},{:.0})", center.0, center.1, radius_km * 1000.0)),
        ("$order", "timestamp DESC".into()),
        ("$limit", "500".into()),
    ];
    let payload: Vec<Value> = response_json(
        "City of Winnipeg permanent count stations",
        HTTP.get("https://data.winnipeg.ca/resource/46sc-6jrs.json")
            .query(&query),
    )
    .await?;
    Ok(parse_winnipeg_counts(&payload, center, radius_km, limit))
}

#[derive(Debug, Clone)]
struct NearbyRoadStation {
    id: String,
    name: String,
    latitude: f64,
    longitude: f64,
    distance_km: f64,
}

fn nearest_fintraffic_station(
    payload: &Value,
    center: (f64, f64),
    radius_km: f64,
) -> Option<NearbyRoadStation> {
    payload
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|feature| {
            feature
                .pointer("/properties/collectionStatus")
                .and_then(Value::as_str)
                == Some("GATHERING")
        })
        .filter_map(|feature| {
            let id = value_string(feature.get("id"))
                .or_else(|| value_string(feature.pointer("/properties/id")))?;
            let name = value_string(feature.pointer("/properties/name"))
                .unwrap_or_else(|| format!("TMS {id}"));
            let (distance_km, latitude, longitude, _) =
                geometry_distance(feature.get("geometry"), center)?;
            (distance_km <= radius_km).then_some(NearbyRoadStation {
                id,
                name,
                latitude,
                longitude,
                distance_km,
            })
        })
        .min_by(|left, right| left.distance_km.total_cmp(&right.distance_km))
}

fn parse_fintraffic_tms_flow(
    payload: &Value,
    station: &NearbyRoadStation,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    let mut rejected_data_as_of = None;
    let mut observation_candidate_present = false;
    for sensor in payload
        .get("sensorValues")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(sensor_name) = sensor.get("name").and_then(Value::as_str) else {
            continue;
        };
        let unit = sensor.get("unit").and_then(Value::as_str);
        let metric =
            if sensor_name.starts_with("OHITUKSET_5MIN_KIINTEA_SUUNTA") && unit == Some("kpl/h") {
                "vehicle_flow_rate"
            } else if sensor_name.starts_with("KESKINOPEUS_60MIN_KIINTEA_SUUNTA")
                && unit == Some("km/h")
            {
                "average_speed"
            } else {
                continue;
            };
        let Some(value) = value_f64(sensor.get("value")).filter(|value| *value >= 0.0) else {
            continue;
        };
        observation_candidate_present = true;
        let measured_at = value_string(sensor.get("measuredTime"));
        if !observation_is_usable(
            measured_at.as_deref(),
            chrono_tz::UTC,
            FINTRAFFIC_OBSERVATION_MAX_AGE_MINUTES,
        ) {
            update_latest(&mut rejected_data_as_of, measured_at);
            continue;
        }
        update_latest(&mut data_as_of, measured_at.clone());
        let (record_freshness, record_time_age_seconds) = timestamp_freshness(
            measured_at.as_deref(),
            chrono_tz::UTC,
            FINTRAFFIC_OBSERVATION_MAX_AGE_MINUTES,
        );
        records.push(json!({
            "record_type": "traffic_flow_observation",
            "source": "fintraffic_tms_sensor_values",
            "fact_kind": "sensor_observation",
            "station_id": station.id,
            "station_name": station.name,
            "latitude": station.latitude,
            "longitude": station.longitude,
            "distance_km": rounded_distance_km(station.distance_km),
            "distance_basis": "station_point_haversine",
            "sensor_id": sensor.get("id"),
            "provider_sensor_name": sensor_name,
            "metric": metric,
            "flow_rate_vehicles_per_hour": (metric == "vehicle_flow_rate").then_some(value),
            "average_speed_kmh": (metric == "average_speed").then_some(value),
            "vehicle_count": null,
            "is_simultaneous_nearby_vehicle_count": false,
            "provider_unit": unit,
            "time_window_start": sensor.get("timeWindowStart"),
            "time_window_end": sensor.get("timeWindowEnd"),
            "measured_at": measured_at,
            "record_freshness": record_freshness,
            "record_time_age_seconds": record_time_age_seconds,
            "provider_data_updated_at": payload.get("dataUpdatedTime"),
            "source_url": format!("https://tie.digitraffic.fi/api/tms/v1/stations/{}/data", station.id),
            "attribution": "Source: Fintraffic / digitraffic.fi, license CC BY 4.0",
            "license_url": "https://www.digitraffic.fi/en/terms-of-service/",
        }));
        if records.len() >= limit {
            break;
        }
    }
    time_sensitive_road_provider_data(
        records,
        data_as_of.or(rejected_data_as_of),
        "observation_time",
        chrono_tz::UTC,
        FINTRAFFIC_OBSERVATION_MAX_AGE_MINUTES,
        observation_candidate_present,
    )
}

async fn fintraffic_tms_flow(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let (metadata, cache_state, cache_age_seconds) = cached_road_directory(
        &FINTRAFFIC_STATION_DIRECTORY,
        FINTRAFFIC_DIRECTORY_CACHE_TTL,
        async {
            let value: Value = response_json(
                "Fintraffic TMS station directory",
                HTTP.get("https://tie.digitraffic.fi/api/tms/v1/stations")
                    .header("Digitraffic-User", DIGITRAFFIC_USER),
            )
            .await?;
            if value.get("features").and_then(Value::as_array).is_none() {
                return Err("Fintraffic TMS station directory has no feature array".into());
            }
            Ok(value)
        },
    )
    .await?;
    let Some(station) = nearest_fintraffic_station(&metadata, center, radius_km) else {
        let mut data = road_provider_data(
            Vec::new(),
            value_string(metadata.get("dataUpdatedTime")),
            "feed_generated_at",
            false,
            chrono_tz::UTC,
        );
        data.cache_state = Some(cache_state.into());
        data.cache_age_seconds = Some(cache_age_seconds);
        return Ok(data);
    };
    let source = "Fintraffic TMS station data";
    let payload: Value = response_json(
        source,
        HTTP.get(format!(
            "https://tie.digitraffic.fi/api/tms/v1/stations/{}/data",
            station.id
        ))
        .header("Digitraffic-User", DIGITRAFFIC_USER),
    )
    .await?;
    let mut data = parse_fintraffic_tms_flow(&payload, &station, limit);
    data.cache_state = Some(cache_state.into());
    data.cache_age_seconds = Some(cache_age_seconds);
    Ok(data)
}

fn nearest_norway_station(
    payload: &Value,
    center: (f64, f64),
    radius_km: f64,
) -> Option<NearbyRoadStation> {
    payload
        .pointer("/data/trafficRegistrationPoints")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|station| {
            let id = value_string(station.get("id"))?;
            let name = value_string(station.get("name"))
                .unwrap_or_else(|| format!("Traffic registration point {id}"));
            let latitude = value_f64(station.pointer("/location/coordinates/latLon/lat"))?;
            let longitude = value_f64(station.pointer("/location/coordinates/latLon/lon"))?;
            let distance_km = haversine_km(center.0, center.1, latitude, longitude);
            (distance_km <= radius_km).then_some(NearbyRoadStation {
                id,
                name,
                latitude,
                longitude,
                distance_km,
            })
        })
        .min_by(|left, right| left.distance_km.total_cmp(&right.distance_km))
}

fn graphql_errors(payload: &Value) -> Option<String> {
    let errors = payload.get("errors")?.as_array()?;
    (!errors.is_empty()).then(|| {
        errors
            .iter()
            .filter_map(|error| value_string(error.get("message")))
            .take(3)
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn parse_norway_hourly_count(payload: &Value, station: &NearbyRoadStation) -> RoadProviderData {
    let edges = payload
        .pointer("/data/trafficData/volume/byHour/edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    let observation_candidate_present = edges.clone().next().is_some();
    let latest_provider_time = edges
        .clone()
        .filter_map(|edge| value_string(edge.pointer("/node/to")))
        .max();
    let latest = edges
        .filter_map(|edge| {
            let node = edge.get("node")?;
            let end = value_string(node.get("to"))?;
            let parsed = parse_provider_time(&end, chrono_tz::UTC)?;
            observation_is_usable(
                Some(&end),
                chrono_tz::UTC,
                NORWAY_OBSERVATION_MAX_AGE_MINUTES,
            )
            .then_some((parsed, node))
        })
        .max_by_key(|(parsed, _)| *parsed)
        .map(|(_, node)| node);
    let Some(node) = latest else {
        return time_sensitive_road_provider_data(
            Vec::new(),
            latest_provider_time,
            "aggregation_interval_end",
            chrono_tz::UTC,
            NORWAY_OBSERVATION_MAX_AGE_MINUTES,
            observation_candidate_present,
        );
    };
    let interval_end = value_string(node.get("to"));
    let Some(vehicle_count) = value_i64(node.pointer("/total/volumeNumbers/volume")) else {
        return time_sensitive_road_provider_data(
            Vec::new(),
            interval_end,
            "aggregation_interval_end",
            chrono_tz::UTC,
            NORWAY_OBSERVATION_MAX_AGE_MINUTES,
            observation_candidate_present,
        );
    };
    let (record_freshness, record_time_age_seconds) = timestamp_freshness(
        interval_end.as_deref(),
        chrono_tz::UTC,
        NORWAY_OBSERVATION_MAX_AGE_MINUTES,
    );
    let record = json!({
        "record_type": "vehicle_count_observation",
        "source": "norway_public_roads_traffic_data",
        "fact_kind": "delayed_aggregated_sensor_observation",
        "station_id": station.id,
        "station_name": station.name,
        "latitude": station.latitude,
        "longitude": station.longitude,
        "distance_km": rounded_distance_km(station.distance_km),
        "distance_basis": "station_point_haversine",
        "vehicle_count": vehicle_count,
        "count_unit": "vehicles_in_provider_hour_interval",
        "is_simultaneous_nearby_vehicle_count": false,
        "count_interval_duration": "PT1H",
        "count_interval_start": node.get("from"),
        "count_interval_end": node.get("to"),
        "record_freshness": record_freshness,
        "record_time_age_seconds": record_time_age_seconds,
        "coverage_percentage": node.pointer("/total/coverage/percentage"),
        "coverage_unit": node.pointer("/total/coverage/unit"),
        "publication_delay": "officially approximately 2 to 3 hours after collection",
        "vehicle_level_data_public": false,
        "source_url": "https://trafikkdata.atlas.vegvesen.no/om-api",
        "attribution": "Source: Norwegian Public Roads Administration, licensed under NLOD",
    });
    time_sensitive_road_provider_data(
        vec![record],
        interval_end,
        "aggregation_interval_end",
        chrono_tz::UTC,
        NORWAY_OBSERVATION_MAX_AGE_MINUTES,
        observation_candidate_present,
    )
}

async fn norway_vehicle_counts(
    center: (f64, f64),
    radius_km: f64,
) -> Result<RoadProviderData, String> {
    let endpoint = "https://trafikkdata-api.atlas.vegvesen.no";
    let station_query = "{ trafficRegistrationPoints(searchQuery:{isOperational:true,trafficType:VEHICLE}) { id name location { coordinates { latLon { lat lon } } } } }";
    let (stations, cache_state, cache_age_seconds) = cached_road_directory(
        &NORWAY_STATION_DIRECTORY,
        NORWAY_DIRECTORY_CACHE_TTL,
        async {
            let value: Value = response_json(
                "Norwegian Public Roads traffic station directory",
                HTTP.post(endpoint).json(&json!({ "query": station_query })),
            )
            .await?;
            if let Some(error) = graphql_errors(&value) {
                return Err(format!("Norwegian traffic station query failed: {error}"));
            }
            Ok(value)
        },
    )
    .await?;
    let Some(station) = nearest_norway_station(&stations, center, radius_km) else {
        let mut data = road_provider_data(
            Vec::new(),
            None,
            "aggregation_interval_end",
            false,
            chrono_tz::UTC,
        );
        data.cache_state = Some(cache_state.into());
        data.cache_age_seconds = Some(cache_age_seconds);
        return Ok(data);
    };
    if !station
        .id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Norwegian traffic station returned an invalid identifier".into());
    }
    let from = (Utc::now() - ChronoDuration::hours(12)).to_rfc3339();
    let to = Utc::now().to_rfc3339();
    let data_query = format!(
        "{{ trafficData(trafficRegistrationPointId:\"{}\") {{ volume {{ byHour(from:\"{from}\",to:\"{to}\",first:20) {{ edges {{ node {{ from to total {{ coverage {{ percentage unit }} volumeNumbers {{ volume }} }} }} }} pageInfo {{ hasNextPage }} }} }} }} }}",
        station.id
    );
    let payload: Value = response_json(
        "Norwegian Public Roads hourly vehicle counts",
        HTTP.post(endpoint).json(&json!({ "query": data_query })),
    )
    .await?;
    if let Some(error) = graphql_errors(&payload) {
        return Err(format!("Norwegian hourly count query failed: {error}"));
    }
    let mut data = parse_norway_hourly_count(&payload, &station);
    data.cache_state = Some(cache_state.into());
    data.cache_age_seconds = Some(cache_age_seconds);
    Ok(data)
}

fn parse_nyc_traffic_flow(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> RoadProviderData {
    let mut data_as_of = None;
    let mut returned_data_as_of = None;
    let mut rejected_data_as_of = None;
    let mut records = Vec::new();
    let mut latest_first = payload.iter().collect::<Vec<_>>();
    latest_first.sort_by(|left, right| {
        value_string(right.get("data_as_of")).cmp(&value_string(left.get("data_as_of")))
    });
    let mut seen_links = HashSet::new();
    for item in latest_first {
        let Some(link_id) =
            value_string(item.get("link_id")).or_else(|| value_string(item.get("id")))
        else {
            continue;
        };
        let observed_at = value_string(item.get("data_as_of"));
        if !observation_is_usable(
            observed_at.as_deref(),
            chrono_tz::America::New_York,
            NYC_OBSERVATION_MAX_AGE_MINUTES,
        ) {
            update_latest(&mut rejected_data_as_of, observed_at);
            continue;
        }
        if !seen_links.insert(link_id.clone()) {
            continue;
        }
        update_latest(&mut data_as_of, observed_at.clone());
        let Some(points) = item.get("link_points").and_then(Value::as_str) else {
            continue;
        };
        let Some((distance_km, latitude, longitude)) = nyc_polyline_distance(points, center) else {
            continue;
        };
        if distance_km > radius_km {
            continue;
        }
        let Some(speed) = value_f64(item.get("speed")).filter(|value| *value >= 0.0) else {
            continue;
        };
        let (record_freshness, record_time_age_seconds) = timestamp_freshness(
            observed_at.as_deref(),
            chrono_tz::America::New_York,
            NYC_OBSERVATION_MAX_AGE_MINUTES,
        );
        update_latest(&mut returned_data_as_of, observed_at.clone());
        records.push(json!({
            "record_type": "traffic_flow_observation",
            "source": "nyc_dot_traffic_speeds",
            "fact_kind": "provider_estimate",
            "link_id": link_id,
            "link_name": item.get("link_name"),
            "borough": item.get("borough"),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "nearest_validated_polyline_vertex",
            "speed_provider_value": speed,
            "speed_unit": "not_documented_in_dataset_schema",
            "travel_time_provider_value": value_i64(item.get("travel_time")),
            "travel_time_unit": "not_documented_in_dataset_schema",
            "provider_status_code": item.get("status"),
            "provider_status_interpretation": "raw; dataset schema does not document status-code semantics",
            "data_as_of_local": observed_at,
            "provider_timezone": "America/New_York",
            "record_freshness": record_freshness,
            "record_time_age_seconds": record_time_age_seconds,
            "vehicle_count": null,
        }));
        if records.len() >= limit {
            break;
        }
    }
    let source_data_as_of = if records.is_empty() {
        data_as_of.or(rejected_data_as_of)
    } else {
        returned_data_as_of
    };
    time_sensitive_road_provider_data(
        records,
        source_data_as_of,
        "observation_time",
        chrono_tz::America::New_York,
        NYC_OBSERVATION_MAX_AGE_MINUTES,
        !payload.is_empty(),
    )
}

async fn nyc_traffic_flow(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = [
        (
            "$select",
            "id,speed,travel_time,status,data_as_of,link_id,link_points,owner,borough,link_name",
        ),
        ("$order", "data_as_of DESC"),
        ("$limit", "1000"),
    ];
    let payload: Vec<Value> = response_json(
        "NYC DOT Traffic Speeds",
        HTTP.get("https://data.cityofnewyork.us/resource/i4gi-tjb9.json")
            .query(&query),
    )
    .await?;
    Ok(parse_nyc_traffic_flow(&payload, center, radius_km, limit))
}

fn parse_chicago_traffic_flow(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    let mut returned_data_as_of = None;
    let mut rejected_data_as_of = None;
    for item in payload {
        let updated_at = value_string(item.get("_last_updt"));
        if !observation_is_usable(
            updated_at.as_deref(),
            chrono_tz::America::Chicago,
            CHICAGO_FLOW_OBSERVATION_MAX_AGE_MINUTES,
        ) {
            update_latest(&mut rejected_data_as_of, updated_at);
            continue;
        }
        update_latest(&mut data_as_of, updated_at.clone());
        let Some(start_latitude) = value_f64(item.get("_lif_lat")) else {
            continue;
        };
        let Some(start_longitude) = value_f64(item.get("start_lon")) else {
            continue;
        };
        let end_latitude = value_f64(item.get("_lit_lat")).unwrap_or(start_latitude);
        let end_longitude = value_f64(item.get("_lit_lon")).unwrap_or(start_longitude);
        let latitude = (start_latitude + end_latitude) / 2.0;
        let longitude = (start_longitude + end_longitude) / 2.0;
        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
        if distance_km > radius_km {
            continue;
        }
        let Some(speed) = value_f64(item.get("_traffic")).filter(|value| *value >= 0.0) else {
            continue;
        };
        let (record_freshness, record_time_age_seconds) = timestamp_freshness(
            updated_at.as_deref(),
            chrono_tz::America::Chicago,
            CHICAGO_FLOW_OBSERVATION_MAX_AGE_MINUTES,
        );
        update_latest(&mut returned_data_as_of, updated_at.clone());
        records.push(json!({
            "record_type": "traffic_flow_observation",
            "source": "chicago_traffic_tracker_segments",
            "fact_kind": "provider_estimate",
            "segment_id": item.get("segmentid"),
            "street": item.get("street"),
            "direction": item.get("_direction"),
            "from_street": item.get("_fromst"),
            "to_street": item.get("_tost"),
            "segment_length_miles": value_f64(item.get("_length")),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "segment_midpoint_haversine",
            "speed_provider_value": speed,
            "speed_unit": "not_documented_in_dataset_schema",
            "data_as_of_local": updated_at,
            "provider_timezone": "America/Chicago",
            "record_freshness": record_freshness,
            "record_time_age_seconds": record_time_age_seconds,
            "vehicle_count": null,
        }));
    }
    records.sort_by(|left, right| {
        value_f64(left.get("distance_km"))
            .unwrap_or(f64::INFINITY)
            .total_cmp(&value_f64(right.get("distance_km")).unwrap_or(f64::INFINITY))
    });
    records.truncate(limit);
    let source_data_as_of = if records.is_empty() {
        data_as_of.or(rejected_data_as_of)
    } else {
        returned_data_as_of
    };
    time_sensitive_road_provider_data(
        records,
        source_data_as_of,
        "observation_time",
        chrono_tz::America::Chicago,
        CHICAGO_FLOW_OBSERVATION_MAX_AGE_MINUTES,
        !payload.is_empty(),
    )
}

async fn chicago_traffic_flow(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = [
        ("$select", "segmentid,street,_direction,_fromst,_tost,_length,start_lon,_lif_lat,_lit_lon,_lit_lat,_traffic,_last_updt"),
        ("$limit", "1500"),
    ];
    let payload: Vec<Value> = response_json(
        "Chicago Traffic Tracker",
        HTTP.get("https://data.cityofchicago.org/resource/n4j6-wkkf.json")
            .query(&query),
    )
    .await?;
    Ok(parse_chicago_traffic_flow(
        &payload, center, radius_km, limit,
    ))
}

fn parse_austin_incidents(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    lookback_hours: u32,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    for item in payload {
        let published_at = value_string(item.get("published_date"));
        let status_updated_at = value_string(item.get("traffic_report_status_date_time"));
        if !within_lookback(published_at.as_deref(), chrono_tz::UTC, lookback_hours) {
            continue;
        }
        let Some(latitude) = value_f64(item.get("latitude")) else {
            continue;
        };
        let Some(longitude) = value_f64(item.get("longitude")) else {
            continue;
        };
        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
        if distance_km > radius_km {
            continue;
        }
        update_latest(&mut data_as_of, status_updated_at.clone());
        records.push(json!({
            "record_type": "road_incident",
            "source": "austin_real_time_traffic_incidents",
            "fact_kind": "provider_record",
            "provider_event_id": item.get("traffic_report_id"),
            "incident_type": item.get("issue_reported"),
            "provider_status": item.get("traffic_report_status"),
            "reported_at": published_at,
            "updated_at": status_updated_at,
            "agency": item.get("agency"),
            "road_location": item.get("address"),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "event_point_haversine",
            "source_url": "https://data.austintexas.gov/resource/dx9v-zd7x",
        }));
        if records.len() >= limit {
            break;
        }
    }
    road_provider_data(
        records,
        data_as_of,
        "event_updated_at",
        false,
        chrono_tz::UTC,
    )
}

async fn austin_incidents(
    center: (f64, f64),
    radius_km: f64,
    lookback_hours: u32,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = vec![
        ("$select", "traffic_report_id,published_date,issue_reported,latitude,longitude,address,traffic_report_status,traffic_report_status_date_time,agency".into()),
        ("$where", format!("within_circle(location,{:.5},{:.5},{:.0})", center.0, center.1, radius_km * 1000.0)),
        ("$order", "published_date DESC".into()),
        ("$limit", "200".into()),
    ];
    let payload: Vec<Value> = response_json(
        "Austin real-time traffic incidents",
        HTTP.get("https://data.austintexas.gov/resource/dx9v-zd7x.json")
            .query(&query),
    )
    .await?;
    Ok(parse_austin_incidents(
        &payload,
        center,
        radius_km,
        lookback_hours,
        limit,
    ))
}

fn parse_calgary_incidents(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    for item in payload {
        let modified_at = value_string(item.get("modified_dt"));
        update_latest(&mut data_as_of, modified_at.clone());
        let Some(latitude) = value_f64(item.get("latitude")) else {
            continue;
        };
        let Some(longitude) = value_f64(item.get("longitude")) else {
            continue;
        };
        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
        if distance_km > radius_km {
            continue;
        }
        records.push(json!({
            "record_type": "road_incident",
            "source": "calgary_current_traffic_incidents",
            "fact_kind": "unverified_provider_signal",
            "incident_type": item.get("description"),
            "road_location": item.get("incident_info"),
            "provider_start_at": item.get("start_dt"),
            "updated_at": modified_at,
            "provider_timezone": null,
            "timestamp_timezone_assumption": "America/Edmonton inferred from provider jurisdiction; dataset schema does not document a timezone",
            "provider_record_count": value_i64(item.get("count")),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "event_point_haversine",
            "source_url": "https://data.calgary.ca/resource/4jah-h97u",
        }));
        if records.len() >= limit {
            break;
        }
    }
    road_provider_data(
        records,
        data_as_of,
        "event_updated_at",
        false,
        chrono_tz::America::Edmonton,
    )
}

async fn calgary_incidents(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = vec![
        (
            "$select",
            "incident_info,description,start_dt,modified_dt,quadrant,count,longitude,latitude"
                .into(),
        ),
        (
            "$where",
            format!(
                "within_circle(point,{:.5},{:.5},{:.0})",
                center.0,
                center.1,
                radius_km * 1000.0
            ),
        ),
        ("$order", "modified_dt DESC".into()),
        ("$limit", "100".into()),
    ];
    let payload: Vec<Value> = response_json(
        "Calgary current traffic incidents",
        HTTP.get("https://data.calgary.ca/resource/4jah-h97u.json")
            .query(&query),
    )
    .await?;
    Ok(parse_calgary_incidents(&payload, center, radius_km, limit))
}

fn parse_tfl_disruptions(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    for item in payload {
        let updated_at = value_string(item.get("lastModifiedTime"))
            .or_else(|| value_string(item.get("currentUpdateDateTime")));
        let Some((distance_km, latitude, longitude, distance_basis)) =
            geometry_distance(item.get("geography"), center)
        else {
            continue;
        };
        if distance_km > radius_km {
            continue;
        }
        update_latest(&mut data_as_of, updated_at.clone());
        records.push(json!({
            "record_type": "road_disruption",
            "source": "transport_for_london_road_disruptions",
            "fact_kind": "provider_disruption_record",
            "provider_event_id": item.get("id"),
            "incident_type": item.get("category"),
            "incident_subtype": item.get("subCategory"),
            "severity": item.get("severity"),
            "road_location": item.get("location"),
            "current_update": item.get("currentUpdate"),
            "current_update_at": item.get("currentUpdateDateTime"),
            "last_modified_at": updated_at,
            "start_at": item.get("startDateTime"),
            "end_at": item.get("endDateTime"),
            "has_closures": item.get("hasClosures"),
            "is_provisional": item.get("isProvisional"),
            "corridor_ids": item.get("corridorIds"),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": distance_basis,
            "source_url": "https://api.tfl.gov.uk/Road/all/Disruption",
            "attribution": "Source: Transport for London Open Data",
            "terms_url": "https://tfl.gov.uk/corporate/terms-and-conditions/transport-data-service",
        }));
    }
    records.sort_by(|left, right| {
        value_f64(left.get("distance_km"))
            .unwrap_or(f64::INFINITY)
            .total_cmp(&value_f64(right.get("distance_km")).unwrap_or(f64::INFINITY))
    });
    records.truncate(limit);
    road_provider_data(
        records,
        data_as_of,
        "event_updated_at",
        false,
        chrono_tz::UTC,
    )
}

async fn tfl_disruptions(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let payload: Vec<Value> = response_json(
        "Transport for London road disruptions",
        HTTP.get("https://api.tfl.gov.uk/Road/all/Disruption"),
    )
    .await?;
    Ok(parse_tfl_disruptions(&payload, center, radius_km, limit))
}

fn parse_drivebc_incidents(
    payload: &Value,
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let mut records = Vec::new();
    let mut data_as_of = None;
    let events = payload
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "DriveBC Open511 response omitted the expected events array".to_string())?;
    for item in events {
        if !item.is_object() {
            return Err("DriveBC Open511 events array contained a non-object entry".into());
        }
        let updated_at = value_string(item.get("updated"));
        let Some((distance_km, latitude, longitude, distance_basis)) =
            geometry_distance(item.get("geography"), center)
        else {
            continue;
        };
        if distance_km > radius_km {
            continue;
        }
        update_latest(&mut data_as_of, updated_at.clone());
        records.push(json!({
            "record_type": "road_incident",
            "source": "drivebc_open511",
            "fact_kind": "provider_record",
            "provider_event_id": item.get("id"),
            "headline": item.get("headline"),
            "incident_type": item.get("event_type"),
            "incident_subtypes": item.get("event_subtypes"),
            "severity": item.get("severity"),
            "provider_status": item.get("status"),
            "created_at": item.get("created"),
            "updated_at": updated_at,
            "description": item.get("description"),
            "roads": item.get("roads"),
            "areas": item.get("areas"),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": distance_basis,
            "source_url": item.get("url"),
        }));
    }
    records.sort_by(|left, right| {
        value_f64(left.get("distance_km"))
            .unwrap_or(f64::INFINITY)
            .total_cmp(&value_f64(right.get("distance_km")).unwrap_or(f64::INFINITY))
    });
    records.truncate(limit);
    Ok(road_provider_data(
        records,
        data_as_of,
        "event_updated_at",
        false,
        chrono_tz::UTC,
    ))
}

async fn drivebc_incidents(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let payload: Value = response_json(
        "DriveBC Open511",
        HTTP.get("https://api.open511.gov.bc.ca/events")
            .query(&[("status", "ACTIVE"), ("limit", "500")]),
    )
    .await?;
    parse_drivebc_incidents(&payload, center, radius_km, limit)
}

fn parse_fintraffic_announcements(
    payload: &Value,
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let mut records = Vec::new();
    let data_as_of = value_string(payload.get("dataUpdatedTime"));
    let features = payload
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "Fintraffic traffic announcements response omitted the expected features array"
                .to_string()
        })?;
    for feature in features {
        if !feature.is_object() {
            return Err(
                "Fintraffic traffic announcements features array contained a non-object entry"
                    .into(),
            );
        }
        let properties = feature.get("properties").unwrap_or(&Value::Null);
        let version_time = value_string(properties.get("versionTime"));
        let announcement_type = value_string(properties.get("trafficAnnouncementType"));
        if announcement_type
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("ended"))
        {
            continue;
        }
        let Some((distance_km, latitude, longitude, distance_basis)) =
            geometry_distance(feature.get("geometry"), center)
        else {
            continue;
        };
        if distance_km > radius_km {
            continue;
        }
        let announcement = properties
            .get("announcements")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("language").and_then(Value::as_str) == Some("en"))
                    .or_else(|| items.first())
            });
        records.push(json!({
            "record_type": "road_announcement",
            "source": "fintraffic_traffic_announcements_v2",
            "fact_kind": "provider_record",
            "provider_event_id": properties.get("situationId"),
            "provider_announcement_type": announcement_type,
            "feed_lifecycle_state": "not_marked_ended",
            "is_active_in_provider_feed": true,
            "situation_type": properties.get("situationType"),
            "release_at": properties.get("releaseTime"),
            "updated_at": version_time,
            "title": announcement.and_then(|value| value.get("title")),
            "road_location": announcement.and_then(|value| value.pointer("/location/description")),
            "reported_features": announcement.and_then(|value| value.get("features")),
            "start_at": announcement.and_then(|value| value.pointer("/timeAndDuration/startTime")),
            "end_at": announcement.and_then(|value| value.pointer("/timeAndDuration/endTime")),
            "sender": announcement.and_then(|value| value.get("sender")),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": distance_basis,
            "source_url": "https://liikennetilanne.fintraffic.fi/",
        }));
    }
    records.sort_by(|left, right| {
        value_f64(left.get("distance_km"))
            .unwrap_or(f64::INFINITY)
            .total_cmp(&value_f64(right.get("distance_km")).unwrap_or(f64::INFINITY))
    });
    records.truncate(limit);
    Ok(road_provider_data(
        records,
        data_as_of,
        "feed_generated_at",
        false,
        chrono_tz::UTC,
    ))
}

async fn fintraffic_announcements(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let latitude_delta = radius_km / 111.0;
    let longitude_delta = radius_km / (111.0 * center.0.to_radians().cos().abs().max(0.1));
    let query = [
        ("xMin", format!("{:.5}", center.1 - longitude_delta)),
        ("yMin", format!("{:.5}", center.0 - latitude_delta)),
        ("xMax", format!("{:.5}", center.1 + longitude_delta)),
        ("yMax", format!("{:.5}", center.0 + latitude_delta)),
    ];
    let payload: Value = response_json(
        "Fintraffic traffic announcements V2",
        HTTP.get("https://tie.digitraffic.fi/api/traffic-message/v2/traffic-announcements")
            .query(&query)
            .header("Digitraffic-User", DIGITRAFFIC_USER),
    )
    .await?;
    parse_fintraffic_announcements(&payload, center, radius_km, limit)
}

fn parse_chicago_crashes(
    payload: &[Value],
    center: (f64, f64),
    radius_km: f64,
    lookback_hours: u32,
    limit: usize,
) -> RoadProviderData {
    let mut records = Vec::new();
    let mut data_as_of = None;
    for item in payload {
        let crash_at = value_string(item.get("crash_date"));
        if !within_lookback(
            crash_at.as_deref(),
            chrono_tz::America::Chicago,
            lookback_hours,
        ) {
            continue;
        }
        let Some(latitude) = value_f64(item.get("latitude")) else {
            continue;
        };
        let Some(longitude) = value_f64(item.get("longitude")) else {
            continue;
        };
        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
        if distance_km > radius_km {
            continue;
        }
        update_latest(&mut data_as_of, crash_at.clone());
        records.push(json!({
            "record_type": "road_crash_report",
            "source": "chicago_police_traffic_crashes",
            "fact_kind": "police_report",
            "provider_event_id": item.get("crash_record_id"),
            "event_time_local": crash_at,
            "provider_timezone": "America/Chicago",
            "police_notified_at_local": item.get("date_police_notified"),
            "crash_type": item.get("crash_type"),
            "first_crash_type": item.get("first_crash_type"),
            "report_type": item.get("report_type"),
            "damage_category": item.get("damage"),
            "number_of_units": value_i64(item.get("num_units")),
            "injuries_total": value_i64(item.get("injuries_total")),
            "injuries_fatal": value_i64(item.get("injuries_fatal")),
            "most_severe_injury": item.get("most_severe_injury"),
            "street": format!("{} {} {}",
                value_string(item.get("street_no")).unwrap_or_default(),
                value_string(item.get("street_direction")).unwrap_or_default(),
                value_string(item.get("street_name")).unwrap_or_default()).trim().to_string(),
            "weather_condition": item.get("weather_condition"),
            "roadway_surface_condition": item.get("roadway_surface_cond"),
            "latitude": latitude,
            "longitude": longitude,
            "distance_km": rounded_distance_km(distance_km),
            "distance_basis": "report_point_haversine",
            "source_url": "https://data.cityofchicago.org/resource/85ca-t3if",
        }));
        if records.len() >= limit {
            break;
        }
    }
    road_provider_data(
        records,
        data_as_of,
        "event_time",
        false,
        chrono_tz::America::Chicago,
    )
}

async fn chicago_crashes(
    center: (f64, f64),
    radius_km: f64,
    lookback_hours: u32,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let query = vec![
        ("$select", "crash_record_id,crash_date,first_crash_type,weather_condition,roadway_surface_cond,report_type,crash_type,damage,date_police_notified,street_no,street_direction,street_name,num_units,most_severe_injury,injuries_total,injuries_fatal,latitude,longitude".into()),
        ("$where", format!("within_circle(location,{:.5},{:.5},{:.0})", center.0, center.1, radius_km * 1000.0)),
        ("$order", "crash_date DESC".into()),
        ("$limit", "200".into()),
    ];
    let payload: Vec<Value> = response_json(
        "Chicago Police traffic crash reports",
        HTTP.get("https://data.cityofchicago.org/resource/85ca-t3if.json")
            .query(&query),
    )
    .await?;
    Ok(parse_chicago_crashes(
        &payload,
        center,
        radius_km,
        lookback_hours,
        limit,
    ))
}

#[derive(Debug, PartialEq, Eq)]
struct CaltransChpIncident {
    incident_id: String,
    provider_type: String,
    event_time_local: String,
    event_time_utc: DateTime<Utc>,
    road: String,
}

fn normalized_element_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_caltrans_chp_event_time(value: &str) -> Option<(String, DateTime<Utc>)> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let naive = ["%b %e %Y %l:%M%p", "%b %d %Y %l:%M%p"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(&normalized, format).ok())?;
    let utc = chrono_tz::America::Los_Angeles
        .from_local_datetime(&naive)
        .earliest()?
        .with_timezone(&Utc);
    Some((naive.format("%Y-%m-%dT%H:%M:%S").to_string(), utc))
}

fn parse_caltrans_chp_html(
    description: &str,
    header_selector: &Selector,
    title_selector: &Selector,
    report_selector: &Selector,
) -> Option<CaltransChpIncident> {
    let document = Html::parse_fragment(description);
    let header = normalized_element_text(document.select(header_selector).next()?);
    let incident_id = header.strip_prefix("CHP Incident ")?.trim();
    if incident_id.is_empty()
        || incident_id.len() > 64
        || !incident_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    let provider_type = normalized_element_text(document.select(title_selector).next()?);
    if provider_type.is_empty() || provider_type.len() > 200 {
        return None;
    }
    let report = document.select(report_selector).next()?;
    let report_parts = report
        .text()
        .map(|part| part.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let (event_time_local, event_time_utc) = parse_caltrans_chp_event_time(report_parts.first()?)?;
    // The first paragraph's second text node is the provider road/location.
    // Any later text nodes, and every subsequent paragraph, are ignored.
    let road = report_parts.get(1)?.trim().to_string();
    if road.is_empty() || road.len() > 240 {
        return None;
    }
    Some(CaltransChpIncident {
        incident_id: incident_id.to_string(),
        provider_type,
        event_time_local,
        event_time_utc,
        road,
    })
}

fn parse_caltrans_chp_coordinates(value: &str) -> Option<(f64, f64)> {
    let mut parts = value.trim().split(',');
    let longitude = parts.next()?.trim().parse::<f64>().ok()?;
    let latitude = parts.next()?.trim().parse::<f64>().ok()?;
    coordinates(Some(latitude), Some(longitude)).ok()
}

fn normalize_http_last_modified(value: &str) -> Option<String> {
    DateTime::parse_from_rfc2822(value)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .ok()
        .map(|value| {
            value
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        })
}

fn parse_caltrans_chp_kml_at(
    payload: &[u8],
    http_last_modified: &str,
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
    now: DateTime<Utc>,
) -> Result<RoadProviderData, String> {
    if parse_provider_time(http_last_modified, chrono_tz::UTC).is_none() {
        return Err("Caltrans QuickMap CHP HTTP Last-Modified time was invalid".into());
    }
    let header_selector = Selector::parse(".iw-header-left")
        .map_err(|_| "Caltrans QuickMap CHP header selector was invalid")?;
    let title_selector = Selector::parse(".iw-title")
        .map_err(|_| "Caltrans QuickMap CHP title selector was invalid")?;
    let report_selector = Selector::parse(".iw-body p.iw-text")
        .map_err(|_| "Caltrans QuickMap CHP report selector was invalid")?;

    let mut reader = XmlReader::from_reader(payload);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut saw_kml = false;
    let mut saw_kml_namespace = false;
    let mut saw_document = false;
    let mut document_name = String::new();
    let mut capture_document_name = false;
    let mut in_placemark = false;
    let mut capture_description = false;
    let mut capture_coordinates = false;
    let mut description = String::new();
    let mut coordinates_text = String::new();
    let mut placemark_count = 0usize;
    let mut structurally_valid_incidents = 0usize;
    let mut future_local_incidents = 0usize;
    let mut records = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Start(start)) => match start.local_name().as_ref() {
                b"kml" => {
                    saw_kml = true;
                    for attribute in start.attributes().with_checks(true) {
                        let attribute = attribute.map_err(|error| {
                            format!("Caltrans QuickMap CHP KML attribute was invalid: {error}")
                        })?;
                        if attribute.key.as_ref() == b"xmlns" {
                            let value = attribute
                                .decode_and_unescape_value(reader.decoder())
                                .map_err(|error| {
                                    format!(
                                        "Caltrans QuickMap CHP KML namespace was invalid: {error}"
                                    )
                                })?;
                            saw_kml_namespace = value == "http://www.opengis.net/kml/2.2";
                        }
                    }
                }
                b"Document" => saw_document = true,
                b"Placemark" => {
                    in_placemark = true;
                    description.clear();
                    coordinates_text.clear();
                }
                b"name" if saw_document && !in_placemark && document_name.is_empty() => {
                    capture_document_name = true;
                }
                b"description" if in_placemark => capture_description = true,
                b"coordinates" if in_placemark => capture_coordinates = true,
                _ => {}
            },
            Ok(XmlEvent::Text(text)) => {
                let decoded = text.decode().map_err(|error| {
                    format!("Caltrans QuickMap CHP KML text was invalid: {error}")
                })?;
                if capture_document_name {
                    document_name.push_str(&decoded);
                } else if capture_description {
                    description.push_str(&decoded);
                } else if capture_coordinates {
                    coordinates_text.push_str(&decoded);
                }
            }
            Ok(XmlEvent::CData(data)) if capture_description => {
                description.push_str(&data.decode().map_err(|error| {
                    format!("Caltrans QuickMap CHP KML CDATA was invalid: {error}")
                })?);
            }
            Ok(XmlEvent::End(end)) => match end.local_name().as_ref() {
                b"name" => capture_document_name = false,
                b"description" => capture_description = false,
                b"coordinates" => capture_coordinates = false,
                b"Placemark" => {
                    in_placemark = false;
                    placemark_count += 1;
                    if let (Some(incident), Some((latitude, longitude))) = (
                        parse_caltrans_chp_html(
                            &description,
                            &header_selector,
                            &title_selector,
                            &report_selector,
                        ),
                        parse_caltrans_chp_coordinates(&coordinates_text),
                    ) {
                        structurally_valid_incidents += 1;
                        let distance_km = haversine_km(center.0, center.1, latitude, longitude);
                        if distance_km <= radius_km {
                            if incident.event_time_utc
                                > now + ChronoDuration::minutes(PROVIDER_FUTURE_TOLERANCE_MINUTES)
                            {
                                future_local_incidents += 1;
                            } else if records.len() < limit {
                                records.push(json!({
                                    "record_type": "road_incident_public_feed_entry",
                                    "source": "caltrans_quickmap_chp_incidents",
                                    "fact_kind": "provider_current_feed_membership",
                                    "provider_incident_id": incident.incident_id,
                                    "provider_incident_type": incident.provider_type,
                                    "event_time_local": incident.event_time_local,
                                    "provider_timezone": "America/Los_Angeles",
                                    "road": incident.road,
                                    "latitude": latitude,
                                    "longitude": longitude,
                                    "distance_km": rounded_distance_km(distance_km),
                                    "distance_basis": "incident_point_haversine",
                                    "present_in_current_public_feed": true,
                                    "source_url": "https://quickmap.dot.ca.gov/",
                                    "source_data_url": CALTRANS_CHP_KML_URL,
                                    "attribution": "Information courtesy of California Highway Patrol (CHP), via Caltrans QuickMap",
                                    "terms_url": "https://www.chp.ca.gov/about-us/conditions-of-use/",
                                }));
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(XmlEvent::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(format!(
                    "Caltrans QuickMap CHP returned invalid KML: {error}"
                ));
            }
        }
        buffer.clear();
    }

    if !saw_kml || !saw_kml_namespace || !saw_document || document_name.trim() != "CHP Incidents" {
        return Err("Caltrans QuickMap CHP response did not match the expected KML schema".into());
    }
    if placemark_count > 0 && structurally_valid_incidents == 0 {
        return Err(
            "Caltrans QuickMap CHP placemarks did not match the expected incident schema".into(),
        );
    }
    if records.is_empty() && future_local_incidents > 0 {
        return Err(
            "Caltrans QuickMap CHP returned only local incident times more than 5 minutes in the future"
                .into(),
        );
    }

    Ok(time_sensitive_road_provider_data_at(
        records,
        Some(http_last_modified.to_string()),
        "http_last_modified",
        chrono_tz::UTC,
        CALTRANS_CHP_REPRESENTATION_MAX_AGE_MINUTES,
        true,
        now,
    ))
}

async fn caltrans_chp_incidents(
    center: (f64, f64),
    radius_km: f64,
    limit: usize,
) -> Result<RoadProviderData, String> {
    let source = "Caltrans QuickMap CHP incidents";
    let mut response = HTTP
        .get(CALTRANS_CHP_KML_URL)
        .send()
        .await
        .map_err(|error| format!("{source} request failed: {}", error.without_url()))?;
    let http_status = response.status();
    if !http_status.is_success() {
        return Err(format!("{source} returned HTTP {http_status}"));
    }
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .unwrap_or_default();
    if !matches!(
        content_type,
        "application/vnd.google-earth.kml+xml" | "application/xml" | "text/xml"
    ) {
        return Err(format!("{source} returned an unexpected Content-Type"));
    }
    let http_last_modified = response
        .headers()
        .get(header::LAST_MODIFIED)
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_http_last_modified)
        .ok_or_else(|| format!("{source} omitted a valid HTTP Last-Modified time"))?;
    if response
        .content_length()
        .is_some_and(|length| length > CALTRANS_CHP_MAX_RESPONSE_BYTES as u64)
    {
        return Err(format!("{source} response exceeded the byte limit"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("{source} response read failed: {}", error.without_url()))?
    {
        if bytes.len().saturating_add(chunk.len()) > CALTRANS_CHP_MAX_RESPONSE_BYTES {
            return Err(format!("{source} response exceeded the byte limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_caltrans_chp_kml_at(
        &bytes,
        &http_last_modified,
        center,
        radius_km,
        limit,
        Utc::now(),
    )
}

#[allow(clippy::too_many_arguments)]
fn merge_road_provider(
    source: &str,
    relevant: bool,
    covered: bool,
    result: Option<Result<RoadProviderData, String>>,
    success_detail: &str,
    coverage_detail: &str,
    records: &mut Vec<Value>,
    statuses: &mut Vec<LiveSourceStatus>,
) {
    if !relevant {
        return;
    }
    if !covered {
        statuses.push(status(
            source,
            LiveSourceState::NoCoverage,
            0,
            coverage_detail,
            None,
        ));
        return;
    }
    match result {
        Some(Ok(data)) => {
            let count = data.records.len();
            let state = if data.stale {
                LiveSourceState::Stale
            } else if count == 0 {
                LiveSourceState::Empty
            } else if data.delayed {
                LiveSourceState::Delayed
            } else {
                LiveSourceState::Success
            };
            let detail = if data.stale {
                let time_state = data.stale_after_minutes.map(|minutes| {
                    observation_window(data.data_as_of.as_deref(), data.timezone, minutes).state
                });
                match time_state {
                    Some(ObservationWindowState::Future) => format!(
                        "No observations were returned because the provider timestamp is more than {PROVIDER_FUTURE_TOLERANCE_MINUTES} minutes in the future."
                    ),
                    Some(ObservationWindowState::MissingOrInvalid) =>
                        "No observations were returned because the provider timestamp was missing or unparseable."
                            .into(),
                    _ => {
                        let threshold = data
                            .stale_after_minutes
                            .map(|minutes| format!("{minutes} minutes"))
                            .unwrap_or_else(|| "this source's freshness threshold".into());
                        format!(
                            "No observations were returned because the newest provider timestamp is older than {threshold}."
                        )
                    }
                }
            } else if count == 0 {
                "The provider responded successfully, but no locally matching records were returned."
                    .into()
            } else if data.delayed {
                format!(
                    "{success_detail} The newest provider timestamp is more than {DELAYED_AFTER_MINUTES} minutes old, so this source is marked delayed."
                )
            } else {
                success_detail.to_string()
            };
            statuses.push(evidenced_status(
                source,
                state,
                count,
                detail,
                data.data_as_of,
                data.data_as_of_kind,
                data.timezone,
                data.stale_after_minutes,
                data.cache_state,
                data.cache_age_seconds,
            ));
            records.extend(data.records);
        }
        Some(Err(error)) => statuses.push(status(source, LiveSourceState::Failed, 0, error, None)),
        None => statuses.push(status(
            source,
            LiveSourceState::Failed,
            0,
            "Provider execution was not scheduled despite matching coverage.",
            None,
        )),
    }
}

#[tauri::command]
pub async fn road_environment(
    kind: String,
    latitude: f64,
    longitude: f64,
    radius_km: Option<u32>,
    lookback_hours: Option<u32>,
    limit: Option<u32>,
) -> Result<LiveDataResponse, String> {
    let center = coordinates(Some(latitude), Some(longitude))?;
    let kind = kind.trim().to_lowercase();
    if !matches!(
        kind.as_str(),
        "overview" | "vehicle_counts" | "traffic_flow" | "road_incidents"
    ) {
        return Err(
            "kind must be overview, vehicle_counts, traffic_flow, or road_incidents".into(),
        );
    }
    let radius_km = radius_km.unwrap_or(10).clamp(1, 100) as f64;
    let lookback_hours = lookback_hours.unwrap_or(24).clamp(1, 720);
    let limit = bounded_limit(limit, 20);
    let wants_counts = matches!(kind.as_str(), "overview" | "vehicle_counts");
    let wants_flow = matches!(kind.as_str(), "overview" | "traffic_flow");
    let wants_incidents = matches!(kind.as_str(), "overview" | "road_incidents");

    let winnipeg_covered = WINNIPEG_COVERAGE.may_cover(center.0, center.1, radius_km);
    let nyc_covered = NEW_YORK_CITY_COVERAGE.may_cover(center.0, center.1, radius_km);
    let chicago_covered = CHICAGO_COVERAGE.may_cover(center.0, center.1, radius_km);
    let austin_covered = AUSTIN_TRAVIS_COVERAGE.may_cover(center.0, center.1, radius_km);
    let calgary_covered = CALGARY_COVERAGE.may_cover(center.0, center.1, radius_km);
    let drivebc_covered = drivebc_may_cover(center, radius_km);
    let finland_covered = polygon_may_cover(center, radius_km, FINLAND_MAINLAND_GATE);
    let norway_covered = polygon_may_cover(center, radius_km, NORWAY_MAINLAND_GATE);
    let london_covered = LONDON_COVERAGE.may_cover(center.0, center.1, radius_km);
    let caltrans_chp_covered = polygon_may_cover(center, radius_km, CALIFORNIA_CHP_ROUGH_GATE);

    let winnipeg = async {
        if wants_counts && winnipeg_covered {
            Some(
                timed(
                    "City of Winnipeg permanent count stations",
                    winnipeg_vehicle_counts(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let norway_counts = async {
        if wants_counts && norway_covered {
            Some(
                timed(
                    "Norwegian Public Roads hourly vehicle counts",
                    norway_vehicle_counts(center, radius_km),
                )
                .await,
            )
        } else {
            None
        }
    };
    let nyc_flow = async {
        if wants_flow && nyc_covered {
            Some(
                timed(
                    "NYC DOT Traffic Speeds",
                    nyc_traffic_flow(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let chicago_flow = async {
        if wants_flow && chicago_covered {
            Some(
                timed(
                    "Chicago Traffic Tracker",
                    chicago_traffic_flow(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let fintraffic_flow = async {
        if wants_flow && finland_covered {
            Some(
                timed(
                    "Fintraffic TMS sensor values",
                    fintraffic_tms_flow(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let austin = async {
        if wants_incidents && austin_covered {
            Some(
                timed(
                    "Austin real-time traffic incidents",
                    austin_incidents(center, radius_km, lookback_hours, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let calgary = async {
        if wants_incidents && calgary_covered {
            Some(
                timed(
                    "Calgary current traffic incidents",
                    calgary_incidents(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let drivebc = async {
        if wants_incidents && drivebc_covered {
            Some(
                timed(
                    "DriveBC Open511",
                    drivebc_incidents(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let tfl = async {
        if wants_incidents && london_covered {
            Some(
                timed(
                    "Transport for London road disruptions",
                    tfl_disruptions(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let fintraffic = async {
        if wants_incidents && finland_covered {
            Some(
                timed(
                    "Fintraffic traffic announcements V2",
                    fintraffic_announcements(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let chicago_crash_reports = async {
        if wants_incidents && chicago_covered {
            Some(
                timed(
                    "Chicago Police traffic crash reports",
                    chicago_crashes(center, radius_km, lookback_hours, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let caltrans_chp = async {
        if wants_incidents && caltrans_chp_covered {
            Some(
                timed(
                    "Caltrans QuickMap CHP incidents",
                    caltrans_chp_incidents(center, radius_km, limit),
                )
                .await,
            )
        } else {
            None
        }
    };
    let (
        winnipeg,
        norway_counts,
        nyc_flow,
        chicago_flow,
        fintraffic_flow,
        austin,
        calgary,
        drivebc,
        tfl,
        fintraffic,
        chicago_crash_reports,
        caltrans_chp,
    ) = tokio::join!(
        winnipeg,
        norway_counts,
        nyc_flow,
        chicago_flow,
        fintraffic_flow,
        austin,
        calgary,
        drivebc,
        tfl,
        fintraffic,
        chicago_crash_reports,
        caltrans_chp
    );

    let mut records = Vec::new();
    let mut statuses = Vec::new();
    merge_road_provider(
        "winnipeg_permanent_count_stations",
        wants_counts,
        winnipeg_covered,
        winnipeg,
        "Official permanent radar-station directional vehicle counts were returned.",
        "The keyless vehicle-count provider currently covers Winnipeg permanent count stations only.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "norway_public_roads_traffic_data",
        wants_counts,
        norway_covered,
        norway_counts,
        "The nearest operational registration point's latest published hourly vehicle count was returned with its coverage percentage.",
        "The Norwegian Public Roads count source covers operational registration points in Norway and does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "nyc_dot_traffic_speeds",
        wants_flow,
        nyc_covered,
        nyc_flow,
        "The latest available snapshot for each returned NYC DOT road link was selected.",
        "The NYC DOT speed feed does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "chicago_traffic_tracker_segments",
        wants_flow,
        chicago_covered,
        chicago_flow,
        "Chicago arterial segment speed estimates based on CTA bus GPS traces were returned.",
        "The Chicago Traffic Tracker feed does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "fintraffic_tms_sensor_values",
        wants_flow,
        finland_covered,
        fintraffic_flow,
        "The nearest Fintraffic TMS station's published flow-rate and average-speed sensor values were returned with provider windows and units.",
        "Fintraffic TMS stations cover Finland and do not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "austin_real_time_traffic_incidents",
        wants_incidents,
        austin_covered,
        austin,
        "Austin-Travis public-safety traffic incident records within the requested lookback were returned.",
        "The Austin-Travis incident feed does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "calgary_current_traffic_incidents",
        wants_incidents,
        calgary_covered,
        calgary,
        "Calgary's current camera-derived traffic disruption signals were returned.",
        "The Calgary incident feed does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "drivebc_open511",
        wants_incidents,
        drivebc_covered,
        drivebc,
        "Active DriveBC Open511 events were returned and locally radius-filtered.",
        "DriveBC Open511 covers British Columbia and does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "transport_for_london_road_disruptions",
        wants_incidents,
        london_covered,
        tfl,
        "Current TfL road disruption records were returned and locally radius-filtered.",
        "The TfL road disruption feed covers London and does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "fintraffic_traffic_announcements_v2",
        wants_incidents,
        finland_covered,
        fintraffic,
        "Non-ended Fintraffic V2 traffic announcements were returned and locally radius-filtered.",
        "Fintraffic traffic announcements cover Finland and do not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "chicago_police_traffic_crashes",
        wants_incidents,
        chicago_covered,
        chicago_crash_reports,
        "Finalized or amended Chicago Police crash reports within the requested lookback were returned.",
        "The Chicago Police crash-report dataset does not cover this requested area.",
        &mut records,
        &mut statuses,
    );
    merge_road_provider(
        "caltrans_quickmap_chp_incidents",
        wants_incidents,
        caltrans_chp_covered,
        caltrans_chp,
        "Entries present in the current Caltrans QuickMap CHP public feed were returned with dispatch narratives excluded.",
        "The Caltrans QuickMap CHP source covers public CHP roadway incident entries in California only and does not cover this requested area.",
        &mut records,
        &mut statuses,
    );

    records.sort_by(|left, right| {
        value_f64(left.get("distance_km"))
            .unwrap_or(f64::INFINITY)
            .total_cmp(&value_f64(right.get("distance_km")).unwrap_or(f64::INFINITY))
    });
    Ok(response(
        "road_environment",
        records,
        statuses,
        vec![
            "There is no anonymous global API that counts every nearby vehicle or reports every minor crash. Providers are queried only inside configured city/province/country coverage gates, but those gates and the underlying sensor networks do not prove road-by-road coverage; no_coverage means the system has no applicable source, not that the road is clear.".into(),
            "A vehicle_count is the provider's count at one fixed sensor for its timestamp interval. It is not the number of distinct vehicles simultaneously surrounding the user, and counts from different stations must not be added because the same vehicle may pass more than one sensor. Norway's public hourly aggregates are normally delayed by approximately 2 to 3 hours and include a provider coverage percentage.".into(),
            "Fintraffic TMS kpl/h values are provider vehicle flow rates for stated five-minute windows, not raw five-minute counts or simultaneous nearby vehicles. Traffic speed is a sensor value, road-link observation, or provider estimate and does not reveal vehicle count, route travel time, or every local street.".into(),
            "The public NYC DOT and Chicago Traffic Tracker dataset schemas do not document a speed unit, so their records expose speed_provider_value with speed_unit=not_documented_in_dataset_schema instead of assuming mph.".into(),
            "Austin records may already be archived, Calgary signals are camera-derived and explicitly unverified, DriveBC exposes active-feed events, TfL exposes road disruptions rather than a complete crash registry, and Chicago records are finalized/amended police reports rather than a live dispatch feed. Caltrans QuickMap exposes selected public CHP entries on or near California roadways; feed membership does not prove an entry is complete, independently verified, still active, or representative of local-police-only and unreported incidents. Calgary's naive timestamps have no timezone in the dataset schema; America/Edmonton is only a jurisdiction-based interpretation for source freshness.".into(),
            "Caltrans QuickMap CHP records intentionally exclude the provider's dispatch narrative, including possible plate, phone, medical, and person details. Only the incident identifier, provider type, first reported local time, road text, and point location are retained; America/Los_Angeles is a jurisdiction-based interpretation of the provider's naive event time.".into(),
            "Fintraffic trafficAnnouncementType is a provider announcement/lifecycle type, not a collision category. Ended tombstones are excluded; reported_features retains the provider's event features for non-ended announcements.".into(),
            "lookback_hours filters Austin records and Chicago Police reports only. Calgary, DriveBC, TfL, Fintraffic, and Caltrans QuickMap are current-feed queries with their own time or membership semantics. limit is applied per applicable provider, so overview can return more than limit records when multiple local sources apply.".into(),
            "Dynamic road observations bypass the connector cache. Only the Fintraffic and Norway station directories can be cached briefly; cache_state and cache_age_seconds describe that directory cache, not the age of the returned observation.".into(),
            "DriveBC applicability is gated with the generalized Statistics Canada 2021 Digital Boundary for British Columbia (PRUID 59), used under the Open Government Licence - Canada; the boundary gate is not evidence that DriveBC monitors every road inside it.".into(),
            "Freshness gates fail closed for current numeric observations: Winnipeg and NYC 60 minutes, Chicago 30 minutes, Fintraffic TMS 15 minutes, and Norway hourly counts 8 hours. Caltrans QuickMap's HTTP Last-Modified is an http_last_modified freshness proxy for the returned representation, not a proven feed-generation or event-update time; it fails closed after 15 minutes. If no valid local CHP entries remain solely because their event times are more than 5 minutes in the future, the source fails explicitly instead of being rewritten as empty. Accepted numeric data more than 15 minutes old is marked delayed. Other active/current-feed event records can remain valid without a recent event modification, so their event timestamps remain separate instead of being silently discarded; a successful event-feed request is not proof of completeness.".into(),
            "An empty covered feed does not prove there are no vehicles, crashes, hazards, blocked lanes, or delayed reports. Use emergency services and official traveler-information channels for safety-critical decisions.".into(),
            "distance_km is Haversine distance to a provider point, segment midpoint, or nearest point on a reported geometry path; it is zero when the authorized/query point lies inside a reported Polygon or MultiPolygon after respecting interior holes. It is not driving distance or response time.".into(),
        ],
    ))
}

fn tracking_number_mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 6 {
        return "*".repeat(chars.len());
    }
    format!(
        "{}{}{}",
        chars[..3].iter().collect::<String>(),
        "*".repeat(chars.len() - 6),
        chars[chars.len() - 3..].iter().collect::<String>()
    )
}

fn carrier_details(carrier: &str) -> Option<(&'static str, &'static str)> {
    match carrier {
        "ups" => Some(("UPS", "https://www.ups.com/track")),
        "usps" => Some(("USPS", "https://tools.usps.com/go/TrackConfirmAction")),
        "fedex" => Some(("FedEx", "https://www.fedex.com/fedextrack/")),
        "dhl" | "dhl_express" => Some(("DHL", "https://www.dhl.com/global-en/home/tracking.html")),
        "sf" | "sf_express" | "顺丰" => Some((
            "SF Express",
            "https://www.sf-express.com/chn/sc/dynamic_function/waybill/",
        )),
        "china_post" | "ems" | "中国邮政" => {
            Some(("China Post / EMS", "https://www.ems.com.cn/"))
        }
        "yto" | "圆通" => Some(("YTO Express", "https://www.yto.net.cn/")),
        "zto" | "中通" => Some(("ZTO Express", "https://www.zto.com/")),
        "sto" | "申通" => Some(("STO Express", "https://www.sto.cn/")),
        "yunda" | "韵达" => Some(("Yunda Express", "https://www.yundaex.com/")),
        "jd" | "jd_logistics" | "京东" => Some(("JD Logistics", "https://www.jdl.com/")),
        _ => None,
    }
}

fn inferred_carrier(value: &str) -> Option<&'static str> {
    let uppercase = value.to_uppercase();
    if uppercase.starts_with("1Z") && uppercase.len() == 18 {
        Some("ups")
    } else if uppercase.len() == 13
        && uppercase.starts_with(|character: char| character.is_ascii_alphabetic())
        && uppercase.ends_with("CN")
    {
        Some("china_post")
    } else {
        None
    }
}

#[tauri::command]
pub fn track_shipment(
    tracking_number: String,
    carrier: Option<String>,
) -> Result<LiveDataResponse, String> {
    let tracking_number = tracking_number.trim();
    if !(6..=64).contains(&tracking_number.len())
        || !tracking_number
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(
            "tracking_number must be 6-64 ASCII letters/digits (hyphen and underscore allowed)"
                .into(),
        );
    }
    let normalized_carrier = carrier
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
        .or_else(|| inferred_carrier(tracking_number).map(str::to_string));
    let Some(normalized_carrier) = normalized_carrier else {
        return Ok(response(
            "shipment_tracking",
            vec![json!({
                "tracking_number_masked": tracking_number_mask(tracking_number),
                "carrier": null,
                "automation_status": "carrier_required",
            })],
            vec![status(
                "official_carrier_tracking",
                LiveSourceState::Skipped,
                0,
                "The number pattern is not unique enough to identify a carrier without guessing.",
                None,
            )],
            vec![
                "No universal official keyless machine API exists for cross-carrier tracking; provide the carrier to receive its official verification page.".into(),
                "No shipment event, location, ETA, delivery state, or carrier identity was guessed.".into(),
            ],
        ));
    };
    let Some((carrier_name, official_url)) = carrier_details(&normalized_carrier) else {
        return Ok(response(
            "shipment_tracking",
            vec![json!({
                "tracking_number_masked": tracking_number_mask(tracking_number),
                "automation_status": "unsupported_carrier",
            })],
            vec![status(
                "official_carrier_tracking",
                LiveSourceState::Skipped,
                0,
                "No verified official keyless connector is registered for this carrier.",
                None,
            )],
            vec!["No third-party scraping endpoint was used as a substitute for an official carrier source.".into()],
        ));
    };
    Ok(response(
        "shipment_tracking",
        vec![json!({
            "tracking_number_masked": tracking_number_mask(tracking_number),
            "carrier": carrier_name,
            "official_tracking_url": official_url,
            "automation_status": "manual_official_verification_required",
            "tracking_events": [],
            "estimated_delivery": null,
        })],
        vec![status(
            "official_carrier_tracking",
            LiveSourceState::Skipped,
            0,
            "The carrier has no verified official keyless machine endpoint in this integration; the official tracking page is returned without fabricating events.",
            None,
        )],
        vec![
            "The official page may require manual entry, a CAPTCHA, login, or additional recipient verification.".into(),
            "The full tracking number is not echoed in the tool result or written to the connector response; only a masked form is returned.".into(),
            "An official page URL is not a claim that the parcel exists, is in transit, or has any particular delivery status.".into(),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caltrans_chp_fixture(placemarks: &str) -> Vec<u8> {
        format!(
            r#"<kml xmlns="http://www.opengis.net/kml/2.2"><Document><name>CHP Incidents</name>{placemarks}</Document></kml>"#
        )
        .into_bytes()
    }

    fn caltrans_chp_placemark(
        incident_id: &str,
        provider_type: &str,
        reported_time: &str,
        road: &str,
        dispatch_notes: &str,
        coordinates: Option<&str>,
    ) -> String {
        let point = coordinates
            .map(|coordinates| format!("<Point><coordinates>{coordinates}</coordinates></Point>"))
            .unwrap_or_default();
        format!(
            r#"<Placemark><description><![CDATA[
                <div class="infowindow-content">
                  <div class="iw-header"><div class="iw-header-left">CHP Incident {incident_id}</div></div>
                  <div class="iw-body">
                    <h2 class="iw-title">{provider_type}</h2>
                    <p class="iw-text">{reported_time}<br />{road}</p>
                    <p class="iw-text">{dispatch_notes}</p>
                  </div>
                </div>
            ]]></description>{point}</Placemark>"#
        )
    }

    #[test]
    fn caltrans_chp_parser_is_structured_radius_filtered_and_privacy_minimized() {
        let near = caltrans_chp_placemark(
            "260712LA0001",
            "1182-Trfc Collision-No Inj",
            "Jul 12 2026  8:52AM",
            "US101 S / Sunset Ave Ofr",
            "PLATE 7ABC123 / CALL 555-0100 / AMBULANCE ENRT / MALE DRIVER",
            Some("-118.2437,34.0522,0"),
        );
        let far = caltrans_chp_placemark(
            "260712GG0002",
            "1125-Traffic Hazard",
            "Jul 12 2026  8:50AM",
            "US101 N / Market St",
            "PRIVATE DISPATCH DETAIL",
            Some("-122.4194,37.7749,0"),
        );
        let missing_coordinates = caltrans_chp_placemark(
            "260712LA0003",
            "1183-Trfc Collision-Unkn Inj",
            "Jul 12 2026  8:51AM",
            "I10 W / Alameda St",
            "MEDICAL DETAIL",
            None,
        );
        let bad_coordinates = caltrans_chp_placemark(
            "260712LA0004",
            "1125-Traffic Hazard",
            "Jul 12 2026  8:51AM",
            "I5 N / Fourth St",
            "PERSON DETAIL",
            Some("bad,coordinates,0"),
        );
        let future = caltrans_chp_placemark(
            "260712LA0005",
            "1182-Trfc Collision-No Inj",
            "Jul 12 2026  9:06AM",
            "SR110 S / Ninth St",
            "PHONE 555-0101",
            Some("-118.2500,34.0500,0"),
        );
        let malformed_html = r#"<Placemark><description><![CDATA[
            <div class="infowindow-content"><div class="iw-header-left">CHP Incident 260712LA0006</div>
            <p class="iw-text">Jul 12 2026 8:49AM<br />I10 E / Main St</p></div>
        ]]></description><Point><coordinates>-118.2400,34.0500,0</coordinates></Point></Placemark>"#;
        let payload = caltrans_chp_fixture(&format!(
            "{near}{far}{missing_coordinates}{bad_coordinates}{future}{malformed_html}"
        ));
        let now = DateTime::parse_from_rfc3339("2026-07-12T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let data = parse_caltrans_chp_kml_at(
            &payload,
            "2026-07-12T15:59:00Z",
            (34.0522, -118.2437),
            10.0,
            20,
            now,
        )
        .unwrap();

        assert_eq!(data.records.len(), 1);
        let record = &data.records[0];
        assert_eq!(record["provider_incident_id"], "260712LA0001");
        assert_eq!(
            record["provider_incident_type"],
            "1182-Trfc Collision-No Inj"
        );
        assert_eq!(record["event_time_local"], "2026-07-12T08:52:00");
        assert_eq!(record["provider_timezone"], "America/Los_Angeles");
        assert_eq!(record["road"], "US101 S / Sunset Ave Ofr");
        assert_eq!(record["present_in_current_public_feed"], true);
        assert_eq!(data.data_as_of.as_deref(), Some("2026-07-12T15:59:00Z"));
        assert_eq!(data.data_as_of_kind.as_deref(), Some("http_last_modified"));
        assert!(!data.stale);

        let serialized = serde_json::to_string(&data.records).unwrap();
        for sensitive in [
            "7ABC123",
            "555-0100",
            "AMBULANCE",
            "MALE DRIVER",
            "PRIVATE DISPATCH",
            "MEDICAL DETAIL",
            "PERSON DETAIL",
            "555-0101",
        ] {
            assert!(!serialized.contains(sensitive));
        }
        for forbidden_key in ["dispatch", "notes", "medical", "person", "plate", "phone"] {
            assert!(record.get(forbidden_key).is_none());
        }
    }

    #[test]
    fn caltrans_chp_parser_validates_schema_and_accepts_a_genuine_empty_feed() {
        let now = DateTime::parse_from_rfc3339("2026-07-12T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let empty = caltrans_chp_fixture("");
        let data = parse_caltrans_chp_kml_at(
            &empty,
            "2026-07-12T15:59:00Z",
            (34.0522, -118.2437),
            10.0,
            20,
            now,
        )
        .unwrap();
        assert!(data.records.is_empty());
        assert_eq!(data.data_as_of_kind.as_deref(), Some("http_last_modified"));

        let wrong_schema =
            br#"<kml xmlns="http://www.opengis.net/kml/2.2"><Document><name>Other</name></Document></kml>"#;
        assert!(parse_caltrans_chp_kml_at(
            wrong_schema,
            "2026-07-12T15:59:00Z",
            (34.0522, -118.2437),
            10.0,
            20,
            now,
        )
        .is_err());
    }

    #[test]
    fn caltrans_chp_parser_fails_closed_for_stale_or_future_http_last_modified() {
        let incident = caltrans_chp_placemark(
            "260712LA0001",
            "1182-Trfc Collision-No Inj",
            "Jul 12 2026  8:52AM",
            "US101 S / Sunset Ave Ofr",
            "PRIVATE DISPATCH DETAIL",
            Some("-118.2437,34.0522,0"),
        );
        let payload = caltrans_chp_fixture(&incident);
        let now = DateTime::parse_from_rfc3339("2026-07-12T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        for http_last_modified in ["2026-07-12T15:44:59Z", "2026-07-12T16:05:01Z"] {
            let data = parse_caltrans_chp_kml_at(
                &payload,
                http_last_modified,
                (34.0522, -118.2437),
                10.0,
                20,
                now,
            )
            .unwrap();

            assert!(data.stale);
            assert!(data.records.is_empty());
            assert_eq!(data.data_as_of.as_deref(), Some(http_last_modified));
            assert_eq!(data.data_as_of_kind.as_deref(), Some("http_last_modified"));
        }
    }

    #[test]
    fn caltrans_chp_parser_reports_local_future_times_instead_of_empty() {
        let future = caltrans_chp_placemark(
            "260712LA0005",
            "1182-Trfc Collision-No Inj",
            "Jul 12 2026  9:06AM",
            "SR110 S / Ninth St",
            "PRIVATE DISPATCH DETAIL",
            Some("-118.2500,34.0500,0"),
        );
        let payload = caltrans_chp_fixture(&future);
        let now = DateTime::parse_from_rfc3339("2026-07-12T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let error = match parse_caltrans_chp_kml_at(
            &payload,
            "2026-07-12T15:59:00Z",
            (34.0522, -118.2437),
            10.0,
            20,
            now,
        ) {
            Ok(_) => panic!("future-only local CHP records must not become empty"),
            Err(error) => error,
        };

        assert!(error.contains("more than 5 minutes in the future"));
    }

    #[test]
    fn caltrans_chp_http_time_and_california_gate_are_explicit() {
        assert_eq!(
            normalize_http_last_modified("Sun, 12 Jul 2026 16:06:42 GMT").as_deref(),
            Some("2026-07-12T16:06:42Z")
        );
        assert!(polygon_may_cover(
            (34.0522, -118.2437),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(polygon_may_cover(
            (37.7749, -122.4194),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(polygon_may_cover(
            (38.5816, -121.4944),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(polygon_may_cover(
            (32.7157, -117.1611),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(!polygon_may_cover(
            (36.1699, -115.1398),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(!polygon_may_cover(
            (33.4484, -112.0740),
            10.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
        assert!(!polygon_may_cover(
            (32.5149, -117.0382),
            1.0,
            CALIFORNIA_CHP_ROUGH_GATE
        ));
    }

    #[test]
    fn earthquake_fixture_preserves_event_and_update_times() {
        let payload = json!({
            "features": [{
                "id": "us-test",
                "properties": {
                    "mag": 5.2,
                    "title": "M 5.2 test",
                    "place": "Test place",
                    "time": 1000,
                    "updated": 2000,
                    "status": "reviewed",
                    "url": "https://earthquake.usgs.gov/test"
                },
                "geometry": { "coordinates": [121.5, 31.2, 10.0] }
            }]
        });
        let records = parse_earthquakes(&payload, 4.0, Some((31.2, 121.5, 10.0)), 5);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["event_time_unix_ms"], 1000);
        assert_eq!(records[0]["updated_at_unix_ms"], 2000);
        assert_eq!(records[0]["review_status"], "reviewed");
    }

    #[test]
    fn earthquake_filter_never_turns_distance_into_intensity() {
        let payload = json!({
            "features": [{
                "id": "far",
                "properties": { "mag": 6.0, "time": 1, "updated": 2 },
                "geometry": { "coordinates": [0.0, 0.0, 10.0] }
            }]
        });
        assert!(parse_earthquakes(&payload, 2.5, Some((50.0, 50.0, 20.0)), 5).is_empty());
    }

    #[test]
    fn earthquake_feed_never_raises_the_requested_magnitude_floor() {
        assert!(earthquake_feed(Some("month"), 1.0)
            .0
            .ends_with("1.0_month.geojson"));
        assert!(earthquake_feed(Some("month"), 0.5)
            .0
            .ends_with("all_month.geojson"));
        assert!(earthquake_feed(Some("week"), 0.5)
            .0
            .ends_with("all_week.geojson"));
    }

    #[test]
    fn shipment_router_masks_numbers_and_never_invents_events() {
        let response = track_shipment("1Z999AA10123456784".into(), None).unwrap();
        assert_eq!(response.records[0]["carrier"], "UPS");
        assert_eq!(
            response.records[0]["tracking_number_masked"],
            "1Z9************784"
        );
        assert_eq!(response.records[0]["tracking_events"], json!([]));
        assert_eq!(response.source_statuses[0].status, LiveSourceState::Skipped);
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("1Z999AA10123456784"));
    }

    #[test]
    fn ambiguous_shipment_number_requires_carrier() {
        let response = track_shipment("123456789012".into(), None).unwrap();
        assert_eq!(response.records[0]["automation_status"], "carrier_required");
        assert_eq!(response.records[0]["carrier"], Value::Null);
    }

    #[test]
    fn shipment_router_never_echoes_untrusted_carrier_input() {
        let tracking_number = "SFTEST123456";
        let response =
            track_shipment(tracking_number.into(), Some(tracking_number.into())).unwrap();
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains(tracking_number));

        let response = track_shipment(tracking_number.into(), None).unwrap();
        assert_eq!(response.records[0]["automation_status"], "carrier_required");
    }

    #[test]
    fn flight_fixture_maps_feed_and_observation_times_separately() {
        let state = vec![
            json!("abc123"),
            json!("CALL123 "),
            json!("Testland"),
            json!(100),
            json!(110),
            json!(121.5),
            json!(31.2),
            json!(1000.0),
            json!(false),
            json!(200.0),
            json!(90.0),
            json!(1.0),
            Value::Null,
            json!(1100.0),
            json!("1234"),
            json!(false),
            json!(0),
            json!(false),
        ];
        let record = flight_record(&state, Some(120)).unwrap();
        assert_eq!(record["feed_time_unix"], 120);
        assert_eq!(record["time_position_unix"], 100);
        assert_eq!(record["last_contact_unix"], 110);
        assert_eq!(record["callsign"], "CALL123");
    }

    #[test]
    fn market_codes_are_strict_and_bounded() {
        assert_eq!(currency_code(Some(" usd "), "base").unwrap(), "USD");
        assert!(currency_code(Some("USDT"), "base").is_err());
        assert_eq!(asset_code(Some("btc"), "base").unwrap(), "BTC");
        assert!(asset_code(Some("BTC/USD"), "base").is_err());
    }

    #[test]
    fn coordinate_validation_rejects_partial_or_invalid_values() {
        assert!(coordinates(Some(31.2), None).is_err());
        assert!(coordinates(Some(91.0), Some(0.0)).is_err());
        assert_eq!(coordinates(Some(31.2), Some(121.5)).unwrap(), (31.2, 121.5));
    }

    #[test]
    fn winnipeg_counts_keep_station_interval_semantics_and_deduplicate_sites() {
        let latest = (Utc::now().with_timezone(&chrono_tz::America::Winnipeg)
            - ChronoDuration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let earlier = (Utc::now().with_timezone(&chrono_tz::America::Winnipeg)
            - ChronoDuration::minutes(70))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let payload = vec![
            json!({
                "timestamp": latest,
                "site": "Station A",
                "latitude": "49.8951",
                "longitude": "-97.1384",
                "total": "120",
                "northbound": "70",
                "southbound": "40",
                "left": "55",
                "right": "65"
            }),
            json!({
                "timestamp": earlier,
                "site": "Station A",
                "latitude": "49.8951",
                "longitude": "-97.1384",
                "total": "99"
            }),
            json!({
                "timestamp": latest,
                "site": "Station B",
                "latitude": "49.9000",
                "longitude": "-97.1400",
                "total": "70"
            }),
        ];

        let data = parse_winnipeg_counts(&payload, (49.8951, -97.1384), 5.0, 10);

        assert_eq!(data.records.len(), 2);
        assert_eq!(data.records[0]["station_name"], "Station A");
        assert_eq!(data.records[0]["vehicle_count"], 120);
        assert_eq!(data.records[0]["directional_counts"]["northbound"], 70);
        assert_eq!(data.records[0]["directional_counts"]["southbound"], 40);
        assert_eq!(
            data.records[0]["count_unit"],
            "provider_vehicle_count_for_timestamp_interval"
        );
        assert_eq!(data.records[0]["count_interval_duration"], Value::Null);
        assert_eq!(data.records[1]["station_name"], "Station B");
        assert_eq!(data.records[1]["vehicle_count"], 70);
    }

    #[test]
    fn traffic_speed_records_never_claim_a_vehicle_count() {
        let nyc_observed = (Utc::now().with_timezone(&chrono_tz::America::New_York)
            - ChronoDuration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let chicago_observed = (Utc::now().with_timezone(&chrono_tz::America::Chicago)
            - ChronoDuration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let nyc = parse_nyc_traffic_flow(
            &[json!({
                "link_id": "nyc-link",
                "link_points": "40.7500,-73.9900 40.7510,-73.9910",
                "speed": "24.5",
                "travel_time": "80",
                "data_as_of": nyc_observed
            })],
            (40.75, -73.99),
            2.0,
            5,
        );
        let chicago = parse_chicago_traffic_flow(
            &[json!({
                "segmentid": "chicago-segment",
                "_lif_lat": "41.8800",
                "start_lon": "-87.6300",
                "_lit_lat": "41.8820",
                "_lit_lon": "-87.6320",
                "_traffic": "19",
                "_last_updt": chicago_observed
            })],
            (41.881, -87.631),
            2.0,
            5,
        );

        assert_eq!(nyc.records.len(), 1);
        assert_eq!(nyc.records[0]["vehicle_count"], Value::Null);
        assert_eq!(nyc.records[0]["speed_provider_value"], 24.5);
        assert_eq!(
            nyc.records[0]["speed_unit"],
            "not_documented_in_dataset_schema"
        );
        assert_eq!(chicago.records.len(), 1);
        assert_eq!(chicago.records[0]["vehicle_count"], Value::Null);
        assert_eq!(chicago.records[0]["speed_provider_value"], 19.0);
        assert_eq!(
            chicago.records[0]["speed_unit"],
            "not_documented_in_dataset_schema"
        );
    }

    #[test]
    fn numeric_road_parsers_apply_source_specific_maximum_ages() {
        let winnipeg_old = (Utc::now().with_timezone(&chrono_tz::America::Winnipeg)
            - ChronoDuration::minutes(WINNIPEG_OBSERVATION_MAX_AGE_MINUTES + 1))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let nyc_old = (Utc::now().with_timezone(&chrono_tz::America::New_York)
            - ChronoDuration::minutes(NYC_OBSERVATION_MAX_AGE_MINUTES + 1))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let chicago_old = (Utc::now().with_timezone(&chrono_tz::America::Chicago)
            - ChronoDuration::minutes(CHICAGO_FLOW_OBSERVATION_MAX_AGE_MINUTES + 1))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let fintraffic_old = (Utc::now()
            - ChronoDuration::minutes(FINTRAFFIC_OBSERVATION_MAX_AGE_MINUTES + 1))
        .to_rfc3339();
        let station = NearbyRoadStation {
            id: "20002".into(),
            name: "Espoo test station".into(),
            latitude: 60.220898,
            longitude: 24.637997,
            distance_km: 1.25,
        };

        let winnipeg = parse_winnipeg_counts(
            &[json!({
                "timestamp": winnipeg_old,
                "site": "Station A",
                "latitude": "49.8951",
                "longitude": "-97.1384",
                "total": "120"
            })],
            (49.8951, -97.1384),
            5.0,
            5,
        );
        let nyc = parse_nyc_traffic_flow(
            &[json!({
                "link_id": "nyc-link",
                "link_points": "40.7500,-73.9900 40.7510,-73.9910",
                "speed": "24.5",
                "data_as_of": nyc_old
            })],
            (40.75, -73.99),
            2.0,
            5,
        );
        let chicago = parse_chicago_traffic_flow(
            &[json!({
                "segmentid": "chicago-segment",
                "_lif_lat": "41.8800",
                "start_lon": "-87.6300",
                "_lit_lat": "41.8820",
                "_lit_lon": "-87.6320",
                "_traffic": "19",
                "_last_updt": chicago_old
            })],
            (41.881, -87.631),
            2.0,
            5,
        );
        let fintraffic = parse_fintraffic_tms_flow(
            &json!({
                "sensorValues": [{
                    "id": 5016,
                    "name": "OHITUKSET_5MIN_KIINTEA_SUUNTA1",
                    "unit": "kpl/h",
                    "value": 100.0,
                    "measuredTime": fintraffic_old
                }]
            }),
            &station,
            5,
        );

        for data in [winnipeg, nyc, chicago, fintraffic] {
            assert!(data.stale);
            assert!(data.records.is_empty());
            assert_eq!(data.data_as_of_kind.as_deref(), Some("observation_time"));
        }
    }

    #[test]
    fn numeric_road_parsers_fail_closed_when_candidate_times_are_missing() {
        let station = NearbyRoadStation {
            id: "station-1".into(),
            name: "Test station".into(),
            latitude: 60.220898,
            longitude: 24.637997,
            distance_km: 1.25,
        };
        let winnipeg = parse_winnipeg_counts(
            &[json!({
                "site": "Station A",
                "latitude": "49.8951",
                "longitude": "-97.1384",
                "total": "120"
            })],
            (49.8951, -97.1384),
            5.0,
            5,
        );
        let nyc = parse_nyc_traffic_flow(
            &[json!({
                "link_id": "nyc-link",
                "link_points": "40.7500,-73.9900 40.7510,-73.9910",
                "speed": "24.5"
            })],
            (40.75, -73.99),
            2.0,
            5,
        );
        let chicago = parse_chicago_traffic_flow(
            &[json!({
                "segmentid": "chicago-segment",
                "_lif_lat": "41.8800",
                "start_lon": "-87.6300",
                "_lit_lat": "41.8820",
                "_lit_lon": "-87.6320",
                "_traffic": "19"
            })],
            (41.881, -87.631),
            2.0,
            5,
        );
        let fintraffic = parse_fintraffic_tms_flow(
            &json!({
                "sensorValues": [{
                    "id": 5016,
                    "name": "OHITUKSET_5MIN_KIINTEA_SUUNTA1",
                    "unit": "kpl/h",
                    "value": 100.0
                }]
            }),
            &station,
            5,
        );
        let norway = parse_norway_hourly_count(
            &json!({
                "data": { "trafficData": { "volume": { "byHour": { "edges": [{
                    "node": { "total": { "volumeNumbers": { "volume": 10 } } }
                }] } } } }
            }),
            &station,
        );

        for data in [winnipeg, nyc, chicago, fintraffic, norway] {
            assert!(data.stale);
            assert!(data.records.is_empty());
            assert!(data.data_as_of.is_none());
        }
    }

    #[test]
    fn nyc_flow_keeps_the_latest_snapshot_for_each_link() {
        let newest = (Utc::now().with_timezone(&chrono_tz::America::New_York)
            - ChronoDuration::minutes(5))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let second_link_time = (Utc::now().with_timezone(&chrono_tz::America::New_York)
            - ChronoDuration::minutes(15))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let old_duplicate = (Utc::now().with_timezone(&chrono_tz::America::New_York)
            - ChronoDuration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let payload = vec![
            json!({
                "link_id": "link-a",
                "link_points": "40.7500,-73.9900 40.7510,-73.9910",
                "speed": "25",
                "data_as_of": newest
            }),
            json!({
                "link_id": "link-b",
                "link_points": "40.7520,-73.9920 40.7530,-73.9930",
                "speed": "18",
                "data_as_of": second_link_time
            }),
            json!({
                "link_id": "link-a",
                "link_points": "40.7500,-73.9900 40.7510,-73.9910",
                "speed": "99",
                "data_as_of": old_duplicate
            }),
        ];

        let data = parse_nyc_traffic_flow(&payload, (40.751, -73.991), 5.0, 10);

        assert_eq!(data.records.len(), 2);
        assert!(data
            .records
            .iter()
            .any(|record| record["link_id"] == "link-a"
                && record["speed_provider_value"].as_f64() == Some(25.0)));
        assert!(data
            .records
            .iter()
            .any(|record| record["link_id"] == "link-b"
                && record["speed_provider_value"].as_f64() == Some(18.0)));
        assert!(!data
            .records
            .iter()
            .any(|record| record["speed_provider_value"].as_f64() == Some(99.0)));
    }

    #[test]
    fn nyc_polyline_rejects_out_of_region_coordinate_outliers() {
        let distance = nyc_polyline_distance(
            "40.7500,-73.9900 40.7510,-73.9910 40.7500,-74.8410",
            (40.7500, -74.8410),
        )
        .unwrap();
        assert!(distance.0 > 50.0);
        assert!(
            nyc_polyline_distance("40.7500,-74.8410 40.7510,-74.8420", (40.7500, -74.8410))
                .is_none()
        );
    }

    #[test]
    fn drivebc_geometry_is_filtered_by_nearest_vertex_radius() {
        let payload = json!({
            "events": [
                {
                    "id": "near-line",
                    "updated": "2026-07-12T12:00:00Z",
                    "geography": {
                        "type": "LineString",
                        "coordinates": [[-123.5000, 49.5000], [-123.1207, 49.2827]]
                    }
                },
                {
                    "id": "far-point",
                    "updated": "2026-07-12T12:01:00Z",
                    "geography": {
                        "type": "Point",
                        "coordinates": [-122.0000, 50.0000]
                    }
                }
            ]
        });

        let data = parse_drivebc_incidents(&payload, (49.2827, -123.1207), 5.0, 10).unwrap();

        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0]["provider_event_id"], "near-line");
        assert_eq!(data.records[0]["distance_basis"], "nearest_geometry_vertex");
        assert_eq!(data.records[0]["distance_km"], 0.0);
    }

    #[test]
    fn fintraffic_geometry_is_filtered_by_nearest_vertex_radius() {
        let updated = (Utc::now() - ChronoDuration::minutes(2)).to_rfc3339();
        let payload = json!({
            "dataUpdatedTime": updated,
            "features": [
                {
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[25.5000, 61.0000], [24.9384, 60.1699]]
                    },
                    "properties": {
                        "situationId": "near-line",
                        "trafficAnnouncementType": "general",
                        "versionTime": updated,
                        "announcements": [{
                            "language": "en",
                            "title": "Nearby event"
                        }]
                    }
                },
                {
                    "geometry": {
                        "type": "Point",
                        "coordinates": [27.0000, 65.0000]
                    },
                    "properties": {
                        "situationId": "far-point",
                        "versionTime": updated,
                        "announcements": [{
                            "language": "en",
                            "title": "Far event"
                        }]
                    }
                }
            ]
        });

        let data = parse_fintraffic_announcements(&payload, (60.1699, 24.9384), 5.0, 10).unwrap();

        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0]["provider_event_id"], "near-line");
        assert_eq!(data.records[0]["title"], "Nearby event");
        assert_eq!(data.records[0]["provider_announcement_type"], "general");
        assert_eq!(data.records[0]["incident_type"], Value::Null);
        assert_eq!(data.records[0]["distance_basis"], "nearest_geometry_vertex");
        assert_eq!(data.records[0]["distance_km"], 0.0);
    }

    #[test]
    fn fintraffic_ended_tombstones_are_not_returned_as_current_events() {
        let updated = (Utc::now() - ChronoDuration::minutes(2)).to_rfc3339();
        let payload = json!({
            "dataUpdatedTime": updated,
            "features": [{
                "geometry": { "type": "Point", "coordinates": [24.9384, 60.1699] },
                "properties": {
                    "situationId": "ended-event",
                    "trafficAnnouncementType": "ended",
                    "versionTime": updated,
                    "announcements": [{ "language": "en", "features": ["collision"] }]
                }
            }]
        });

        let data = parse_fintraffic_announcements(&payload, (60.1699, 24.9384), 5.0, 10).unwrap();

        assert!(data.records.is_empty());
        assert!(!data.stale);
    }

    #[test]
    fn road_geometry_uses_a_crossing_segment_not_only_distant_vertices() {
        let geometry = json!({
            "type": "LineString",
            "coordinates": [[-0.25, 51.5], [0.25, 51.5]]
        });

        let (distance_km, latitude, longitude, basis) =
            geometry_distance(Some(&geometry), (51.5, 0.0)).unwrap();

        assert!(distance_km < 0.001);
        assert!((latitude - 51.5).abs() < 0.000_001);
        assert!(longitude.abs() < 0.000_001);
        assert_eq!(basis, "nearest_geometry_segment");
    }

    #[test]
    fn road_geometry_handles_polygon_containment_holes_and_multipolygons() {
        let polygon = json!({
            "type": "Polygon",
            "coordinates": [[
                [-0.2, 51.4], [0.2, 51.4], [0.2, 51.6], [-0.2, 51.6], [-0.2, 51.4]
            ]]
        });
        let inside = geometry_distance(Some(&polygon), (51.5, 0.0)).unwrap();
        assert_eq!(inside, (0.0, 51.5, 0.0, "inside_reported_polygon"));

        let polygon_with_hole = json!({
            "type": "Polygon",
            "coordinates": [
                [[-0.2, 51.4], [0.2, 51.4], [0.2, 51.6], [-0.2, 51.6], [-0.2, 51.4]],
                [[-0.02, 51.48], [0.02, 51.48], [0.02, 51.52], [-0.02, 51.52], [-0.02, 51.48]]
            ]
        });
        let in_hole = geometry_distance(Some(&polygon_with_hole), (51.5, 0.0)).unwrap();
        assert!(in_hole.0 > 1.0);
        assert_ne!(in_hole.3, "inside_reported_polygon");

        let multipolygon = json!({
            "type": "MultiPolygon",
            "coordinates": [
                [[[-2.0, 50.0], [-1.8, 50.0], [-1.8, 50.2], [-2.0, 50.2], [-2.0, 50.0]]],
                [[[-0.2, 51.4], [0.2, 51.4], [0.2, 51.6], [-0.2, 51.6], [-0.2, 51.4]]]
            ]
        });
        let inside_multi = geometry_distance(Some(&multipolygon), (51.5, 0.0)).unwrap();
        assert_eq!(inside_multi, (0.0, 51.5, 0.0, "inside_reported_polygon"));
    }

    #[test]
    fn road_event_parsers_reject_missing_or_malformed_root_arrays() {
        assert!(parse_drivebc_incidents(&json!({}), (49.28, -123.12), 5.0, 10).is_err());
        assert!(parse_drivebc_incidents(
            &json!({ "events": "not-an-array" }),
            (49.28, -123.12),
            5.0,
            10
        )
        .is_err());
        assert!(parse_drivebc_incidents(
            &json!({ "events": ["not-an-object"] }),
            (49.28, -123.12),
            5.0,
            10
        )
        .is_err());
        assert!(
            parse_drivebc_incidents(&json!({ "events": [] }), (49.28, -123.12), 5.0, 10)
                .unwrap()
                .records
                .is_empty()
        );

        assert!(parse_fintraffic_announcements(&json!({}), (60.17, 24.94), 5.0, 10).is_err());
        assert!(parse_fintraffic_announcements(
            &json!({ "features": "not-an-array" }),
            (60.17, 24.94),
            5.0,
            10
        )
        .is_err());
        assert!(parse_fintraffic_announcements(
            &json!({ "features": ["not-an-object"] }),
            (60.17, 24.94),
            5.0,
            10
        )
        .is_err());
        assert!(parse_fintraffic_announcements(
            &json!({ "features": [] }),
            (60.17, 24.94),
            5.0,
            10
        )
        .unwrap()
        .records
        .is_empty());
    }

    #[test]
    fn country_gates_exclude_st_petersburg_from_finland_and_norway() {
        assert!(polygon_may_cover(
            (60.1699, 24.9384),
            10.0,
            FINLAND_MAINLAND_GATE
        ));
        assert!(polygon_may_cover(
            (63.095, 21.616),
            10.0,
            FINLAND_MAINLAND_GATE
        ));
        assert!(!polygon_may_cover(
            (63.825, 20.263),
            10.0,
            FINLAND_MAINLAND_GATE
        ));
        assert!(polygon_may_cover(
            (59.9139, 10.7522),
            10.0,
            NORWAY_MAINLAND_GATE
        ));
        assert!(!polygon_may_cover(
            (59.9311, 30.3609),
            10.0,
            FINLAND_MAINLAND_GATE
        ));
        assert!(!polygon_may_cover(
            (59.9311, 30.3609),
            10.0,
            NORWAY_MAINLAND_GATE
        ));
    }

    #[test]
    fn official_bc_gate_includes_islands_and_excludes_nearby_us_cities() {
        assert!(drivebc_may_cover((54.0111, -132.1460), 1.0)); // Masset
        assert!(drivebc_may_cover((48.4284, -123.3656), 1.0)); // Victoria
        assert!(drivebc_may_cover((54.3150, -130.3208), 1.0)); // Prince Rupert
        assert!(!drivebc_may_cover((58.3019, -134.4197), 1.0)); // Juneau
        assert!(!drivebc_may_cover((48.7519, -122.4787), 1.0)); // Bellingham
        assert!(!drivebc_may_cover((55.3422, -131.6461), 1.0)); // Ketchikan
    }

    #[test]
    fn observation_window_enforces_future_delayed_and_stale_boundaries() {
        let now = DateTime::parse_from_rfc3339("2026-07-12T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let near_future = (now + ChronoDuration::minutes(5)).to_rfc3339();
        let invalid_future =
            (now + ChronoDuration::minutes(5) + ChronoDuration::seconds(1)).to_rfc3339();
        let delayed = (now - ChronoDuration::minutes(16)).to_rfc3339();
        let stale = (now - ChronoDuration::minutes(61)).to_rfc3339();

        assert_eq!(
            observation_window_at(Some(&near_future), chrono_tz::UTC, 60, now).state,
            ObservationWindowState::NearRealTime
        );
        assert_eq!(
            observation_window_at(Some(&invalid_future), chrono_tz::UTC, 60, now).state,
            ObservationWindowState::Future
        );
        assert_eq!(
            observation_window_at(Some(&delayed), chrono_tz::UTC, 60, now).state,
            ObservationWindowState::Delayed
        );
        assert_eq!(
            observation_window_at(Some(&stale), chrono_tz::UTC, 60, now).state,
            ObservationWindowState::Stale
        );
    }

    #[test]
    fn dynamic_road_observations_fail_closed_on_old_or_missing_time() {
        let old = (Utc::now() - ChronoDuration::hours(3)).to_rfc3339();
        let stale = time_sensitive_road_provider_data(
            vec![json!({ "value": 1 })],
            Some(old),
            "observation_time",
            chrono_tz::UTC,
            60,
            true,
        );
        assert!(stale.stale);
        assert!(stale.records.is_empty());

        let missing = time_sensitive_road_provider_data(
            vec![json!({ "value": 1 })],
            None,
            "observation_time",
            chrono_tz::UTC,
            60,
            true,
        );
        assert!(missing.stale);
        assert!(missing.records.is_empty());

        let genuinely_empty = time_sensitive_road_provider_data(
            Vec::new(),
            None,
            "observation_time",
            chrono_tz::UTC,
            60,
            false,
        );
        assert!(!genuinely_empty.stale);
        assert!(genuinely_empty.records.is_empty());

        let mut records = Vec::new();
        let mut statuses = Vec::new();
        merge_road_provider(
            "stale_test_source",
            true,
            true,
            Some(Ok(stale)),
            "Records were returned.",
            "No coverage.",
            &mut records,
            &mut statuses,
        );
        assert_eq!(statuses[0].status, LiveSourceState::Stale);
        assert_eq!(statuses[0].freshness.as_deref(), Some("stale"));
    }

    #[test]
    fn future_observations_cannot_produce_a_success_source_status() {
        let future = (Utc::now() + ChronoDuration::minutes(6)).to_rfc3339();
        let data = time_sensitive_road_provider_data(
            vec![json!({ "value": 1 })],
            Some(future),
            "observation_time",
            chrono_tz::UTC,
            60,
            true,
        );
        let mut records = Vec::new();
        let mut statuses = Vec::new();

        merge_road_provider(
            "future_test_source",
            true,
            true,
            Some(Ok(data)),
            "Records were returned.",
            "No coverage.",
            &mut records,
            &mut statuses,
        );

        assert!(records.is_empty());
        assert_eq!(statuses[0].status, LiveSourceState::Stale);
        assert!(statuses[0]
            .detail
            .contains("more than 5 minutes in the future"));
        assert_eq!(
            statuses[0].data_as_of_kind.as_deref(),
            Some("observation_time")
        );
    }

    #[test]
    fn accepted_numeric_observations_older_than_fifteen_minutes_are_delayed() {
        let observed_at = (Utc::now() - ChronoDuration::minutes(20)).to_rfc3339();
        let data = time_sensitive_road_provider_data(
            vec![json!({ "value": 1 })],
            Some(observed_at),
            "aggregation_interval_end",
            chrono_tz::UTC,
            8 * 60,
            true,
        );
        let mut records = Vec::new();
        let mut statuses = Vec::new();

        merge_road_provider(
            "delayed_test_source",
            true,
            true,
            Some(Ok(data)),
            "Records were returned.",
            "No coverage.",
            &mut records,
            &mut statuses,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(statuses[0].status, LiveSourceState::Delayed);
        assert_eq!(statuses[0].freshness.as_deref(), Some("delayed"));
        assert_eq!(
            statuses[0].data_as_of_kind.as_deref(),
            Some("aggregation_interval_end")
        );
    }

    #[tokio::test]
    async fn road_directory_cache_never_caches_dynamic_observations() {
        let cache: &'static RwLock<Option<CachedDirectory>> =
            Box::leak(Box::new(RwLock::new(None)));
        let (first, first_state, first_age) =
            cached_road_directory(cache, Duration::from_secs(60), async {
                Ok(json!({ "directory": 1 }))
            })
            .await
            .unwrap();
        assert_eq!(first["directory"], 1);
        assert_eq!(first_state, "directory_miss_dynamic_bypassed");
        assert_eq!(first_age, 0);

        let (second, second_state, _) =
            cached_road_directory(cache, Duration::from_secs(60), async {
                Err("the cache hit must not poll this fetch".into())
            })
            .await
            .unwrap();
        assert_eq!(second["directory"], 1);
        assert_eq!(second_state, "directory_hit_dynamic_bypassed");
    }

    #[tokio::test]
    async fn road_directory_cache_refresh_is_single_flight() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        let cache: &'static RwLock<Option<CachedDirectory>> =
            Box::leak(Box::new(RwLock::new(None)));
        let fetches = Arc::new(AtomicUsize::new(0));
        let first_fetches = Arc::clone(&fetches);
        let second_fetches = Arc::clone(&fetches);

        let (first, second) = tokio::join!(
            cached_road_directory(cache, Duration::from_secs(60), async move {
                first_fetches.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(json!({ "directory": 1 }))
            }),
            cached_road_directory(cache, Duration::from_secs(60), async move {
                second_fetches.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(json!({ "directory": 2 }))
            }),
        );

        assert_eq!(fetches.load(Ordering::SeqCst), 1);
        assert_eq!(first.unwrap().0, second.unwrap().0);
    }

    #[test]
    fn empty_road_source_status_never_uses_a_success_claim() {
        let mut records = Vec::new();
        let mut statuses = Vec::new();
        merge_road_provider(
            "test_source",
            true,
            true,
            Some(Ok(road_provider_data(
                Vec::new(),
                None,
                "event_updated_at",
                false,
                chrono_tz::UTC,
            ))),
            "Records were returned.",
            "No coverage.",
            &mut records,
            &mut statuses,
        );
        assert!(records.is_empty());
        assert_eq!(statuses[0].status, LiveSourceState::Empty);
        assert!(statuses[0].detail.contains("no locally matching records"));
        assert!(!statuses[0].detail.contains("Records were returned"));
    }

    #[test]
    fn future_incident_times_do_not_pass_lookback_filters() {
        let austin_future = (Utc::now() + ChronoDuration::minutes(10)).to_rfc3339();
        let austin_payload = vec![json!({
            "traffic_report_id": "future-austin",
            "published_date": austin_future,
            "traffic_report_status_date_time": austin_future,
            "latitude": "30.2672",
            "longitude": "-97.7431"
        })];
        let austin = parse_austin_incidents(&austin_payload, (30.2672, -97.7431), 2.0, 24, 5);
        assert!(austin.records.is_empty());
        assert!(austin.data_as_of.is_none());

        let chicago_future = (Utc::now().with_timezone(&chrono_tz::America::Chicago)
            + ChronoDuration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let chicago_payload = vec![json!({
            "crash_record_id": "future-chicago",
            "crash_date": chicago_future,
            "latitude": "41.8810",
            "longitude": "-87.6310"
        })];
        let chicago = parse_chicago_crashes(&chicago_payload, (41.881, -87.631), 2.0, 24, 5);
        assert!(chicago.records.is_empty());
        assert!(chicago.data_as_of.is_none());
    }

    #[test]
    fn chicago_crash_event_time_is_distinct_from_police_notification_time() {
        let crash_at = (Utc::now().with_timezone(&chrono_tz::America::Chicago)
            - ChronoDuration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let police_notified_at = (Utc::now().with_timezone(&chrono_tz::America::Chicago)
            - ChronoDuration::minutes(5))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let payload = vec![json!({
            "crash_record_id": "crash-1",
            "crash_date": crash_at,
            "date_police_notified": police_notified_at,
            "latitude": "41.8810",
            "longitude": "-87.6310"
        })];

        let data = parse_chicago_crashes(&payload, (41.881, -87.631), 2.0, 1, 5);

        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0]["event_time_local"], crash_at);
        assert_eq!(
            data.records[0]["police_notified_at_local"],
            police_notified_at
        );
        assert_eq!(data.data_as_of.as_deref(), Some(crash_at.as_str()));
        assert_eq!(data.data_as_of_kind.as_deref(), Some("event_time"));
    }

    #[test]
    fn fintraffic_tms_keeps_flow_rate_separate_from_vehicle_count() {
        let measured_at = (Utc::now() - ChronoDuration::minutes(2)).to_rfc3339();
        let station = NearbyRoadStation {
            id: "20002".into(),
            name: "Espoo test station".into(),
            latitude: 60.220898,
            longitude: 24.637997,
            distance_km: 1.25,
        };
        let payload = json!({
            "dataUpdatedTime": measured_at,
            "sensorValues": [
                {
                    "id": 5016,
                    "name": "OHITUKSET_5MIN_KIINTEA_SUUNTA1",
                    "unit": "kpl/h",
                    "value": 1416.0,
                    "timeWindowStart": "2026-07-12T14:15:00Z",
                    "timeWindowEnd": "2026-07-12T14:20:00Z",
                    "measuredTime": measured_at
                },
                {
                    "id": 5056,
                    "name": "KESKINOPEUS_60MIN_KIINTEA_SUUNTA1",
                    "unit": "km/h",
                    "value": 98.0,
                    "timeWindowStart": "2026-07-12T13:00:00Z",
                    "timeWindowEnd": "2026-07-12T14:00:00Z",
                    "measuredTime": measured_at
                }
            ]
        });

        let data = parse_fintraffic_tms_flow(&payload, &station, 10);

        assert_eq!(data.records.len(), 2);
        assert_eq!(data.records[0]["metric"], "vehicle_flow_rate");
        assert_eq!(data.records[0]["flow_rate_vehicles_per_hour"], 1416.0);
        assert_eq!(data.records[0]["vehicle_count"], Value::Null);
        assert_eq!(data.records[0]["time_window_start"], "2026-07-12T14:15:00Z");
        assert_eq!(data.records[1]["metric"], "average_speed");
        assert_eq!(data.records[1]["average_speed_kmh"], 98.0);
    }

    #[test]
    fn norway_hourly_count_uses_latest_interval_and_preserves_coverage() {
        let latest_end = (Utc::now() - ChronoDuration::hours(3)).to_rfc3339();
        let latest_start = (Utc::now() - ChronoDuration::hours(4)).to_rfc3339();
        let earlier_end = latest_start.clone();
        let earlier_start = (Utc::now() - ChronoDuration::hours(5)).to_rfc3339();
        let station = NearbyRoadStation {
            id: "station-1".into(),
            name: "Norway test station".into(),
            latitude: 60.4,
            longitude: 11.2,
            distance_km: 2.5,
        };
        let payload = json!({
            "data": { "trafficData": { "volume": { "byHour": { "edges": [
                { "node": {
                    "from": earlier_start,
                    "to": earlier_end,
                    "total": {
                        "coverage": { "percentage": 80.0, "unit": "HOUR" },
                        "volumeNumbers": { "volume": 100 }
                    }
                } },
                { "node": {
                    "from": latest_start,
                    "to": latest_end,
                    "total": {
                        "coverage": { "percentage": 100.0, "unit": "HOUR" },
                        "volumeNumbers": { "volume": 321 }
                    }
                } }
            ] } } } }
        });

        let data = parse_norway_hourly_count(&payload, &station);

        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0]["vehicle_count"], 321);
        assert_eq!(data.records[0]["coverage_percentage"], 100.0);
        assert_eq!(data.records[0]["count_interval_end"], latest_end);
        assert_eq!(data.records[0]["count_interval_duration"], "PT1H");
        assert!(data.delayed);
        assert_eq!(
            data.data_as_of_kind.as_deref(),
            Some("aggregation_interval_end")
        );
    }

    #[test]
    fn tfl_disruptions_are_locally_radius_filtered() {
        let payload = vec![
            json!({
                "id": "near",
                "category": "Traffic Incidents",
                "subCategory": "Collision",
                "lastModifiedTime": "2026-07-12T14:11:38Z",
                "geography": { "type": "Point", "coordinates": [-0.1276, 51.5072] }
            }),
            json!({
                "id": "far",
                "category": "Works",
                "lastModifiedTime": "2026-07-12T14:12:00Z",
                "geography": { "type": "Point", "coordinates": [-0.5000, 51.8000] }
            }),
        ];

        let data = parse_tfl_disruptions(&payload, (51.5072, -0.1276), 5.0, 10);

        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0]["provider_event_id"], "near");
        assert_eq!(data.records[0]["incident_subtype"], "Collision");
    }

    #[tokio::test]
    async fn shanghai_road_environment_returns_no_coverage_without_network_wait() {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            road_environment(
                "overview".into(),
                31.2304,
                121.4737,
                Some(10),
                Some(24),
                Some(5),
            ),
        )
        .await
        .expect("no-coverage routing should not wait for a network request")
        .unwrap();

        assert!(response.records.is_empty());
        for source in [
            "winnipeg_permanent_count_stations",
            "norway_public_roads_traffic_data",
            "fintraffic_tms_sensor_values",
            "transport_for_london_road_disruptions",
        ] {
            assert!(response
                .source_statuses
                .iter()
                .any(|status| status.source == source));
        }
        assert!(response
            .source_statuses
            .iter()
            .all(|status| status.status == LiveSourceState::NoCoverage));
    }

    #[tokio::test]
    #[ignore = "calls the live Caltrans QuickMap CHP public feed"]
    async fn live_caltrans_chp_returns_current_feed_evidence() {
        let response = road_environment(
            "road_incidents".into(),
            34.0522,
            -118.2437,
            Some(100),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        let status = response
            .source_statuses
            .iter()
            .find(|status| status.source == "caltrans_quickmap_chp_incidents")
            .expect("Caltrans CHP source status should be present");

        assert!(matches!(
            status.status,
            LiveSourceState::Success | LiveSourceState::Empty
        ));
        assert!(status.data_as_of.is_some());
        assert_eq!(
            status.data_as_of_kind.as_deref(),
            Some("http_last_modified")
        );
    }

    #[tokio::test]
    #[ignore = "calls live public keyless road-data sources"]
    async fn live_road_environment_sources_return_provider_evidence() {
        let winnipeg = road_environment(
            "vehicle_counts".into(),
            49.951733,
            -97.149032,
            Some(10),
            Some(24),
            Some(5),
        )
        .await
        .unwrap();
        assert!(winnipeg.source_statuses.iter().any(|status| {
            status.source == "winnipeg_permanent_count_stations"
                && matches!(
                    status.status,
                    LiveSourceState::Success | LiveSourceState::Delayed | LiveSourceState::Stale
                )
                && status.data_as_of.is_some()
                && status.data_as_of_kind.as_deref() == Some("observation_time")
        }));

        let norway = road_environment(
            "vehicle_counts".into(),
            60.41426,
            11.241171,
            Some(10),
            Some(24),
            Some(5),
        )
        .await
        .unwrap();
        assert!(norway.source_statuses.iter().any(|status| {
            status.source == "norway_public_roads_traffic_data"
                && matches!(
                    status.status,
                    LiveSourceState::Success | LiveSourceState::Delayed
                )
                && status.data_as_of_kind.as_deref() == Some("aggregation_interval_end")
        }));
        assert!(norway.records.iter().any(|record| {
            record.get("source").and_then(Value::as_str) == Some("norway_public_roads_traffic_data")
                && record
                    .get("vehicle_count")
                    .and_then(Value::as_i64)
                    .is_some()
        }));

        let finland_flow = road_environment(
            "traffic_flow".into(),
            60.220898,
            24.637997,
            Some(10),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(finland_flow.source_statuses.iter().any(|status| {
            status.source == "fintraffic_tms_sensor_values"
                && status.status == LiveSourceState::Success
        }));
        assert!(finland_flow.records.iter().any(|record| {
            record.get("source").and_then(Value::as_str) == Some("fintraffic_tms_sensor_values")
                && record.get("vehicle_count") == Some(&Value::Null)
        }));
        let finland_flow_again = road_environment(
            "traffic_flow".into(),
            60.220898,
            24.637997,
            Some(10),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(finland_flow_again.source_statuses.iter().any(|status| {
            status.source == "fintraffic_tms_sensor_values"
                && status.cache_state.as_deref() == Some("directory_hit_dynamic_bypassed")
        }));

        let nyc = road_environment(
            "traffic_flow".into(),
            40.7500,
            -73.9900,
            Some(20),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(nyc.source_statuses.iter().any(|status| {
            status.source == "nyc_dot_traffic_speeds"
                && matches!(
                    status.status,
                    LiveSourceState::Success
                        | LiveSourceState::Delayed
                        | LiveSourceState::Empty
                        | LiveSourceState::Stale
                )
        }));
        assert!(nyc.records.iter().all(|record| {
            record.get("source").and_then(Value::as_str) != Some("nyc_dot_traffic_speeds")
                || record.get("speed_unit").and_then(Value::as_str)
                    == Some("not_documented_in_dataset_schema")
        }));

        let finland_incidents = road_environment(
            "road_incidents".into(),
            60.1699,
            24.9384,
            Some(100),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(finland_incidents.source_statuses.iter().any(|status| {
            status.source == "fintraffic_traffic_announcements_v2"
                && matches!(
                    status.status,
                    LiveSourceState::Success | LiveSourceState::Empty
                )
        }));

        let london = road_environment(
            "road_incidents".into(),
            51.5072,
            -0.1276,
            Some(100),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(london.source_statuses.iter().any(|status| {
            status.source == "transport_for_london_road_disruptions"
                && status.status == LiveSourceState::Success
        }));

        let drivebc = road_environment(
            "road_incidents".into(),
            49.2827,
            -123.1207,
            Some(100),
            Some(24),
            Some(10),
        )
        .await
        .unwrap();
        assert!(drivebc.source_statuses.iter().any(|status| {
            status.source == "drivebc_open511"
                && matches!(
                    status.status,
                    LiveSourceState::Success | LiveSourceState::Empty
                )
                && status.data_as_of_kind.as_deref() == Some("event_updated_at")
        }));
    }

    #[tokio::test]
    #[ignore = "calls live public keyless data sources"]
    async fn live_keyless_sources_return_structured_statuses() {
        let succeeded = |response: &LiveDataResponse| {
            response
                .source_statuses
                .iter()
                .any(|status| status.status == LiveSourceState::Success)
        };
        let weather = live_environment(
            "weather".into(),
            Some(31.23),
            Some(121.47),
            None,
            None,
            None,
            None,
            Some(2),
        )
        .await
        .unwrap();
        assert!(succeeded(&weather));
        let air = live_environment(
            "air_quality".into(),
            Some(31.23),
            Some(121.47),
            None,
            None,
            None,
            None,
            Some(2),
        )
        .await
        .unwrap();
        assert!(succeeded(&air));
        let marine = live_environment(
            "marine".into(),
            Some(37.77),
            Some(-122.52),
            None,
            None,
            None,
            None,
            Some(2),
        )
        .await
        .unwrap();
        assert!(succeeded(&marine));
        let fx = live_markets(
            "exchange_rate".into(),
            Some("USD".into()),
            Some("CNY".into()),
        )
        .await
        .unwrap();
        assert!(succeeded(&fx));
        let crypto = live_markets("crypto".into(), Some("BTC".into()), Some("USD".into()))
            .await
            .unwrap();
        assert_eq!(crypto.source_statuses.len(), 2);
        assert!(succeeded(&crypto));
        let earthquakes = live_environment(
            "earthquakes".into(),
            None,
            None,
            None,
            Some("day".into()),
            Some(4.5),
            None,
            Some(2),
        )
        .await
        .unwrap();
        assert!(
            succeeded(&earthquakes)
                || earthquakes.source_statuses[0].status == LiveSourceState::Empty
        );
        let hazards = live_environment(
            "natural_hazards".into(),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(2),
        )
        .await
        .unwrap();
        assert!(succeeded(&hazards));
        let flights = live_flights(48.85, 2.35, Some(50), Some(2)).await.unwrap();
        assert!(succeeded(&flights));
    }
}
