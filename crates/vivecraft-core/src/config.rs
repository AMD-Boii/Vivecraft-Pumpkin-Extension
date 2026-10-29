//! Configuration preserves legacy keys without claiming unimplemented capabilities.
use saphyr::{LoadableYamlNode, Yaml};
use saphyr_parser::{Event, EventReceiver, Parser};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};
use toml::Value;

pub const MAX_CONFIG_BYTES: u64 = 1_048_576;
pub const CONFIG_EXAMPLE: &str = include_str!("../../../config.example.toml");
pub const COMMANDS: &[&str] = &["vivecraft-server-config", "vive", "vse"];
pub const PERMISSIONS: &[&str] = &[
    "vivecraft.config",
    "vivecraft.no-pvp-standing-vs-standing",
    "vivecraft.no-pvp-standing-vs-seated",
    "vivecraft.no-pvp-standing-vs-nonvr",
    "vivecraft.no-pvp-seated-vs-seated",
    "vivecraft.no-pvp-seated-vs-nonvr",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
    #[serde(default)]
    pub inactive: BTreeMap<String, Value>,
    /// Unknown imported values remain data, never effective runtime settings.
    #[serde(default)]
    pub unmapped: BTreeMap<String, Value>,
}
impl Default for Settings {
    fn default() -> Self {
        toml::from_str(CONFIG_EXAMPLE).expect("checked-in default TOML must parse")
    }
}
impl Settings {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() as u64 > MAX_CONFIG_BYTES {
            return Err("configuration exceeds 1 MiB".into());
        }
        let loaded: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut result = Self::default();
        for (path, value) in loaded.settings {
            if !result.settings.contains_key(&path) {
                return Err(format!("not an active setting: {path}"));
            }
            result.assign(&path, value)?;
        }
        for (path, value) in loaded.inactive {
            if !result.inactive.contains_key(&path) {
                return Err(format!("not a recognized inactive setting: {path}"));
            }
            result.assign(&path, value)?;
        }
        result.unmapped = loaded.unmapped;
        result.validate()?;
        Ok(result)
    }
    pub fn boolean(&self, path: &str) -> bool {
        self.settings
            .get(path)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.settings.get(path).or_else(|| self.inactive.get(path))
    }
    pub fn active(&self, path: &str) -> bool {
        self.settings.contains_key(path)
    }
    pub fn render(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }
    fn assign(&mut self, path: &str, value: Value) -> Result<(), String> {
        let expected = self
            .get(path)
            .ok_or_else(|| format!("unknown setting: {path}"))?;
        let value = match (expected, value) {
            (Value::Float(_), Value::Integer(n)) => Value::Float(n as f64),
            (Value::String(_), Value::String(s)) if enum_values(path).is_some() => {
                Value::String(s.to_ascii_uppercase())
            }
            (_, value) => value,
        };
        if std::mem::discriminant(expected) != std::mem::discriminant(&value) {
            return Err(format!("wrong type for {path}"));
        }
        if let Some(values) = value.as_array()
            && (values.len() > 4096 || values.iter().any(|v| v.as_str().is_none()))
        {
            return Err(format!("expected bounded string list: {path}"));
        }
        if value.as_str().is_some_and(|s| s.len() > 32767) {
            return Err(format!("string too long: {path}"));
        }
        if let Some(allowed) = enum_values(path)
            && !allowed.contains(&value.as_str().unwrap_or_default())
        {
            return Err(format!("invalid enum: {path}"));
        }
        let number = value
            .as_float()
            .or_else(|| value.as_integer().map(|n| n as f64));
        if let Some(n) = number {
            let (min, max) =
                limits(path).ok_or_else(|| format!("missing numeric schema for {path}"))?;
            if !n.is_finite() || n < min || n > max {
                return Err(format!("{path} must be in {min}..={max}"));
            }
        }
        if let Some(slot) = self.settings.get_mut(path) {
            *slot = value;
        } else {
            self.inactive.insert(path.into(), value);
        }
        Ok(())
    }
    fn validate(&self) -> Result<(), String> {
        let number = |path| self.get(path).and_then(Value::as_float).unwrap_or_default();
        if number("worldScale.min") > number("worldScale.max") {
            return Err("worldScale.min exceeds worldScale.max".into());
        }
        Ok(())
    }
    /// Returns a new candidate. The caller persists it before replacing live state.
    pub fn edit(&self, path: &str, action: &str, argument: Option<&str>) -> Result<Self, String> {
        let mut result = self.clone();
        let value = self
            .get(path)
            .ok_or_else(|| format!("unknown setting: {path}"))?;
        if action != "reset" && argument.is_none() {
            return Err("missing setting value".into());
        }
        let argument = argument.unwrap_or_default();
        let next = match action {
            "reset" => Self::default()
                .get(path)
                .cloned()
                .ok_or("unknown default")?,
            "set" => match value {
                Value::Boolean(_) => {
                    Value::Boolean(argument.parse().map_err(|_| "expected true or false")?)
                }
                Value::Integer(_) => {
                    Value::Integer(argument.parse().map_err(|_| "expected integer")?)
                }
                Value::Float(_) => Value::Float(argument.parse().map_err(|_| "expected number")?),
                Value::String(_) => Value::String(argument.into()),
                _ => return Err("use add/remove for list settings".into()),
            },
            "add" | "remove" => {
                let mut list = value.as_array().ok_or("not a list setting")?.clone();
                let entry = Value::String(argument.into());
                if action == "add" && !list.contains(&entry) {
                    list.push(entry);
                } else if action == "remove" {
                    list.retain(|v| v != &entry);
                }
                Value::Array(list)
            }
            _ => return Err("unknown action".into()),
        };
        result.assign(path, next)?;
        result.validate()?;
        Ok(result)
    }
}
fn enum_values(path: &str) -> Option<&'static [&'static str]> {
    match path {
        "general.checkForUpdateType" => Some(&["RELEASE", "BETA", "ALPHA"]),
        "climbey.blockmode" => Some(&["DISABLED", "WHITELIST", "BLACKLIST"]),
        "vrChanges.bow.headshotIndicator" => Some(&["NONE", "AUDIO", "VISUAL", "BOTH"]),
        _ => None,
    }
}
fn limits(path: &str) -> Option<(f64, f64)> {
    Some(match path {
        "general.messageAndKickDelay" => (100., 1000.),
        "setSpigotSettings.movedWronglyThreshold" | "setSpigotSettings.movedTooQuickly" => {
            (0., 10000.)
        }
        "vrChanges.creeperSwellDistance" => (0.1, 10.),
        "vrChanges.mobAttackRangeAdjustment" => (-0.7, 0.),
        "vrChanges.bootsArmorDamage" => (0., 5.),
        "vrChanges.projectileInaccuracyMultiplier" => (0., 1.),
        "vrChanges.roomscaleShieldCooldown" => (0., 1000.),
        "vrChanges.roomscaleShieldCooldownDamageTrigger" => (0., 100.),
        "teleport.upLimit" | "teleport.downLimit" => (1., 16.),
        "teleport.horizontalLimit" => (1., 32.),
        "worldScale.min" | "worldScale.max" => (0.1, 100.),
        _ if path.starts_with("vrChanges.bow.") => (1., 10.),
        _ => return None,
    })
}

// Validate the event stream before the general loader can resolve aliases or overwrite duplicate keys.
#[derive(Default)]
struct SafeYaml {
    stack: Vec<Option<(bool, BTreeSet<String>)>>,
    error: Option<String>,
    documents: usize,
    nodes: usize,
}
impl SafeYaml {
    fn fail(&mut self, message: &str) {
        if self.error.is_none() {
            self.error = Some(message.into());
        }
    }
    fn scalar(&mut self, value: &str) {
        if let Some(Some((key, seen))) = self.stack.last_mut() {
            if *key && (value == "<<" || !seen.insert(value.into())) {
                self.fail("duplicate or merge YAML key");
                return;
            }
            *key = !*key;
        }
    }
    fn container(&mut self) {
        if let Some(Some((key, _))) = self.stack.last_mut() {
            if *key {
                self.fail("complex YAML keys are not supported");
            } else {
                *key = true;
            }
        }
    }
}
impl<'a> EventReceiver<'a> for SafeYaml {
    fn on_event(&mut self, event: Event<'a>) {
        self.nodes += 1;
        if self.nodes > 20_000 {
            self.fail("too many YAML nodes");
        }
        match event {
            Event::DocumentStart(_) => {
                self.documents += 1;
                if self.documents > 1 {
                    self.fail("only one YAML document is supported");
                }
            }
            Event::Alias(_) => self.fail("YAML aliases are not supported"),
            Event::Scalar(value, _, anchor, tag) => {
                if anchor != 0 || tag.is_some() {
                    self.fail("YAML anchors/tags are not supported");
                }
                self.scalar(&value);
            }
            Event::MappingStart(anchor, ref tag) | Event::SequenceStart(anchor, ref tag) => {
                if anchor != 0 || tag.is_some() {
                    self.fail("YAML anchors/tags are not supported");
                }
                self.container();
                self.stack
                    .push(if matches!(event, Event::MappingStart(..)) {
                        Some((true, BTreeSet::new()))
                    } else {
                        None
                    });
                if self.stack.len() > 32 {
                    self.fail("YAML nesting exceeds 32 levels");
                }
            }
            Event::MappingEnd | Event::SequenceEnd => {
                self.stack.pop();
            }
            _ => {}
        }
    }
}

pub struct ImportReport {
    pub settings: Settings,
    pub warnings: Vec<String>,
}
pub fn import_yaml(text: &str) -> Result<ImportReport, String> {
    if text.len() as u64 > MAX_CONFIG_BYTES {
        return Err("YAML exceeds 1 MiB".into());
    }
    let mut validator = SafeYaml::default();
    Parser::new_from_str(text)
        .load(&mut validator, true)
        .map_err(|e| e.to_string())?;
    if let Some(error) = validator.error {
        return Err(error);
    }
    let docs = Yaml::load_from_str(text).map_err(|e| e.to_string())?;
    let doc = docs.first().ok_or("empty YAML")?;
    let mut flat = BTreeMap::new();
    flatten_yaml(doc, "", &mut flat)?;
    let mut report = ImportReport {
        settings: Settings::default(),
        warnings: Vec::new(),
    };
    for (path, value) in flat {
        if report.settings.get(&path).is_some() {
            report.settings.assign(&path, value)?;
            if !report.settings.active(&path) {
                report
                    .warnings
                    .push(format!("retained but inactive: {path}"));
            }
        } else {
            report.warnings.push(format!("unmapped: {path}"));
            report.settings.unmapped.insert(path, value);
        }
    }
    report.settings.validate()?;
    Ok(report)
}
fn yaml_scalar(node: &Yaml<'_>) -> Result<Value, String> {
    if let Some(v) = node.as_bool() {
        Ok(Value::Boolean(v))
    } else if let Some(v) = node.as_integer() {
        Ok(Value::Integer(v))
    } else if let Some(v) = node.as_floating_point() {
        if v.is_finite() {
            Ok(Value::Float(v))
        } else {
            Err("non-finite YAML number".into())
        }
    } else if let Some(v) = node.as_str() {
        Ok(Value::String(v.into()))
    } else if let Some(seq) = node.as_vec() {
        seq.iter()
            .map(yaml_scalar)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    } else {
        Err("unsupported YAML value (null, tag, or complex list)".into())
    }
}
fn flatten_yaml(
    node: &Yaml<'_>,
    prefix: &str,
    out: &mut BTreeMap<String, Value>,
) -> Result<(), String> {
    let map = node.as_mapping().ok_or("expected YAML mapping")?;
    for (key, value) in map {
        let key = key.as_str().ok_or("expected string YAML key")?;
        let path = if prefix.is_empty() {
            key.into()
        } else {
            format!("{prefix}.{key}")
        };
        if value.as_mapping().is_some() {
            flatten_yaml(value, &path, out)?;
        } else if out.insert(path.clone(), yaml_scalar(value)?).is_some() {
            return Err(format!("duplicate flattened path: {path}"));
        }
    }
    Ok(())
}

pub fn read_bounded(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_CONFIG_BYTES {
        return Err("configuration exceeds 1 MiB".into());
    }
    let mut text = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > MAX_CONFIG_BYTES {
        return Err("configuration exceeds 1 MiB".into());
    }
    Ok(text)
}
/// Import only a regular staged file directly inside the plugin data directory; never activate it.
pub fn import_file(folder: &Path, name: &str) -> Result<Vec<String>, String> {
    let name_path = Path::new(name);
    if name_path.components().count() != 1
        || !matches!(name_path.components().next(), Some(Component::Normal(_)))
        || name.contains(['/', '\\', ':'])
    {
        return Err("supply a plain staged filename".into());
    }
    let source = folder.join(name);
    let metadata = fs::symlink_metadata(&source).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("import source must be a regular file".into());
    }
    let root = folder.canonicalize().map_err(|e| e.to_string())?;
    if source.canonicalize().map_err(|e| e.to_string())?.parent() != Some(root.as_path()) {
        return Err("source escapes plugin data directory".into());
    }
    let report = import_yaml(&read_bounded(&source)?)?;
    let output = folder.join("config.imported.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(report.settings.render()?.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(report.warnings)
}
/// Atomic replace using same-directory rename. On platforms that refuse replacement, fail closed.
pub fn save_atomic(folder: &Path, settings: &Settings) -> Result<(), String> {
    let text = settings.render()?;
    let temporary = folder.join("config.toml.pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, folder.join("config.toml")).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
