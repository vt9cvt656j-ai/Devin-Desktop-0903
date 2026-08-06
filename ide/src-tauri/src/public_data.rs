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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LiveSourceStatus {
    pub source: String,
    pub status: LiveSourceState,
    pub result_count: usize,
    pub detail: String,
    pub data_as_of: Option<String>,
    /// Optional provider timestamp semantics. Kept in the IPC shape so callers
    /// can distinguish provider time from the local retrieval time.
    pub data_as_of_kind: Option<String>,
    /// Optional freshness metadata supplied only when the provider exposes
    /// enough timestamp information to derive it.
    pub freshness: Option<String>,
    pub provider_time_age_seconds: Option<u64>,
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

fn response(
    topic: &str,
    records: Vec<Value>,
    source_statuses: Vec<LiveSourceStatus>,
    mut limitations: Vec<String>,
) -> LiveDataResponse {
    limitations.push(
        "retrieved_at is when Michael IDE completed this request, not the provider's model-validity, publication, observation, or event time."
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
    let category = category.map(str::trim).filter(|value| !value.is_empty());
    if let Some(category) = category {
        if !category
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(
                "natural_hazards category may contain only letters, digits, '-' or '_'".into(),
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earthquake_fixture_preserves_provider_times() {
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
    fn earthquake_filter_applies_magnitude_radius_and_limit() {
        let payload = json!({
            "features": [
                {
                    "id": "near",
                    "properties": { "mag": 3.0, "time": 1, "updated": 2 },
                    "geometry": { "coordinates": [121.5, 31.2, 10.0] }
                },
                {
                    "id": "weak",
                    "properties": { "mag": 1.0, "time": 1, "updated": 2 },
                    "geometry": { "coordinates": [121.5, 31.2, 10.0] }
                },
                {
                    "id": "far",
                    "properties": { "mag": 6.0, "time": 1, "updated": 2 },
                    "geometry": { "coordinates": [0.0, 0.0, 10.0] }
                }
            ]
        });

        let records = parse_earthquakes(&payload, 2.5, Some((31.2, 121.5, 20.0)), 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["id"], "near");
    }

    #[test]
    fn earthquake_feed_keeps_the_requested_magnitude_floor() {
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
    fn coordinate_and_limit_validation_are_bounded() {
        assert!(coordinates(Some(31.2), None).is_err());
        assert!(coordinates(Some(91.0), Some(0.0)).is_err());
        assert_eq!(coordinates(Some(31.2), Some(121.5)).unwrap(), (31.2, 121.5));
        assert_eq!(bounded_limit(Some(0), 12), 1);
        assert_eq!(bounded_limit(Some(500), 12), MAX_RECORDS);
    }

    #[tokio::test]
    async fn invalid_environment_requests_fail_before_network_access() {
        let invalid_kind =
            live_environment("markets".into(), None, None, None, None, None, None, None)
                .await
                .unwrap_err();
        assert!(invalid_kind.contains("kind must be"));

        let missing_coordinate = live_environment(
            "weather".into(),
            Some(31.2),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(missing_coordinate.contains("both required"));

        let invalid_category = live_environment(
            "natural_hazards".into(),
            None,
            None,
            None,
            None,
            None,
            Some("wild fires/unsafe".into()),
            None,
        )
        .await
        .expect("provider failures are returned as structured statuses");
        assert_eq!(
            invalid_category.source_statuses[0].status,
            LiveSourceState::Failed
        );
        assert!(invalid_category.source_statuses[0]
            .detail
            .contains("category may contain only"));
    }
}
