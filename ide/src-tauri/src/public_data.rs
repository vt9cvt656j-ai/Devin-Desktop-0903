use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECORDS: usize = 50;

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
pub enum LiveSourceState {
    Success,
    Empty,
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
    }
}

fn response(
    topic: &str,
    records: Vec<Value>,
    source_statuses: Vec<LiveSourceStatus>,
    mut limitations: Vec<String>,
) -> LiveDataResponse {
    limitations.push(
        "retrieved_at is when Michael IDE completed this request, not the provider's observation, publication, market, flight, or shipment event time."
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
        .map_err(|error| format!("{source} request failed: {error}"))?;
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
        .map_err(|error| format!("{source} response read failed: {error}"))?
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
