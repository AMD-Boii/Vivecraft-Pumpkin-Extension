use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vivecraft_core::config::{self, Settings};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vivecraft-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn defaults_roundtrip_and_schema_bounds() {
    let settings = Settings::default();
    let rendered = settings.render().unwrap();
    assert_eq!(Settings::parse(&rendered).unwrap(), settings);
    assert_eq!(settings.settings.len(), 4);
    assert!(settings.inactive.len() >= 80);
    for (path, value) in settings.settings.iter().chain(&settings.inactive) {
        assert_eq!(
            settings.edit(path, "reset", None).unwrap().get(path),
            Some(value)
        );
    }
    assert!(
        settings
            .edit("general.requestData", "set", Some("garbage"))
            .is_err()
    );
    assert!(
        settings
            .edit("teleport.upLimit", "set", Some("17"))
            .is_err()
    );
    assert!(settings.edit("worldScale.max", "set", Some("NaN")).is_err());
    assert!(settings.edit("worldScale.min", "set", Some("3")).is_err());
    assert!(
        settings
            .edit("general.checkForUpdateType", "set", Some("garbage"))
            .is_err()
    );
    assert_eq!(
        settings
            .edit("vrChanges.bow.headshotIndicator", "set", Some("audio"))
            .unwrap()
            .get("vrChanges.bow.headshotIndicator")
            .unwrap()
            .as_str(),
        Some("AUDIO")
    );
    let added = settings
        .edit("climbey.blocklist", "add", Some("stone"))
        .unwrap();
    assert_eq!(
        added
            .edit("climbey.blocklist", "remove", Some("stone"))
            .unwrap(),
        settings
    );
}
#[test]
fn import_preserves_unknown_and_unavailable_values() {
    let yaml = "general:\n  sendData: false\nteleport:\n  enabled: true\nnewThing:\n  option: 12\n";
    let report = config::import_yaml(yaml).unwrap();
    assert!(!report.settings.boolean("general.sendData"));
    assert!(!report.settings.boolean("teleport.enabled"));
    assert_eq!(
        report.settings.get("teleport.enabled").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(
        report.settings.unmapped["newThing.option"].as_integer(),
        Some(12)
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("teleport.enabled"))
    );
    assert_eq!(
        Settings::parse(&report.settings.render().unwrap()).unwrap(),
        report.settings
    );
}
#[test]
fn yaml_rejects_aliases_tags_duplicate_keys_and_complex_data() {
    for yaml in [
        "general: {sendData: true, sendData: false}",
        "general:\n  sendData: true\n  'sendData': false",
        "general: &base {sendData: true}\nother: *base",
        "general: !!map {sendData: true}",
        "general: {sendData: !thing true}",
        "general: {sendData: null}",
        "general: {sendData: true}\n---\ngeneral: {}",
        "? [complex, key]\n: value",
        "general: {sendData: true}\ngeneral.sendData: false",
    ] {
        assert!(
            config::import_yaml(yaml).is_err(),
            "accepted unsafe YAML: {yaml}"
        );
    }
    // Ordinary quoted messages containing punctuation must not be mistaken for anchors/tags.
    assert!(config::import_yaml("messages:\n  welcomeVR: '&player joined!'\n").is_ok());
}
#[test]
fn staged_import_never_overwrites_or_activates() {
    let folder = Folder::new();
    let yaml = b"general:\n  sendData: false\n";
    fs::write(folder.0.join("old.yml"), yaml).unwrap();
    config::save_atomic(&folder.0, &Settings::default()).unwrap();
    let original = fs::read(folder.0.join("config.toml")).unwrap();
    config::import_file(&folder.0, "old.yml").unwrap();
    assert_eq!(fs::read(folder.0.join("old.yml")).unwrap(), yaml);
    assert_eq!(fs::read(folder.0.join("config.toml")).unwrap(), original);
    let imported = fs::read(folder.0.join("config.imported.toml")).unwrap();
    assert!(config::import_file(&folder.0, "old.yml").is_err());
    assert_eq!(
        fs::read(folder.0.join("config.imported.toml")).unwrap(),
        imported
    );
    for name in [
        "../old.yml",
        "..\\old.yml",
        "D:\\old.yml",
        "old.yml:stream",
        "",
    ] {
        assert!(config::import_file(&folder.0, name).is_err());
    }
}
#[test]
fn failed_reload_and_save_leave_active_configuration_intact() {
    let folder = Folder::new();
    let settings = Settings::default();
    config::save_atomic(&folder.0, &settings).unwrap();
    let bytes = fs::read(folder.0.join("config.toml")).unwrap();
    assert!(Settings::parse("[settings]\n'general.sendData' = 12\n").is_err());
    fs::write(folder.0.join("config.toml.pending"), b"existing work").unwrap();
    assert!(config::save_atomic(&folder.0, &settings).is_err());
    assert_eq!(fs::read(folder.0.join("config.toml")).unwrap(), bytes);
    assert_eq!(
        fs::read(folder.0.join("config.toml.pending")).unwrap(),
        b"existing work"
    );
}
