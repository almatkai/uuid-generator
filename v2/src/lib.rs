//! Offline engine. Entropy and time are injectable; the production source uses OS entropy.
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};

pub const COMMANDS: &[&str] = &[
    "generate",
    "generateV5",
    "generateV7",
    "generateUlid",
    "generateTypeID",
    "parseTypeID",
    "formatUuid",
    "packTypeID",
    "viewHistory",
];
pub const MAX_COUNT: usize = 10_000;
pub const MAX_TEXT: usize = 1_048_576;
pub const MAX_HISTORY: usize = 100_000;
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub title: String,
    pub message: String,
}
impl Failure {
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.title, self.message)
    }
}
impl std::error::Error for Failure {}
pub type Result<T> = std::result::Result<T, Failure>;
fn resource(message: &str) -> Failure {
    Failure::new("Resource limit", message)
}

pub trait Source {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<()>;
    fn now_ms(&mut self) -> Result<u64>;
}
pub struct OsSource;
impl Source for OsSource {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<()> {
        getrandom::getrandom(bytes).map_err(|_| {
            Failure::new(
                "Entropy unavailable",
                "The operating system could not provide secure randomness",
            )
        })
    }
    fn now_ms(&mut self) -> Result<u64> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::new("Clock error", "Clock is before Unix epoch"))?
            .as_millis();
        u64::try_from(ms).map_err(|_| resource("Clock is out of range"))
    }
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Engine {
    v7: Option<(u64, u32)>,
    type_v7: Option<(u64, u32, u32)>,
    ulid: Option<(u64, u128)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub uuid: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, flatten)]
    pub extra: std::collections::BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Output {
    pub text: String,
    pub entries: Vec<HistoryEntry>,
    pub message: String,
    pub action: String,
}

pub fn timestamp(ms: u64) -> Result<String> {
    chrono::DateTime::from_timestamp_millis(
        i64::try_from(ms).map_err(|_| resource("Timestamp out of range"))?,
    )
    .map(|d| {
        if (0..=9999).contains(&d.year()) {
            d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        } else {
            format!("{:+07}-{}", d.year(), d.format("%m-%dT%H:%M:%S%.3fZ"))
        }
    })
    .ok_or_else(|| resource("Timestamp out of range"))
}
fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}
fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Array(a) => a
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    js_string(v)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            if f == 0.0 {
                "0".into()
            } else if f.abs() >= 1e21 || f.abs() < 1e-6 {
                {
                    let scientific = format!("{f:e}");
                    if let Some((a, b)) = scientific.split_once('e') {
                        format!("{a}e{}{}", if b.starts_with('-') { "" } else { "+" }, b)
                    } else {
                        scientific
                    }
                }
            } else if f.fract() == 0.0 {
                format!("{f:.0}")
            } else {
                f.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
    }
}
pub fn count(value: &Value, ulid: bool) -> Result<usize> {
    let text = if js_truthy(value) {
        js_string(value)
    } else {
        "1".into()
    };
    if text.len() > MAX_TEXT {
        return Err(resource("Count input exceeds 1 MiB"));
    }
    let s = text.trim_start_matches(|c: char| matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'));
    let (negative, s) = if let Some(t) = s.strip_prefix('-') {
        (true, t)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(Failure::new(
            "Invalid number.",
            "An invalid number has been provided. Try an actual number.",
        ));
    }
    let mut n = digits.parse::<f64>().unwrap_or(f64::INFINITY);
    if negative {
        n = -n;
    }
    if n > MAX_COUNT as f64 {
        let noun = if ulid { "ULIDs" } else { "UUIDs" };
        return Err(Failure::new(
            &format!("Too many {noun} requested."),
            &format!(
                "{} exceeds maximum {noun} of 10000. Try a lower number.",
                if n.is_finite() {
                    js_string(&serde_json::json!(n))
                } else {
                    "Infinity".into()
                }
            ),
        ));
    }
    if n < 0.0 {
        return Err(Failure::new("Generation failed", "Invalid array length"));
    }
    Ok(n as usize)
}
pub fn parse_uuid(s: &str) -> Result<[u8; 16]> {
    let fail = || Failure::new("Invalid UUID", "Invalid UUID");
    let b = s.as_bytes();
    if b.len() != 36 || [8, 13, 18, 23].iter().any(|i| b[*i] != b'-') {
        return Err(fail());
    }
    let h: String = s.chars().filter(|c| *c != '-').collect();
    if h.len() != 32 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(fail());
    }
    let mut out = [0; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).map_err(|_| fail())?;
    }
    if out != [0; 16] && out != [255; 16] && (!(1..=8).contains(&(out[6] >> 4)) || out[8] >> 6 != 2)
    {
        return Err(fail());
    }
    Ok(out)
}
pub fn uuid_string(b: [u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(36);
    for (i, c) in b.iter().enumerate() {
        if [4, 6, 8, 10].contains(&i) {
            out.push('-');
        }
        out.push(HEX[usize::from(c >> 4)] as char);
        out.push(HEX[usize::from(c & 15)] as char);
    }
    out
}
pub fn encode32(b: [u8; 16]) -> String {
    let mut n = u128::from_be_bytes(b);
    let mut s = [b'0'; 26];
    for c in s.iter_mut().rev() {
        *c = ALPHABET[(n & 31) as usize];
        n >>= 5;
    }
    String::from_utf8(s.to_vec()).expect("ASCII alphabet")
}
fn prefix_valid(s: &str) -> bool {
    s.len() <= 63
        && (s.is_empty()
            || (s.as_bytes()[0].is_ascii_lowercase()
                && s.as_bytes()[s.len() - 1].is_ascii_lowercase()))
        && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}
pub fn check_prefix(s: &str) -> Result<()> {
    if prefix_valid(s) {
        Ok(())
    } else {
        Err(Failure::new(
            "Invalid prefix.",
            &format!("Invalid prefix \"{s}\". Must be at most 63 ASCII letters [a-z_]"),
        ))
    }
}
pub fn pack(prefix: &str, bytes: [u8; 16]) -> Result<String> {
    check_prefix(prefix)?;
    let s = encode32(bytes);
    Ok(if prefix.is_empty() {
        s
    } else {
        format!("{prefix}_{s}")
    })
}
pub fn parse_typeid(s: &str) -> Result<String> {
    let (p,suffix)=match s.rsplit_once('_'){Some(("",_))=>return Err(Failure::new("Decoding Error",&format!("EmptyPrefixError: Invalid TypeId. Prefix cannot be empty when there's a separator: {s}"))),Some(t)=>t,None=>("",s)};
    if suffix.is_empty() {
        return Err(Failure::new(
            "Decoding Error",
            "InvalidSuffixLengthError: Invalid length. Suffix should have 26 characters, got 0",
        ));
    }
    check_prefix(p).map_err(|e| {
        Failure::new(
            "Decoding Error",
            &format!("InvalidPrefixError: {}", e.message),
        )
    })?;
    let len = suffix.encode_utf16().count();
    if len != 26 {
        return Err(Failure::new("Decoding Error",&format!("InvalidSuffixLengthError: Invalid length. Suffix should have 26 characters, got {len}")));
    }
    let first = suffix.chars().next().unwrap_or(' ');
    if first > '7' {
        return Err(Failure::new("Decoding Error",&format!("InvalidSuffixCharacterError: Invalid suffix. First character \"{first}\" must be in the range [0-7]")));
    }
    let mut n = 0u128;
    for c in suffix.bytes() {
        let v = ALPHABET
            .iter()
            .position(|b| *b == c)
            .ok_or_else(|| Failure::new("Decoding Error", "Error: Invalid base32 character"))?;
        n = (n << 5) | v as u128;
    }
    let b = n.to_be_bytes();
    let uuid = uuid_string(b);
    parse_uuid(&uuid)
        .map_err(|_| Failure::new("Decoding Error", "TypeError: Stringified UUID is invalid"))?;
    Ok(uuid)
}
pub fn format_uuid(s: &str) -> Result<String> {
    let h: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if h.len() != 32 {
        return Err(Failure::new(
            "Formatting Error",
            "Error: Invalid UUID format: must be 32 hexadecimal characters",
        ));
    }
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}
fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(""),
        Some(Value::String(s)) if s.len() <= MAX_TEXT => Ok(s),
        Some(Value::String(_)) => Err(resource("Text input exceeds 1 MiB")),
        _ => Err(Failure::new(
            "Invalid argument",
            "Text arguments must be strings",
        )),
    }
}
impl Engine {
    pub const fn new() -> Self {
        Self {
            v7: None,
            type_v7: None,
            ulid: None,
        }
    }
    pub fn v4(&mut self, source: &mut impl Source) -> Result<String> {
        let mut b = [0; 16];
        source.fill(&mut b)?;
        b[6] = (b[6] & 15) | 64;
        b[8] = (b[8] & 63) | 128;
        Ok(uuid_string(b))
    }
    pub fn v7(&mut self, source: &mut impl Source, typeid: bool) -> Result<[u8; 16]> {
        let mut b = [0; 16];
        let now = source.now_ms()?;
        source.fill(&mut b)?;
        if now > 0xffffffffffff {
            return Err(resource("UUIDv7 clock exceeds 48 bits"));
        }
        let ms;
        if typeid {
            let (mut last, mut hi, mut lo) = self.type_v7.unwrap_or((
                0,
                ((u32::from(b[6]) & 127) << 8) | u32::from(b[7]),
                ((u32::from(b[8]) & 63) << 13) | (u32::from(b[9]) << 5) | u32::from(b[10] >> 3),
            ));
            if now > last {
                last = now;
            }
            if now.saturating_add(10000) > last {
                lo += 1;
                if lo > 0x7ffff {
                    lo = 0;
                    hi += 1;
                    if hi > 0xfff {
                        hi = 0;
                        last += 1;
                    }
                }
            } else {
                last = now;
            }
            self.type_v7 = Some((last, hi, lo));
            ms = last;
            b[6] = 0x70 | ((hi >> 4) & 15) as u8;
            b[7] = hi as u8;
            b[8] = 0x80 | ((lo >> 13) & 63) as u8;
            b[9] = (lo >> 5) as u8;
            b[10] = ((lo << 3) as u8) | (b[10] & 7);
        } else {
            let (last, seq) = self.v7.unwrap_or((0, 0));
            let (last, seq) = if self.v7.is_none() || now > last {
                (
                    now,
                    (u32::from(b[6]) << 23)
                        | (u32::from(b[7]) << 16)
                        | (u32::from(b[8]) << 8)
                        | u32::from(b[9]),
                )
            } else {
                let next = seq.wrapping_add(1);
                (last + u64::from(next == 0), next)
            };
            self.v7 = Some((last, seq));
            ms = last;
            b[6] = 0x70 | ((seq >> 28) & 15) as u8;
            b[7] = (seq >> 20) as u8;
            b[8] = 0x80 | ((seq >> 14) & 63) as u8;
            b[9] = (seq >> 6) as u8;
            b[10] = ((seq << 2) as u8) | (b[10] & 3);
        }
        if ms > 0xffffffffffff {
            return Err(resource("UUIDv7 monotonic timestamp overflow"));
        }
        b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
        Ok(b)
    }
    pub fn ulid(&mut self, source: &mut impl Source) -> Result<String> {
        let now = source.now_ms()?;
        let (ms, random) = if let Some((last, r)) = self.ulid.filter(|(last, _)| now <= *last) {
            (
                last,
                r.checked_add(1)
                    .filter(|n| *n < (1u128 << 80))
                    .ok_or_else(|| {
                        Failure::new("Generation failed", "Incorrectly encoded string")
                    })?,
            )
        } else {
            if now == 0 {
                return Err(Failure::new(
                    "Generation failed",
                    "Cannot read properties of undefined (reading 'length')",
                ));
            }
            let mut r = 0u128;
            for i in 0..16 {
                let mut b = [0];
                source.fill(&mut b)?;
                let index = (u32::from(b[0]) * 32 / 255).min(31);
                r |= u128::from(index) << (5 * i);
            }
            (now, r)
        };
        if ms > 0xffffffffffff {
            return Err(resource("ULID clock exceeds 48 bits"));
        }
        self.ulid = Some((ms, random));
        Ok(encode32(((u128::from(ms) << 80) | random).to_be_bytes()).to_uppercase())
    }
    pub fn execute(
        &mut self,
        command: &str,
        args: &Value,
        prefs: &Value,
        source: &mut impl Source,
    ) -> Result<Output> {
        let upper = prefs["upperCaseLetters"].as_bool().unwrap_or(false);
        let action = prefs["defaultAction"]
            .as_str()
            .unwrap_or("copy")
            .to_string();
        if !["copy", "paste"].contains(&action.as_str()) {
            return Err(Failure::new(
                "Invalid preference",
                "defaultAction must be copy or paste",
            ));
        }
        let verb = if action == "copy" { "Copied" } else { "Pasted" };
        let mut entries = Vec::new();
        let (message, text) = if [
            "generate",
            "generateV5",
            "generateV7",
            "generateUlid",
            "generateTypeID",
        ]
        .contains(&command)
        {
            let count_result = count(
                &args[if command == "generateUlid" {
                    "numberOfULIDsToGenerate"
                } else {
                    "numberOfUUIDsToGenerate"
                }],
                command == "generateUlid",
            );
            if let Err(e) = &count_result {
                if e.title == "Invalid number." {
                    return Err(e.clone());
                }
            }
            let namespace = if command == "generateV5" {
                Some(parse_uuid(prefs["uuidNamespace"].as_str().unwrap_or("")).map_err(|_|Failure::new("Invalid namespace.","An invalid namespace has been defined. A namespace must be a valid UUID."))?)
            } else {
                None
            };
            let n = count_result.map_err(|e| {
                if e.message == "Invalid array length"
                    && ["generate", "generateUlid"].contains(&command)
                {
                    Failure::new(
                        "Invalid number.",
                        "An invalid number has been provided. Try an actual number.",
                    )
                } else {
                    e
                }
            })?;
            let prefix = if command == "generateTypeID" {
                text_arg(args, "prefix")?
            } else {
                ""
            };
            let name = if command == "generateV5" {
                text_arg(args, "name")?
            } else {
                ""
            };
            if command == "generateV5" && name.len().saturating_mul(n) > 64 * MAX_TEXT {
                return Err(resource(
                    "Total UUIDv5 name hashing exceeds 64 MiB per invocation",
                ));
            }
            let mut values = Vec::with_capacity(n);
            for _ in 0..n {
                let s = match command {
                    "generate" => self.v4(source)?,
                    "generateV5" => {
                        let mut hash = Sha1::new();
                        hash.update(namespace.unwrap_or([0; 16]));
                        hash.update(name.as_bytes());
                        let digest = hash.finalize();
                        let mut b = [0; 16];
                        b.copy_from_slice(&digest[..16]);
                        b[6] = (b[6] & 15) | 0x50;
                        b[8] = (b[8] & 63) | 128;
                        uuid_string(b)
                    }
                    "generateV7" if !prefs["base32Encoding"].as_bool().unwrap_or(false) => {
                        uuid_string(self.v7(source, false)?)
                    }
                    "generateV7" => encode32(self.v7(source, true)?),
                    "generateUlid" => self.ulid(source).map_err(|_| {
                        Failure::new(
                            "Invalid number.",
                            "An invalid number has been provided. Try an actual number.",
                        )
                    })?,
                    _ => {
                        check_prefix(prefix)?;
                        pack(prefix, self.v7(source, true)?)?
                    }
                };
                values.push(if upper { s.to_uppercase() } else { s });
            }
            let kind = match command {
                "generate" => "uuidV4",
                "generateV5" => "uuidV5",
                "generateV7" => "uuidV7",
                "generateUlid" => "ulid",
                _ => "typeid",
            };
            for s in &values {
                entries.push(HistoryEntry {
                    uuid: s.clone(),
                    timestamp: timestamp(source.now_ms()?)?,
                    kind: kind.into(),
                    extra: Default::default(),
                });
            }
            let noun = if command == "generateUlid" {
                "ULID"
            } else {
                "UUID"
            };
            let message = if values.len() > 1 {
                format!("✅ {verb} {} new {noun}s.", values.len())
            } else {
                format!("✅ {verb} new {noun}: {}", values.join(","))
            };
            (message, values.join("\r\n"))
        } else {
            let (s, description) = match command {
                "formatUuid" => {
                    let s = text_arg(args, "uuid")?;
                    if s.is_empty() {
                        return Err(Failure::new("No UUID provided", ""));
                    }
                    (format_uuid(s)?, "formatted UUID")
                }
                "parseTypeID" => {
                    let s = text_arg(args, "typeID")?;
                    if s.is_empty() {
                        return Err(Failure::new("No TypeID(s) provided", ""));
                    }
                    (parse_typeid(s)?, "decoded UUID")
                }
                "packTypeID" => {
                    let suffix = text_arg(args, "suffix")?;
                    let uuid = text_arg(args, "uuid")?;
                    if suffix.is_empty() {
                        return Err(Failure::new(
                            "Missing suffix.",
                            "A suffix is required to generate a packed type ID.",
                        ));
                    }
                    if uuid.is_empty() {
                        return Err(Failure::new(
                            "Missing UUID.",
                            "A UUID is required to generate a packed type ID.",
                        ));
                    }
                    let packed = pack(
                        suffix,
                        parse_uuid(uuid).map_err(|_| {
                            Failure::new("Error generating packed type ID.", "Invalid UUID")
                        })?,
                    )
                    .map_err(|e| Failure::new("Error generating packed type ID.", &e.message))?;
                    (
                        if upper { packed.to_uppercase() } else { packed },
                        "packed type ID",
                    )
                }
                _ => {
                    return Err(Failure::new(
                        "Unknown command",
                        "This command does not produce identifiers",
                    ))
                }
            };
            (format!("✅ {verb} {description}: {s}"), s)
        };
        Ok(Output {
            text,
            entries,
            message,
            action,
        })
    }
}
fn text_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    text(args, key)
}

/// No legacy migration. The history key remains a JSON string as in the own source.
/// Unknown root keys are untouched. Reject malformed entries rather than silently dropping them.
pub fn history(store: &Value) -> Result<Vec<HistoryEntry>> {
    if !store.is_object() {
        return Err(Failure::new(
            "Storage error",
            "Native storage must be an object",
        ));
    }
    match store.get("uuidHistory") {
        None => Ok(vec![]),
        Some(Value::String(s)) => {
            if s.len() > 32 * MAX_TEXT {
                return Err(resource("History exceeds 32 MiB"));
            }
            let items: Vec<HistoryEntry> = serde_json::from_str(s).map_err(|_| {
                Failure::new(
                    "Error fetching history",
                    "Failed to retrieve history from local storage",
                )
            })?;
            if items.len() > MAX_HISTORY {
                return Err(resource("History exceeds 100000 entries"));
            }
            for e in &items {
                if e.uuid.len() > MAX_TEXT
                    || e.kind.len() > MAX_TEXT
                    || chrono::DateTime::parse_from_rfc3339(&e.timestamp).is_err()
                {
                    return Err(Failure::new(
                        "Error fetching history",
                        "Invalid history entry",
                    ));
                }
            }
            Ok(items)
        }
        _ => Err(Failure::new(
            "Error fetching history",
            "Invalid data type: Expected a string",
        )),
    }
}
pub fn set_history(store: &mut Value, items: &[HistoryEntry]) -> Result<()> {
    if items.len() > MAX_HISTORY {
        return Err(resource(
            "History limit 100000 reached; delete or clear entries",
        ));
    }
    let s = serde_json::to_string(items)
        .map_err(|_| Failure::new("Storage error", "Cannot serialize history"))?;
    if s.len() > 32 * MAX_TEXT {
        return Err(resource("History exceeds 32 MiB"));
    }
    store["uuidHistory"] = Value::String(s);
    Ok(())
}
