use crate::error::{AppError, AppResult};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct WeatherData {
    pub location: String,
    pub temp: f64,
    pub feels_like: f64,
    pub humidity: u32,
    pub description: String,
    pub icon: String,
    pub wind_speed: f64,
    pub forecast: Vec<ForecastDay>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastDay {
    pub date: String,
    pub temp_min: f64,
    pub temp_max: f64,
    pub description: String,
    pub icon: String,
}

struct CachedWeather {
    data: WeatherData,
    fetched_at: std::time::Instant,
}

pub struct WeatherService {
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, CachedWeather>>>,
}

impl WeatherService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_weather(
        &self,
        location: &str,
        units: &str,
        locale: &str,
        timezone: &str,
    ) -> AppResult<WeatherData> {
        let cache_key = format!("{location}:{units}:{locale}:{timezone}");

        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.fetched_at.elapsed().as_secs() < 600 {
                    return Ok(cached.data.clone());
                }
            }
        }

        let data = self
            .fetch_weather(location, units, locale, timezone)
            .await?;

        self.cache.write().await.insert(
            cache_key,
            CachedWeather {
                data: data.clone(),
                fetched_at: std::time::Instant::now(),
            },
        );

        Ok(data)
    }

    async fn fetch_weather(
        &self,
        location: &str,
        units: &str,
        locale: &str,
        timezone: &str,
    ) -> AppResult<WeatherData> {
        let (name, country) = parse_location(location);
        let language = open_meteo_language(locale);
        let geo = self.geocode(&name, country.as_deref(), language).await?;

        let (temp_unit, wind_unit) = units_to_open_meteo(units);
        let tz = urlencoding::encode(timezone);
        let forecast_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
             &current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m\
             &daily=weather_code,temperature_2m_max,temperature_2m_min\
             &temperature_unit={temp_unit}&wind_speed_unit={wind_unit}\
             &timezone={tz}&forecast_days=5",
            geo.latitude, geo.longitude
        );

        let forecast: ForecastResponse = self
            .client
            .get(&forecast_url)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .json()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let current = forecast.current.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("No current weather data").into())
        })?;

        let daily = forecast.daily.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("No daily forecast data").into())
        })?;

        let display_name = geo
            .admin1
            .as_ref()
            .map(|admin| format!("{}, {}", geo.name, admin))
            .unwrap_or_else(|| geo.name.clone());

        let (description, icon) = weather_code_info(current.weather_code, locale);

        let forecast_days = daily
            .time
            .iter()
            .zip(daily.weather_code.iter())
            .zip(daily.temperature_2m_max.iter())
            .zip(daily.temperature_2m_min.iter())
            .map(|(((date, code), temp_max), temp_min)| {
                let (desc, ic) = weather_code_info(*code, locale);
                ForecastDay {
                    date: date.clone(),
                    temp_min: *temp_min,
                    temp_max: *temp_max,
                    description: desc,
                    icon: ic,
                }
            })
            .collect();

        Ok(WeatherData {
            location: display_name,
            temp: current.temperature_2m,
            feels_like: current.apparent_temperature,
            humidity: current.relative_humidity_2m,
            description,
            icon,
            wind_speed: current.wind_speed_10m,
            forecast: forecast_days,
        })
    }

    async fn geocode(
        &self,
        name: &str,
        country: Option<&str>,
        language: &str,
    ) -> AppResult<GeoResult> {
        let mut query: Vec<(&str, &str)> = vec![
            ("name", name),
            ("count", "5"),
            ("language", language),
            ("format", "json"),
        ];
        let country_owned;
        if let Some(code) = country {
            country_owned = code.to_string();
            query.push(("countryCode", &country_owned));
        }

        let response: GeoResponse = self
            .client
            .get("https://geocoding-api.open-meteo.com/v1/search")
            .query(&query)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .json()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        response
            .results
            .and_then(|mut results| {
                if let Some(code) = country {
                    results.retain(|r| {
                        r.country_code
                            .as_deref()
                            .is_some_and(|c| c.eq_ignore_ascii_case(code))
                    });
                }
                results.into_iter().next()
            })
            .ok_or_else(|| AppError::NotFound(format!("Location not found: {name}")))
    }
}

fn parse_location(location: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = location.splitn(2, ',').map(str::trim).collect();
    match parts.as_slice() {
        [name, country] if !country.is_empty() => (name.to_string(), Some(country.to_string())),
        [name] => (name.to_string(), None),
        _ => (location.trim().to_string(), None),
    }
}

fn units_to_open_meteo(units: &str) -> (&'static str, &'static str) {
    match units {
        "imperial" => ("fahrenheit", "mph"),
        _ => ("celsius", "ms"),
    }
}

fn open_meteo_language(locale: &str) -> &str {
    match locale.split('-').next().unwrap_or("en") {
        "fr" => "fr",
        "de" => "de",
        "es" => "es",
        "it" => "it",
        "pt" => "pt",
        "nl" => "nl",
        _ => "en",
    }
}

fn weather_code_info(code: u32, locale: &str) -> (String, String) {
    let fr = locale.starts_with("fr");
    match code {
        0 => {
            if fr {
                ("Ciel dégagé".into(), "clear".into())
            } else {
                ("Clear sky".into(), "clear".into())
            }
        }
        1 => {
            if fr {
                ("Peu nuageux".into(), "partly-cloudy".into())
            } else {
                ("Mainly clear".into(), "partly-cloudy".into())
            }
        }
        2 => {
            if fr {
                ("Partiellement nuageux".into(), "partly-cloudy".into())
            } else {
                ("Partly cloudy".into(), "partly-cloudy".into())
            }
        }
        3 => {
            if fr {
                ("Couvert".into(), "cloudy".into())
            } else {
                ("Overcast".into(), "cloudy".into())
            }
        }
        45 | 48 => {
            if fr {
                ("Brouillard".into(), "fog".into())
            } else {
                ("Fog".into(), "fog".into())
            }
        }
        51 | 53 | 55 => {
            if fr {
                ("Bruine".into(), "drizzle".into())
            } else {
                ("Drizzle".into(), "drizzle".into())
            }
        }
        56 | 57 => {
            if fr {
                ("Bruine verglaçante".into(), "drizzle".into())
            } else {
                ("Freezing drizzle".into(), "drizzle".into())
            }
        }
        61 | 63 | 65 => {
            if fr {
                ("Pluie".into(), "rain".into())
            } else {
                ("Rain".into(), "rain".into())
            }
        }
        66 | 67 => {
            if fr {
                ("Pluie verglaçante".into(), "rain".into())
            } else {
                ("Freezing rain".into(), "rain".into())
            }
        }
        71 | 73 | 75 | 77 => {
            if fr {
                ("Neige".into(), "snow".into())
            } else {
                ("Snow".into(), "snow".into())
            }
        }
        80 | 81 | 82 => {
            if fr {
                ("Averses".into(), "showers".into())
            } else {
                ("Showers".into(), "showers".into())
            }
        }
        85 | 86 => {
            if fr {
                ("Averses de neige".into(), "snow".into())
            } else {
                ("Snow showers".into(), "snow".into())
            }
        }
        95 | 96 | 99 => {
            if fr {
                ("Orage".into(), "thunderstorm".into())
            } else {
                ("Thunderstorm".into(), "thunderstorm".into())
            }
        }
        _ => {
            if fr {
                ("Inconnu".into(), "cloudy".into())
            } else {
                ("Unknown".into(), "cloudy".into())
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct GeoResponse {
    results: Option<Vec<GeoResult>>,
}

#[derive(Debug, Deserialize)]
struct GeoResult {
    name: String,
    latitude: f64,
    longitude: f64,
    country_code: Option<String>,
    admin1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    current: Option<CurrentWeather>,
    daily: Option<DailyForecast>,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    relative_humidity_2m: u32,
    apparent_temperature: f64,
    weather_code: u32,
    wind_speed_10m: f64,
}

#[derive(Debug, Deserialize)]
struct DailyForecast {
    time: Vec<String>,
    weather_code: Vec<u32>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_location_with_country() {
        let (name, country) = parse_location("Paris,FR");
        assert_eq!(name, "Paris");
        assert_eq!(country.as_deref(), Some("FR"));
    }

    #[test]
    fn parse_location_name_only() {
        let (name, country) = parse_location("Paris");
        assert_eq!(name, "Paris");
        assert_eq!(country, None);
    }
}
