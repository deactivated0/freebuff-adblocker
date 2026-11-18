use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: JsonValue) {
        if let JsonValue::Object(entries) = self {
            for (k, v) in entries.iter_mut() {
                if k == key {
                    *v = value;
                    return;
                }
            }
            entries.push((key.to_string(), value));
        }
    }

    pub fn serialize_pretty(&self, indent_level: usize) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            JsonValue::String(s) => escape_string(s),
            JsonValue::Array(items) => {
                if items.is_empty() {
                    return "[]".to_string();
                }
                let mut out = String::from("[\n");
                let indent = "  ".repeat(indent_level + 1);
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&indent);
                    out.push_str(&item.serialize_pretty(indent_level + 1));
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent_level));
                out.push(']');
                out
            }
            JsonValue::Object(entries) => {
                if entries.is_empty() {
                    return "{}".to_string();
                }
                let mut out = String::from("{\n");
                let indent = "  ".repeat(indent_level + 1);
                for (i, (k, v)) in entries.iter().enumerate() {
                    out.push_str(&indent);
                    out.push_str(&escape_string(k));
                    out.push_str(": ");
                    out.push_str(&v.serialize_pretty(indent_level + 1));
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent_level));
                out.push('}');
                out
            }
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub struct JsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    pub fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') | Some('f') => self.parse_bool().map(JsonValue::Bool),
            Some('n') => self.parse_null().map(|_| JsonValue::Null),
            Some(&c) if c == '-' || c.is_ascii_digit() => self.parse_number().map(JsonValue::Number),
            Some(&c) => Err(format!("Unexpected character: {}", c)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.chars.next(); // consume '{'
        let mut entries = Vec::new();
        loop {
            self.skip_whitespace();
            if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.chars.next() != Some(':') {
                return Err("Expected ':' after key in object".to_string());
            }
            let value = self.parse()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some('}') => {
                    self.chars.next();
                    break;
                }
                Some(&c) => return Err(format!("Expected ',' or '}}' in object, got '{}'", c)),
                None => return Err("Unterminated object".to_string()),
            }
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.chars.next(); // consume '['
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            if let Some(&']') = self.chars.peek() {
                self.chars.next();
                break;
            }
            let item = self.parse()?;
            items.push(item);
            self.skip_whitespace();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some(']') => {
                    self.chars.next();
                    break;
                }
                Some(&c) => return Err(format!("Expected ',' or ']' in array, got '{}'", c)),
                None => return Err("Unterminated array".to_string()),
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.chars.next() != Some('"') {
            return Err("Expected '\"' for string".to_string());
        }
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            match c {
                '"' => return Ok(s),
                '\\' => match self.chars.next() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('b') => s.push('\u{08}'),
                    Some('f') => s.push('\u{0c}'),
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('u') => {
                        let mut hex = String::with_capacity(4);
                        for _ in 0..4 {
                            if let Some(h) = self.chars.next() {
                                hex.push(h);
                            } else {
                                return Err("Incomplete unicode escape in string".to_string());
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                s.push(ch);
                            } else {
                                return Err(format!("Invalid unicode codepoint: {:x}", code));
                            }
                        } else {
                            return Err(format!("Invalid unicode hex: {}", hex));
                        }
                    }
                    Some(c) => return Err(format!("Invalid escape sequence: \\{}", c)),
                    None => return Err("Unterminated escape sequence".to_string()),
                },
                c => s.push(c),
            }
        }
        Err("Unterminated string".to_string())
    }

    fn parse_bool(&mut self) -> Result<bool, String> {
        let mut token = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                token.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        match token.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("Unknown token: {}", token)),
        }
    }

    fn parse_null(&mut self) -> Result<(), String> {
        let mut token = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                token.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        if token == "null" {
            Ok(())
        } else {
            Err(format!("Expected null, got: {}", token))
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        s.parse::<f64>()
            .map_err(|e| format!("Invalid number '{}': {}", s, e))
    }
}

pub fn get_default_settings_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/manicode/settings.json")
    } else {
        PathBuf::from(".config/manicode/settings.json")
    }
}

pub fn get_current_local_date() -> String {
    if let Ok(output) = Command::new("date").arg("+%Y-%m-%d").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
                return s;
            }
        }
    }
    // Fallback: estimate from SystemTime
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Rough Gregorian date from epoch seconds
    let days = (secs / 86400) as i64;
    let (year, month, day) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[derive(Debug)]
pub struct SettingsStatus {
    pub ads_enabled: Option<bool>,
    pub display_ads: Option<Vec<String>>,
    pub display_ad_splash_hidden_on: Option<String>,
    pub is_fully_configured: bool,
}

pub fn inspect_settings(path: &Path) -> io::Result<SettingsStatus> {
    if !path.exists() {
        return Ok(SettingsStatus {
            ads_enabled: None,
            display_ads: None,
            display_ad_splash_hidden_on: None,
            is_fully_configured: false,
        });
    }

    let content = fs::read_to_string(path)?;
    let mut parser = JsonParser::new(&content);
    let val = parser
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let ads_enabled = match val.get("adsEnabled") {
        Some(JsonValue::Bool(b)) => Some(*b),
        _ => None,
    };

    let display_ads = match val.get("displayAds") {
        Some(JsonValue::Array(arr)) => {
            let mut list = Vec::new();
            for item in arr {
                if let JsonValue::String(s) = item {
                    list.push(s.clone());
                }
            }
            Some(list)
        }
        _ => None,
    };

    let display_ad_splash_hidden_on = match val.get("displayAdSplashHiddenOn") {
        Some(JsonValue::String(s)) => Some(s.clone()),
        _ => None,
    };

    let is_fully_configured = ads_enabled == Some(false)
        && display_ads.as_ref().map(|v| v.is_empty()).unwrap_or(false)
        && display_ad_splash_hidden_on.is_some();

    Ok(SettingsStatus {
        ads_enabled,
        display_ads,
        display_ad_splash_hidden_on,
        is_fully_configured,
    })
}

pub fn update_settings_for_ad_removal(path: &Path) -> io::Result<String> {
    let mut val = if path.exists() {
        let content = fs::read_to_string(path)?;
        let mut parser = JsonParser::new(&content);
        parser
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    } else {
        JsonValue::Object(Vec::new())
    };

    let date = get_current_local_date();

    val.set("adsEnabled", JsonValue::Bool(false));
    val.set("displayAds", JsonValue::Array(Vec::new()));
    val.set("displayAdSplashHiddenOn", JsonValue::String(date.clone()));

    let serialized = val.serialize_pretty(0);

    // Atomic write to avoid partial writes
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp_path, serialized.as_bytes())?;
    fs::rename(&tmp_path, path)?;

    Ok(date)
}
