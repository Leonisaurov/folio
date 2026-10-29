//! Libros diarios: día lógico, patrones y políticas.

use std::fmt::Write as _;

use folio_core::FolioError;
use jiff::civil::{Date, date};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_create")]
    pub create: String,
    #[serde(default = "default_title_format")]
    pub title_format: String,
    #[serde(default = "default_folder_format")]
    pub folder_format: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_start_hour")]
    pub day_start_hour: u8,
    #[serde(default = "default_backfill")]
    pub backfill: String,
    #[serde(default)]
    pub recreate_deleted: bool,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub auto_archive_days: u32,
}

fn default_enabled() -> bool {
    true
}
fn default_create() -> String {
    "first_open".into()
}
fn default_title_format() -> String {
    "%d/%m/%Y".into()
}
fn default_folder_format() -> String {
    "%Y-%m-%d".into()
}
fn default_timezone() -> String {
    "local".into()
}
fn default_start_hour() -> u8 {
    4
}
fn default_backfill() -> String {
    "manual".into()
}

impl Default for DailyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            create: default_create(),
            title_format: default_title_format(),
            folder_format: default_folder_format(),
            timezone: default_timezone(),
            day_start_hour: default_start_hour(),
            backfill: default_backfill(),
            recreate_deleted: false,
            template: String::new(),
            auto_archive_days: 0,
        }
    }
}

impl DailyConfig {
    pub fn validate(&self) -> Result<(), FolioError> {
        if self.day_start_hour > 23 {
            return Err(FolioError::Invalid(
                "day_start_hour debe estar entre 0 y 23".into(),
            ));
        }
        if !["first_open", "first_note", "manual"].contains(&self.create.as_str()) {
            return Err(FolioError::Invalid(format!(
                "política daily.create inválida: {}",
                self.create
            )));
        }
        if !["manual", "ask", "auto", "never"].contains(&self.backfill.as_str()) {
            return Err(FolioError::Invalid(format!(
                "política daily.backfill inválida: {}",
                self.backfill
            )));
        }
        if render_pattern(&self.title_format, date(2026, 10, 29))
            .trim()
            .is_empty()
        {
            return Err(FolioError::Invalid(
                "title_format produce un título vacío".into(),
            ));
        }
        let folder = safe_folder(&render_pattern(&self.folder_format, date(2026, 10, 29)));
        if folder.is_empty() {
            return Err(FolioError::Invalid(
                "folder_format produce una carpeta vacía".into(),
            ));
        }
        validate_timezone(&self.timezone)?;
        Ok(())
    }

    pub fn time_zone(&self) -> Result<TimeZone, FolioError> {
        resolve_timezone(&self.timezone)
    }

    pub fn title_for(&self, day: &LogicalDay) -> String {
        render_pattern(&self.title_format, day.date)
    }

    pub fn folder_for(&self, day: &LogicalDay) -> String {
        safe_folder(&render_pattern(&self.folder_format, day.date))
    }
}

fn resolve_timezone(name: &str) -> Result<TimeZone, FolioError> {
    if name.is_empty() || name == "local" {
        Ok(TimeZone::system())
    } else {
        TimeZone::get(name)
            .map_err(|error| FolioError::Invalid(format!("zona horaria '{name}': {error}")))
    }
}

pub fn validate_timezone(name: &str) -> Result<(), FolioError> {
    resolve_timezone(name).map(|_| ())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalDay {
    pub date: Date,
    pub timezone: String,
    pub start: Timestamp,
    /// Inicio del día lógico siguiente (exclusivo).
    pub end: Timestamp,
}

impl LogicalDay {
    pub fn date_string(&self) -> String {
        self.date.to_string()
    }

    pub fn title(&self, config: &DailyConfig) -> String {
        config.title_for(self)
    }

    pub fn folder(&self, config: &DailyConfig) -> String {
        config.folder_for(self)
    }

    pub fn contains(&self, timestamp: Timestamp) -> bool {
        self.start <= timestamp && timestamp < self.end
    }
}

/// Calcula el día lógico para un instante y una hora de corte.
///
/// Con `day_start_hour = 4`, las 03:59 pertenecen al día anterior.
pub fn logical_day(
    now: Timestamp,
    timezone: &str,
    day_start_hour: u8,
) -> Result<LogicalDay, FolioError> {
    if day_start_hour > 23 {
        return Err(FolioError::Invalid("day_start_hour fuera de rango".into()));
    }
    let tz = resolve_timezone(timezone)?;
    let zoned: Zoned = now.to_zoned(tz.clone());
    let local = zoned.datetime();
    let mut date = local.date();
    if local.hour() < day_start_hour as i8 {
        date = date
            .yesterday()
            .map_err(|error| FolioError::Invalid(format!("no se pudo restar un día: {error}")))?;
    }
    let start = start_of_logical_day(date, day_start_hour, &tz)?;
    let next_date = date
        .tomorrow()
        .map_err(|error| FolioError::Invalid(format!("no se pudo sumar un día: {error}")))?;
    let end = start_of_logical_day(next_date, day_start_hour, &tz)?;
    Ok(LogicalDay {
        date,
        timezone: timezone.to_string(),
        start,
        end,
    })
}

fn start_of_logical_day(
    date: Date,
    day_start_hour: u8,
    tz: &TimeZone,
) -> Result<Timestamp, FolioError> {
    let civil = date.at(day_start_hour as i8, 0, 0, 0);
    let zoned = civil.to_zoned(tz.clone()).map_err(|error| {
        FolioError::Invalid(format!("hora de corte inválida en {date}: {error}"))
    })?;
    Ok(zoned.timestamp())
}

/// Rango de días lógicos `[start, end]` inclusive, sin generar huecos.
pub fn backfill_dates(start: Date, end: Date) -> Result<Vec<Date>, FolioError> {
    if end < start {
        return Err(FolioError::Invalid(
            "el rango de backfill termina antes de empezar".into(),
        ));
    }
    let mut dates = Vec::new();
    let mut current = start;
    loop {
        dates.push(current);
        if current == end {
            break;
        }
        if dates.len() > 100_000 {
            return Err(FolioError::Invalid(
                "rango de backfill demasiado grande".into(),
            ));
        }
        current = current
            .tomorrow()
            .map_err(|error| FolioError::Invalid(error.to_string()))?;
    }
    Ok(dates)
}

/// Política: ¿se crea el libro al abrir la app?
pub fn should_create_on_open(config: &DailyConfig) -> bool {
    config.enabled && config.create == "first_open"
}

pub fn should_create_on_first_note(config: &DailyConfig) -> bool {
    config.enabled && config.create == "first_note"
}

const MONTHS_ES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];
const WEEKDAYS_ES: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];

/// Implementación de los códigos `strftime` que usa la configuración.
pub fn render_pattern(pattern: &str, date: Date) -> String {
    let mut out = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let Some(code) = chars.next() else {
            out.push('%');
            break;
        };
        match code {
            '%' => out.push('%'),
            'Y' => {
                let _ = write!(out, "{:04}", date.year());
            }
            'y' => {
                let _ = write!(out, "{:02}", date.year().rem_euclid(100));
            }
            'm' => {
                let _ = write!(out, "{:02}", date.month());
            }
            'd' => {
                let _ = write!(out, "{:02}", date.day());
            }
            'e' => {
                let _ = write!(out, "{:2}", date.day());
            }
            'B' => out.push_str(MONTHS_ES[(date.month() as usize).saturating_sub(1)]),
            'b' => out.push_str(&MONTHS_ES[(date.month() as usize).saturating_sub(1)][..3]),
            'A' => out.push_str(WEEKDAYS_ES[weekday_index(date)]),
            'a' => out.push_str(&WEEKDAYS_ES[weekday_index(date)][..3]),
            'V' => {
                let _ = write!(out, "{:02}", date.iso_week_date().week());
            }
            'j' => {
                let _ = write!(out, "{:03}", date.day_of_year());
            }
            'F' => {
                let _ = write!(
                    out,
                    "{:04}-{:02}-{:02}",
                    date.year(),
                    date.month(),
                    date.day()
                );
            }
            'T' => out.push_str("00:00:00"),
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

fn weekday_index(date: Date) -> usize {
    // jiff: lunes = 1 … domingo = 7.
    (date.weekday().to_monday_one_offset() as usize).saturating_sub(1)
}

/// Sanea una carpeta: sin separadores, sin `.` inicial y sin caracteres raros.
pub fn safe_folder(input: &str) -> String {
    let mut out = String::new();
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let out = out.trim_matches(['.', '-', '_']).to_string();
    if out.is_empty() {
        "sin-fecha".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn rollover_at_four_am() {
        let before = logical_day(ts("2026-10-29T03:59:00Z"), "UTC", 4).unwrap();
        let after = logical_day(ts("2026-10-29T04:00:00Z"), "UTC", 4).unwrap();
        assert_eq!(before.date_string(), "2026-10-28");
        assert_eq!(after.date_string(), "2026-10-29");
        assert!(before.contains(ts("2026-10-29T03:59:59Z")));
        assert!(!before.contains(ts("2026-10-29T04:00:00Z")));
    }

    #[test]
    fn timezone_changes_do_not_duplicate_dates() {
        let instant = ts("2026-10-29T02:30:00Z");
        let madrid = logical_day(instant, "Europe/Madrid", 4).unwrap();
        let new_york = logical_day(instant, "America/New_York", 4).unwrap();
        assert_eq!(madrid.date_string(), "2026-10-28");
        assert_eq!(new_york.date_string(), "2026-10-28");
        let tokyo = logical_day(instant, "Asia/Tokyo", 4).unwrap();
        assert_eq!(tokyo.date_string(), "2026-10-29");
    }

    #[test]
    fn dst_days_have_23_and_25_hours() {
        let spring = logical_day(ts("2026-03-29T12:00:00Z"), "Europe/Madrid", 2).unwrap();
        let hours = (spring.end.as_second() - spring.start.as_second()) / 3600;
        assert_eq!(hours, 23);
        let autumn = logical_day(ts("2026-10-25T12:00:00Z"), "Europe/Madrid", 2).unwrap();
        let hours = (autumn.end.as_second() - autumn.start.as_second()) / 3600;
        assert_eq!(hours, 25);
    }

    #[test]
    fn formats_in_spanish() {
        let date = date(2026, 10, 29);
        assert_eq!(render_pattern("%d/%m/%Y", date), "29/10/2026");
        assert_eq!(render_pattern("%Y/%m/%d", date), "2026/10/29");
        assert_eq!(
            render_pattern("%A, %d de %B de %Y", date),
            "jueves, 29 de octubre de 2026"
        );
        assert_eq!(
            render_pattern("%Y-%m-%d · Semana %V", date),
            "2026-10-29 · Semana 44"
        );
    }

    #[test]
    fn folder_is_safe() {
        assert_eq!(safe_folder("2026/10/29"), "20261029");
        assert_eq!(safe_folder("../../etc"), "etc");
        assert_eq!(safe_folder("   "), "sin-fecha");
    }

    #[test]
    fn backfill_is_inclusive_and_idempotent() {
        let dates = backfill_dates(date(2026, 10, 20), date(2026, 10, 29)).unwrap();
        assert_eq!(dates.len(), 10);
        assert_eq!(dates.first().unwrap().to_string(), "2026-10-20");
        assert_eq!(dates.last().unwrap().to_string(), "2026-10-29");
        assert!(backfill_dates(date(2026, 10, 29), date(2026, 10, 20)).is_err());
    }

    #[test]
    fn policies() {
        let mut config = DailyConfig::default();
        assert!(should_create_on_open(&config));
        assert!(!should_create_on_first_note(&config));
        config.create = "first_note".into();
        assert!(should_create_on_first_note(&config));
        config.create = "manual".into();
        assert!(!should_create_on_open(&config));
    }

    #[test]
    fn invalid_timezone_is_reported() {
        assert!(validate_timezone("Marte/Olympus").is_err());
        assert!(validate_timezone("Europe/Madrid").is_ok());
        assert!(validate_timezone("local").is_ok());
    }
}
