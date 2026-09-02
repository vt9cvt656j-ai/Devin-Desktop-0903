use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

const NOMINATIM_REVERSE_URL: &str = "https://nominatim.openstreetmap.org/reverse";
const ARCGIS_REVERSE_URL: &str =
    "https://geocode.arcgis.com/arcgis/rest/services/World/GeocodeServer/reverseGeocode";
const SOURCE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 512 * 1024;

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
pub enum ReverseSourceState {
    Success,
    Empty,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReverseSourceStatus {
    pub source: String,
    pub status: ReverseSourceState,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReverseAddressCandidate {
    pub source: String,
    pub label: String,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub neighborhood: Option<String>,
    pub suburb: Option<String>,
    pub city_district: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    /// The provider's snapped result coordinate, which can differ from the
    /// input photo metadata coordinate.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CoordinateReverseResponse {
    pub input_latitude: f64,
    pub input_longitude: f64,
    pub candidates: Vec<ReverseAddressCandidate>,
    pub source_statuses: Vec<ReverseSourceStatus>,
    pub retrieved_at: u64,
    pub limitations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NominatimReverseResponse {
    display_name: Option<String>,
    lat: Option<String>,
    lon: Option<String>,
    address: Option<NominatimReverseAddress>,
    error: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct NominatimReverseAddress {
    house_number: Option<String>,
    road: Option<String>,
    pedestrian: Option<String>,
    footway: Option<String>,
    neighbourhood: Option<String>,
    quarter: Option<String>,
    suburb: Option<String>,
    city_district: Option<String>,
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    municipality: Option<String>,
    state: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArcgisReverseResponse {
    address: Option<ArcgisReverseAddress>,
    location: Option<ArcgisLocation>,
    error: Option<ArcgisServiceError>,
}

#[derive(Debug, Deserialize)]
struct ArcgisReverseAddress {
    #[serde(rename = "LongLabel")]
    long_label: Option<String>,
    #[serde(rename = "Match_addr")]
    match_address: Option<String>,
    #[serde(rename = "AddNum")]
    house_number: Option<String>,
    #[serde(rename = "Address")]
    road_address: Option<String>,
    #[serde(rename = "Neighborhood")]
    neighborhood: Option<String>,
    #[serde(rename = "District")]
    district: Option<String>,
    #[serde(rename = "City")]
    city: Option<String>,
    #[serde(rename = "Region")]
    region: Option<String>,
    #[serde(rename = "CntryName")]
    country: Option<String>,
    #[serde(rename = "CountryCode")]
    country_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArcgisLocation {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct ArcgisServiceError {
    code: Option<i64>,
    message: Option<String>,
    #[serde(default)]
    details: Vec<String>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn finite_coordinate(value: Option<f64>, range: std::ops::RangeInclusive<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && range.contains(value))
}

fn language_tag(language: Option<&str>) -> String {
    language
        .map(str::trim)
        .filter(|value| (2..=16).contains(&value.len()))
        .filter(|value| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .unwrap_or("en")
        .replace('_', "-")
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
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{source} returned HTTP {status}"));
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

fn nominatim_candidate(
    response: NominatimReverseResponse,
) -> Result<Option<ReverseAddressCandidate>, String> {
    if let Some(error) = clean(response.error) {
        return Err(format!("Nominatim reverse geocoding returned: {error}"));
    }
    let Some(label) = clean(response.display_name) else {
        return Ok(None);
    };
    let address = response.address.unwrap_or_default();
    Ok(Some(ReverseAddressCandidate {
        source: "nominatim".into(),
        label,
        house_number: clean(address.house_number),
        road: clean(address.road)
            .or_else(|| clean(address.pedestrian))
            .or_else(|| clean(address.footway)),
        neighborhood: clean(address.neighbourhood).or_else(|| clean(address.quarter)),
        suburb: clean(address.suburb),
        city_district: clean(address.city_district),
        city: clean(address.city)
            .or_else(|| clean(address.town))
            .or_else(|| clean(address.village))
            .or_else(|| clean(address.municipality)),
        state: clean(address.state),
        country: clean(address.country),
        country_code: clean(address.country_code),
        latitude: finite_coordinate(
            response.lat.and_then(|value| value.parse().ok()),
            -90.0..=90.0,
        ),
        longitude: finite_coordinate(
            response.lon.and_then(|value| value.parse().ok()),
            -180.0..=180.0,
        ),
    }))
}

fn arcgis_error(error: ArcgisServiceError) -> String {
    let code = error
        .code
        .map(|value| format!(" code {value}"))
        .unwrap_or_default();
    let message = clean(error.message).unwrap_or_else(|| "unknown service error".into());
    let details = error
        .details
        .into_iter()
        .filter_map(|value| clean(Some(value)))
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "ArcGIS World Geocoding returned{code}: {message}{}",
        if details.is_empty() {
            String::new()
        } else {
            format!(" ({details})")
        }
    )
}

fn arcgis_candidate(
    response: ArcgisReverseResponse,
) -> Result<Option<ReverseAddressCandidate>, String> {
    if let Some(error) = response.error {
        return Err(arcgis_error(error));
    }
    let Some(address) = response.address else {
        return Ok(None);
    };
    let Some(label) = clean(address.long_label).or_else(|| clean(address.match_address)) else {
        return Ok(None);
    };
    let location = response.location;
    Ok(Some(ReverseAddressCandidate {
        source: "arcgis_world_geocoding".into(),
        label,
        house_number: clean(address.house_number),
        // ArcGIS exposes a formatted street-address field, not a separate road
        // name. Keep the provider field intact instead of parsing it heuristically.
        road: clean(address.road_address),
        neighborhood: clean(address.neighborhood),
        suburb: None,
        city_district: clean(address.district),
        city: clean(address.city),
        state: clean(address.region),
        country: clean(address.country),
        country_code: clean(address.country_code),
        latitude: finite_coordinate(location.as_ref().map(|value| value.y), -90.0..=90.0),
        longitude: finite_coordinate(location.as_ref().map(|value| value.x), -180.0..=180.0),
    }))
}

async fn reverse_nominatim(
    latitude: f64,
    longitude: f64,
    language: &str,
) -> Result<Option<ReverseAddressCandidate>, String> {
    let lat = format!("{latitude:.7}");
    let lon = format!("{longitude:.7}");
    let response: NominatimReverseResponse = response_json(
        "Nominatim",
        HTTP.get(NOMINATIM_REVERSE_URL).query(&[
            ("lat", lat.as_str()),
            ("lon", lon.as_str()),
            ("format", "jsonv2"),
            ("addressdetails", "1"),
            ("zoom", "18"),
            ("accept-language", language),
        ]),
    )
    .await?;
    nominatim_candidate(response)
}

async fn reverse_arcgis(
    latitude: f64,
    longitude: f64,
    language: &str,
) -> Result<Option<ReverseAddressCandidate>, String> {
    let location = format!("{longitude:.7},{latitude:.7}");
    let response: ArcgisReverseResponse = response_json(
        "ArcGIS World Geocoding",
        HTTP.get(ARCGIS_REVERSE_URL).query(&[
            ("location", location.as_str()),
            ("f", "json"),
            ("forStorage", "false"),
            ("featureTypes", "StreetAddress"),
            ("langCode", language),
        ]),
    )
    .await?;
    arcgis_candidate(response)
}

fn source_status(
    source: &str,
    status: ReverseSourceState,
    detail: impl Into<String>,
) -> ReverseSourceStatus {
    ReverseSourceStatus {
        source: source.into(),
        status,
        detail: detail.into(),
    }
}

/// Reverse-geocode a coordinate that the caller has already obtained from an
/// image's embedded metadata. This command never infers coordinates from image
/// pixels, filenames, IP addresses, or timezones.
#[tauri::command]
pub async fn reverse_geocode_coordinates(
    latitude: f64,
    longitude: f64,
    language: Option<String>,
) -> Result<CoordinateReverseResponse, String> {
    if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
        return Err("latitude must be a finite value between -90 and 90".into());
    }
    if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
        return Err("longitude must be a finite value between -180 and 180".into());
    }
    let language = language_tag(language.as_deref());
    let (nominatim, arcgis) = tokio::join!(
        timed(
            "Nominatim",
            reverse_nominatim(latitude, longitude, &language)
        ),
        timed(
            "ArcGIS World Geocoding",
            reverse_arcgis(latitude, longitude, &language)
        )
    );

    let mut candidates = Vec::with_capacity(2);
    let mut source_statuses = Vec::with_capacity(2);
    for (source, result) in [("nominatim", nominatim), ("arcgis_world_geocoding", arcgis)] {
        match result {
            Ok(Some(candidate)) => {
                candidates.push(candidate);
                source_statuses.push(source_status(
                    source,
                    ReverseSourceState::Success,
                    "A structured reverse-geocoding candidate was returned.",
                ));
            }
            Ok(None) => source_statuses.push(source_status(
                source,
                ReverseSourceState::Empty,
                "No structured reverse-geocoding candidate was returned.",
            )),
            Err(error) => {
                source_statuses.push(source_status(source, ReverseSourceState::Failed, error))
            }
        }
    }

    Ok(CoordinateReverseResponse {
        input_latitude: latitude,
        input_longitude: longitude,
        candidates,
        source_statuses,
        retrieved_at: unix_now(),
        limitations: vec![
            "The input coordinate was supplied by the caller; this command did not infer it from pixels, filename, IP address, or timezone.".into(),
            "Reverse-geocoded labels are nearby public map records, not proof of the camera's exact address or an on-site verification.".into(),
            "Provider-snapped result coordinates can differ from the input coordinate. Conflicting house numbers or labels must be reported, not silently merged.".into(),
            "ArcGIS is queried with forStorage=false. Public geocoding endpoints can be incomplete, stale, rate-limited, or temporarily unavailable.".into(),
            "retrieved_at is the request completion time, not the map record's update time or the photo capture time.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominatim_fixture_keeps_structured_neighborhood_fields() {
        let fixture = r#"{
          "display_name": "283, 胶州路, 康家桥, 曹家渡街道, 静安区, 上海市, 中国",
          "lat": "31.2300400",
          "lon": "121.4387500",
          "address": {
            "house_number": "283",
            "road": "胶州路",
            "neighbourhood": "康家桥",
            "suburb": "曹家渡街道",
            "city": "静安区",
            "state": "上海市",
            "country": "中国",
            "country_code": "cn"
          }
        }"#;
        let response: NominatimReverseResponse = serde_json::from_str(fixture).unwrap();
        let candidate = nominatim_candidate(response).unwrap().unwrap();
        assert_eq!(candidate.house_number.as_deref(), Some("283"));
        assert_eq!(candidate.road.as_deref(), Some("胶州路"));
        assert_eq!(candidate.neighborhood.as_deref(), Some("康家桥"));
        assert_eq!(candidate.suburb.as_deref(), Some("曹家渡街道"));
        assert_eq!(candidate.city.as_deref(), Some("静安区"));
    }

    #[test]
    fn arcgis_fixture_keeps_provider_label_without_claiming_accuracy() {
        let fixture = r#"{
          "address": {
            "LongLabel": "282 Jiao Zhou Rd, 曹家渡街道, Jing'an District, Shanghai City, CHN",
            "Match_addr": "282 Jiao Zhou Rd",
            "AddNum": "282",
            "Address": "282 Jiao Zhou Rd",
            "Neighborhood": "曹家渡街道",
            "District": "Jing'an District",
            "City": "Shanghai City",
            "Region": "Shanghai City",
            "CntryName": "China",
            "CountryCode": "CHN"
          },
          "location": {"x": 121.43875479716, "y": 31.230038216008}
        }"#;
        let response: ArcgisReverseResponse = serde_json::from_str(fixture).unwrap();
        let candidate = arcgis_candidate(response).unwrap().unwrap();
        assert_eq!(candidate.house_number.as_deref(), Some("282"));
        assert_eq!(candidate.neighborhood.as_deref(), Some("曹家渡街道"));
        assert_eq!(candidate.source, "arcgis_world_geocoding");
    }

    #[test]
    fn language_and_coordinates_are_strictly_bounded() {
        assert_eq!(language_tag(Some("zh_CN")), "zh-CN");
        assert_eq!(language_tag(Some("../../etc/passwd")), "en");
        assert_eq!(language_tag(None), "en");
        assert!(finite_coordinate(Some(f64::NAN), -90.0..=90.0).is_none());
        assert!(finite_coordinate(Some(91.0), -90.0..=90.0).is_none());
        assert_eq!(finite_coordinate(Some(-33.9), -90.0..=90.0), Some(-33.9));
    }

    #[tokio::test]
    async fn invalid_coordinates_fail_before_network() {
        assert!(reverse_geocode_coordinates(91.0, 0.0, None).await.is_err());
        assert!(reverse_geocode_coordinates(0.0, 181.0, None).await.is_err());
        assert!(reverse_geocode_coordinates(f64::NAN, 0.0, None)
            .await
            .is_err());
    }

    #[tokio::test]
    #[ignore = "calls live public Nominatim and ArcGIS reverse-geocoding endpoints"]
    async fn live_reverse_geocoding_returns_real_public_candidates() {
        let response = reverse_geocode_coordinates(31.2300382, 121.4387548, Some("zh".into()))
            .await
            .unwrap();
        assert!(!response.candidates.is_empty());
        assert!(response
            .source_statuses
            .iter()
            .any(|status| status.status == ReverseSourceState::Success));
        eprintln!("{}", serde_json::to_string_pretty(&response).unwrap());
    }
}
