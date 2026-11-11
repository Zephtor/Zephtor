use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    latest: LatestVersions,
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct LatestVersions {
    release: String,
}

#[derive(Debug, Deserialize)]
struct VersionDetails {
    id: String,
    downloads: VersionDownloads,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    asset_index: Option<AssetIndex>,
}

#[derive(Debug, Deserialize)]
struct Library {
    downloads: Option<LibraryDownloads>,
    natives: Option<std::collections::HashMap<String, String>>,
    rules: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LibraryDownloads {
    artifact: Option<DownloadArtifact>,
    classifiers: Option<std::collections::HashMap<String, DownloadArtifact>>,
}

#[derive(Debug, Deserialize)]
struct DownloadArtifact {
    path: String,
    url: String,
    sha1: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct VersionDownloads {
    client: Download,
}

#[derive(Debug, Deserialize)]
struct Download {
    url: String,
    sha1: String,
}

pub fn game_directory(home: &Path) -> PathBuf {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    game_directory_at(&data_home)
}

fn game_directory_at(data_home: &Path) -> PathBuf {
    data_home.join("zephtor-mc-launcher/game")
}

pub fn config_value(key: &str) -> Option<String> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    let text = fs::read_to_string(config_home.join("zephtor-mc-launcher/launcher.env")).ok()?;
    parse_config_value(&text, key)
}

fn parse_config_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

pub fn fetch_versions() -> Result<(String, Vec<VersionEntry>), String> {
    let response = reqwest::blocking::get(VERSION_MANIFEST_URL)
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Minecraft-Versionsliste konnte nicht geladen werden: {error}"))?;
    let manifest: VersionManifest = response
        .json()
        .map_err(|error| format!("Minecraft-Versionsliste ist ungültig: {error}"))?;
    Ok((manifest.latest.release, manifest.versions))
}

pub fn install_client(game_dir: &Path, version: &VersionEntry) -> Result<PathBuf, String> {
    if !valid_version_id(&version.id) || !version.url.starts_with("https://") {
        return Err("Ungültige Minecraft-Versionsangaben.".to_owned());
    }

    let client = reqwest::blocking::Client::new();
    let metadata_bytes = client
        .get(&version.url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::bytes)
        .map_err(|error| {
            format!("Minecraft-Versionsdaten konnten nicht geladen werden: {error}")
        })?;
    let details: VersionDetails = serde_json::from_slice(&metadata_bytes)
        .map_err(|error| format!("Minecraft-Versionsdaten sind ungültig: {error}"))?;
    if details.id != version.id {
        return Err(
            "Die heruntergeladenen Versionsdaten stimmen nicht mit der Auswahl überein.".to_owned(),
        );
    }
    if !details.downloads.client.url.starts_with("https://") {
        return Err("Ungültige Download-Adresse für den Minecraft-Client.".to_owned());
    }

    let folder = game_dir.join("versions").join(&version.id);
    fs::create_dir_all(&folder)
        .map_err(|error| format!("Versionsordner konnte nicht angelegt werden: {error}"))?;
    let marker = folder.join(".complete");
    match fs::remove_file(&marker) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Alte Installationsmarkierung konnte nicht entfernt werden: {error}"
            ));
        }
    }
    let metadata_path = folder.join(format!("{}.json", version.id));
    fs::write(&metadata_path, metadata_bytes)
        .map_err(|error| format!("Versionsdaten konnten nicht gespeichert werden: {error}"))?;

    let response = client
        .get(&details.downloads.client.url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::bytes)
        .map_err(|error| format!("Minecraft-Client konnte nicht geladen werden: {error}"))?;
    let digest = format!("{:x}", Sha1::digest(response.as_ref()));
    if digest != details.downloads.client.sha1 {
        return Err("Die SHA-1-Prüfung der Minecraft-Datei ist fehlgeschlagen.".to_owned());
    }

    let client_path = folder.join(format!("{}.jar", version.id));
    fs::write(&client_path, &response)
        .map_err(|error| format!("Minecraft-Client konnte nicht gespeichert werden: {error}"))?;

    for library in &details.libraries {
        if !rules_allow(library.rules.as_ref()) {
            continue;
        }
        if let Some(artifact) = library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
        {
            let relative_path = Path::new(&artifact.path);
            if relative_path.is_absolute()
                || relative_path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
                || !artifact.url.starts_with("https://")
            {
                return Err("Ungültiger Minecraft-Bibliothekspfad.".to_owned());
            }
            let path = game_dir.join("libraries").join(relative_path);
            if !download_verified(&client, &artifact.url, &artifact.sha1, &path)? {
                return Err(format!(
                    "Prüfsumme der Bibliothek {} stimmt nicht.",
                    artifact.path
                ));
            }
            if artifact.path.contains("natives-linux") && artifact.path.ends_with(".jar") {
                extract_natives(&path, &folder.join("natives"))?;
            }
        }

        if let (Some(natives), Some(classifiers)) = (
            library.natives.as_ref(),
            library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.classifiers.as_ref()),
        ) {
            let classifier = if cfg!(target_arch = "x86_64") {
                natives
                    .get("linux")
                    .map(|value| value.replace("${arch}", "64"))
            } else {
                natives.get("linux").cloned()
            };
            if let Some(native) = classifier.and_then(|key| classifiers.get(&key)) {
                let relative_path = Path::new(&native.path);
                if relative_path.is_absolute()
                    || relative_path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                    || !native.url.starts_with("https://")
                {
                    return Err("Ungültiger Minecraft-Native-Pfad.".to_owned());
                }
                let archive = game_dir.join("libraries").join(relative_path);
                if !download_verified(&client, &native.url, &native.sha1, &archive)? {
                    return Err(format!(
                        "Prüfsumme der nativen Bibliothek {} stimmt nicht.",
                        native.path
                    ));
                }
                extract_natives(&archive, &folder.join("natives"))?;
            }
        }
    }

    if let Some(asset_index) = &details.asset_index {
        if !valid_version_id(&asset_index.id) || !asset_index.url.starts_with("https://") {
            return Err("Ungültiger Minecraft-Asset-Index.".to_owned());
        }
        let index_path = game_dir
            .join("assets/indexes")
            .join(format!("{}.json", asset_index.id));
        download_file(&client, &asset_index.url, &index_path)?;
        let index_bytes = fs::read(&index_path)
            .map_err(|error| format!("Asset-Index konnte nicht gelesen werden: {error}"))?;
        let index: serde_json::Value = serde_json::from_slice(&index_bytes)
            .map_err(|error| format!("Minecraft-Asset-Index ist ungültig: {error}"))?;
        if let Some(objects) = index.get("objects").and_then(serde_json::Value::as_object) {
            for object in objects.values() {
                let Some(hash) = object.get("hash").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err("Ungültiger Hash im Minecraft-Asset-Index.".to_owned());
                }
                let prefix = &hash[..2];
                let url = format!("https://resources.download.minecraft.net/{prefix}/{hash}");
                let path = game_dir.join("assets/objects").join(prefix).join(hash);
                if !download_verified(&client, &url, hash, &path)? {
                    return Err(format!("Prüfsumme des Assets {hash} stimmt nicht."));
                }
            }
        }
    }
    fs::write(&marker, b"complete")
        .map_err(|error| format!("Installationsstatus konnte nicht gespeichert werden: {error}"))?;
    Ok(client_path)
}

fn extract_natives(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path)
        .map_err(|error| format!("Native-Bibliothek konnte nicht geöffnet werden: {error}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("Native-Bibliothek ist ungültig: {error}"))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Native-Archiv konnte nicht gelesen werden: {error}"))?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        if relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
            || relative.starts_with("META-INF")
        {
            continue;
        }
        if entry.is_dir() {
            continue;
        }
        let Some(name) = relative.file_name() else {
            continue;
        };
        fs::create_dir_all(destination)
            .map_err(|error| format!("Native-Ordner konnte nicht angelegt werden: {error}"))?;
        let output = destination.join(name);
        let mut file = fs::File::create(&output)
            .map_err(|error| format!("Native-Datei konnte nicht angelegt werden: {error}"))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|error| format!("Native-Datei konnte nicht entpackt werden: {error}"))?;
    }
    Ok(())
}

fn download_verified(
    client: &reqwest::blocking::Client,
    url: &str,
    expected_sha1: &str,
    path: &Path,
) -> Result<bool, String> {
    if path.is_file()
        && let Ok(bytes) = fs::read(path)
        && format!("{:x}", Sha1::digest(&bytes)) == expected_sha1
    {
        return Ok(true);
    }
    let response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::bytes)
        .map_err(|error| format!("Minecraft-Datei konnte nicht geladen werden: {error}"))?;
    if format!("{:x}", Sha1::digest(response.as_ref())) != expected_sha1 {
        return Ok(false);
    }
    download_file_bytes(path, &response)?;
    Ok(true)
}

fn download_file(client: &reqwest::blocking::Client, url: &str, path: &Path) -> Result<(), String> {
    let bytes = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::bytes)
        .map_err(|error| format!("Minecraft-Datei konnte nicht geladen werden: {error}"))?;
    download_file_bytes(path, &bytes)
}

fn download_file_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Ungültiger Minecraft-Dateipfad.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Minecraft-Ordner konnte nicht angelegt werden: {error}"))?;
    fs::write(path, bytes)
        .map_err(|error| format!("Minecraft-Datei konnte nicht gespeichert werden: {error}"))
}

pub fn run_cli(args: &[String]) -> Option<Result<(), String>> {
    match args.first().map(String::as_str) {
        Some("--list-versions") => Some((|| {
            let (latest, versions) = fetch_versions()?;
            println!("Neueste stabile Version: {latest}");
            for version in versions.iter().filter(|version| version.kind == "release") {
                println!("{}", version.id);
            }
            Ok(())
        })()),
        Some("--install-version") => Some((|| {
            let version_id = args
                .get(1)
                .ok_or("Verwendung: --install-version <Version>")?;
            let (_, versions) = fetch_versions()?;
            let version = versions
                .iter()
                .find(|version| version.id == *version_id)
                .ok_or_else(|| format!("Unbekannte Minecraft-Version: {version_id}"))?;
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME ist nicht gesetzt")?;
            let path = install_client(&game_directory(&home), version)?;
            println!("Minecraft-Version installiert: {}", path.display());
            Ok(())
        })()),
        Some("--launch-version") => Some((|| {
            let version_id = args
                .get(1)
                .ok_or("Verwendung: --launch-version <Version>")?;
            if !valid_version_id(version_id) {
                return Err("Ungültige Minecraft-Version.".to_owned());
            }
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME ist nicht gesetzt")?;
            let game_dir = game_directory(&home);
            if !game_dir
                .join("versions")
                .join(version_id)
                .join(".complete")
                .is_file()
            {
                return Err("Diese Minecraft-Version ist noch nicht installiert.".to_owned());
            }
            let account = match (
                std::env::var("ZEPHTOR_MINECRAFT_ACCESS_TOKEN"),
                std::env::var("ZEPHTOR_MINECRAFT_PLAYER"),
                std::env::var("ZEPHTOR_MINECRAFT_UUID"),
            ) {
                (Ok(access_token), Ok(name), Ok(uuid)) => MinecraftAccount {
                    name,
                    uuid,
                    access_token,
                },
                (Err(_), Err(_), Err(_)) => {
                    authenticate_minecraft(&|message| println!("{message}"))?
                }
                _ => {
                    return Err(
                        "Minecraft-Sitzungsvariablen müssen gemeinsam gesetzt sein.".to_owned()
                    );
                }
            };
            launch_version(
                &game_dir,
                version_id,
                &account.name,
                &account.uuid,
                &account.access_token,
            )
        })()),
        _ => None,
    }
}

pub fn launch_latest(notify: &dyn Fn(&str)) -> Result<(), String> {
    let (latest, versions) = fetch_versions()?;
    let version = versions
        .iter()
        .find(|version| version.id == latest)
        .ok_or_else(|| format!("Release {latest} fehlt in der Versionsliste."))?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME ist nicht gesetzt")?;
    let game_dir = game_directory(&home);
    if !game_dir
        .join("versions")
        .join(&version.id)
        .join(".complete")
        .is_file()
    {
        return Err(format!("Minecraft {latest} ist noch nicht installiert."));
    }
    notify("Starte die offizielle Microsoft-Anmeldung …");
    let account = authenticate_minecraft(notify)?;
    launch_version(
        &game_dir,
        &version.id,
        &account.name,
        &account.uuid,
        &account.access_token,
    )
}

fn launch_version(
    game_dir: &Path,
    version_id: &str,
    player_name: &str,
    player_uuid: &str,
    access_token: &str,
) -> Result<(), String> {
    if !valid_version_id(version_id) {
        return Err("Ungültige Minecraft-Version.".to_owned());
    }
    let version_dir = game_dir.join("versions").join(version_id);
    let metadata_path = version_dir.join(format!("{version_id}.json"));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).map_err(|error| {
            format!("Versionsdaten fehlen; Version zuerst installieren: {error}")
        })?)
        .map_err(|error| format!("Versionsdaten sind ungültig: {error}"))?;

    let mut classpath = Vec::new();
    if let Some(libraries) = metadata
        .get("libraries")
        .and_then(serde_json::Value::as_array)
    {
        for library in libraries {
            if !rules_allow(library.get("rules")) {
                continue;
            }
            if let Some(path) = library
                .pointer("/downloads/artifact/path")
                .and_then(serde_json::Value::as_str)
                && !path.contains("natives-")
            {
                classpath.push(game_dir.join("libraries").join(path));
            }
        }
    }
    classpath.push(version_dir.join(format!("{version_id}.jar")));
    let classpath = std::env::join_paths(classpath)
        .map_err(|error| format!("Java-Classpath ist ungültig: {error}"))?;
    let assets_id = metadata
        .pointer("/assetIndex/id")
        .and_then(serde_json::Value::as_str)
        .ok_or("Minecraft-Asset-Index fehlt; Version erneut installieren.")?;
    let main_class = metadata
        .get("mainClass")
        .and_then(serde_json::Value::as_str)
        .ok_or("Minecraft-Hauptklasse fehlt.")?;
    let client_id = std::env::var("ZEPHTOR_MSA_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(read_msa_client_id)
        .unwrap_or_default();
    let natives_dir = version_dir.join("natives");
    let java = std::env::var_os("JAVA")
        .or_else(|| config_value("JAVA").map(Into::into))
        .unwrap_or_else(|| "java".into());
    let mut command = Command::new(java);
    command
        .arg("-Xmx2G")
        .arg(format!("-Djava.library.path={}", natives_dir.display()));
    let replacements = [
        ("${auth_player_name}", player_name.to_owned()),
        ("${version_name}", version_id.to_owned()),
        ("${game_directory}", game_dir.to_string_lossy().into_owned()),
        (
            "${assets_root}",
            game_dir.join("assets").to_string_lossy().into_owned(),
        ),
        ("${assets_index_name}", assets_id.to_owned()),
        ("${auth_uuid}", player_uuid.to_owned()),
        ("${auth_access_token}", access_token.to_owned()),
        ("${auth_session}", access_token.to_owned()),
        ("${user_properties}", "{}".to_owned()),
        ("${user_type}", "msa".to_owned()),
        ("${version_type}", "release".to_owned()),
        ("${clientid}", client_id),
        ("${auth_xuid}", String::new()),
        ("${resolution_width}", "854".to_owned()),
        ("${resolution_height}", "480".to_owned()),
        (
            "${natives_directory}",
            natives_dir.to_string_lossy().into_owned(),
        ),
        (
            "${library_directory}",
            game_dir.join("libraries").to_string_lossy().into_owned(),
        ),
        ("${classpath}", classpath.to_string_lossy().into_owned()),
        ("${classpath_separator}", ":".to_owned()),
    ];
    if let Some(arguments) = metadata
        .pointer("/arguments/jvm")
        .and_then(serde_json::Value::as_array)
    {
        for value in arguments {
            for argument in resolved_arguments(value) {
                command.arg(replace_variables(&argument, &replacements));
            }
        }
    }
    command.arg("-cp").arg(classpath).arg(main_class);
    if let Some(arguments) = metadata
        .pointer("/arguments/game")
        .and_then(serde_json::Value::as_array)
    {
        for argument in arguments {
            for value in resolved_arguments(argument) {
                let value = replace_variables(&value, &replacements);
                if value.contains("${") {
                    return Err(format!("Unbekannter Minecraft-Parameter: {value}"));
                }
                command.arg(value);
            }
        }
    } else if let Some(arguments) = metadata
        .get("minecraftArguments")
        .and_then(serde_json::Value::as_str)
    {
        for argument in arguments.split_whitespace() {
            let mut value = argument.to_owned();
            for (key, replacement) in &replacements {
                value = value.replace(key, replacement);
            }
            command.arg(value);
        }
    } else {
        return Err("Minecraft-Startargumente fehlen.".to_owned());
    }
    command.current_dir(game_dir);
    command
        .spawn()
        .map_err(|error| format!("Java konnte Minecraft nicht starten: {error}"))?;
    Ok(())
}

fn replace_variables(argument: &str, replacements: &[(&str, String)]) -> String {
    replacements
        .iter()
        .fold(argument.to_owned(), |value, (key, replacement)| {
            value.replace(key, replacement)
        })
}

fn resolved_arguments(value: &serde_json::Value) -> Vec<String> {
    if let Some(argument) = value.as_str() {
        return vec![argument.to_owned()];
    }
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    if !rules_allow(object.get("rules")) {
        return Vec::new();
    }
    match object.get("value") {
        Some(serde_json::Value::String(argument)) => vec![argument.clone()],
        Some(serde_json::Value::Array(arguments)) => arguments
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn rules_allow(rules: Option<&serde_json::Value>) -> bool {
    let Some(rules) = rules.and_then(serde_json::Value::as_array) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let os_matches = rule
            .pointer("/os/name")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|name| name == "linux");
        let architecture_matches = rule
            .pointer("/os/arch")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|arch| {
                (arch.contains("64") && cfg!(target_pointer_width = "64"))
                    || (arch.contains("32") && cfg!(target_pointer_width = "32"))
            });
        let features_match = rule
            .get("features")
            .and_then(serde_json::Value::as_object)
            .is_none_or(|features| {
                features
                    .values()
                    .all(|expected| expected.as_bool() == Some(false))
            });
        if os_matches && architecture_matches && features_match {
            allowed = rule.get("action").and_then(serde_json::Value::as_str) == Some("allow");
        }
    }
    allowed
}

struct MinecraftAccount {
    name: String,
    uuid: String,
    access_token: String,
}

fn authenticate_minecraft(notify: &dyn Fn(&str)) -> Result<MinecraftAccount, String> {
    let client_id = std::env::var("ZEPHTOR_MSA_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(read_msa_client_id)
        .ok_or_else(|| {
            "Microsoft-Anmeldung benötigt eine öffentliche OAuth-App-ID. Trage sie in den Einstellungen ein."
                .to_owned()
        })?;
    let client = reqwest::blocking::Client::new();
    let device: serde_json::Value = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Microsoft-Anmeldung konnte nicht gestartet werden: {error}"))?;
    let instructions = device
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Öffne die Microsoft-Geräteanmeldung und gib den angezeigten Code ein.");
    notify(instructions);
    if let Some(uri) = device
        .get("verification_uri")
        .and_then(serde_json::Value::as_str)
        .filter(|uri| uri.starts_with("https://microsoft.com/"))
    {
        let _ = Command::new("xdg-open").arg(uri).spawn();
    }
    let device_code = device
        .get("device_code")
        .and_then(serde_json::Value::as_str)
        .ok_or("Microsoft hat keinen Gerätecode zurückgegeben.")?;
    let interval = device
        .get("interval")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(5);
    let expires_in = device
        .get("expires_in")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(900);
    let start = std::time::Instant::now();
    let msa_token = loop {
        if start.elapsed().as_secs() >= expires_in {
            return Err("Microsoft-Anmeldecode ist abgelaufen.".to_owned());
        }
        thread::sleep(Duration::from_secs(interval));
        let response = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id.as_str()),
                ("device_code", device_code),
            ])
            .send()
            .map_err(|error| format!("Microsoft-Anmeldung fehlgeschlagen: {error}"))?;
        let body: serde_json::Value = response
            .json()
            .map_err(|error| format!("Microsoft-Antwort ist ungültig: {error}"))?;
        if let Some(token) = body.get("access_token").and_then(serde_json::Value::as_str) {
            break token.to_owned();
        }
        match body.get("error").and_then(serde_json::Value::as_str) {
            Some("authorization_pending") => continue,
            Some("slow_down") => thread::sleep(Duration::from_secs(5)),
            Some(error) => return Err(format!("Microsoft-Anmeldung abgelehnt: {error}")),
            None => return Err("Microsoft hat kein Zugriffstoken zurückgegeben.".to_owned()),
        }
    };

    let xbl: serde_json::Value = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={msa_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Xbox-Anmeldung fehlgeschlagen: {error}"))?;
    let xsts: serde_json::Value = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl.get("Token").and_then(serde_json::Value::as_str)
                    .ok_or("Xbox hat kein Zugriffstoken zurückgegeben.")?]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Xbox-XSTS-Anmeldung fehlgeschlagen: {error}"))?;
    let user_hash = xbl
        .pointer("/DisplayClaims/xui/0/uhs")
        .and_then(serde_json::Value::as_str)
        .ok_or("Xbox hat keine Benutzerkennung zurückgegeben.")?;
    let xsts_token = xsts
        .get("Token")
        .and_then(serde_json::Value::as_str)
        .ok_or("Xbox-XSTS hat kein Zugriffstoken zurückgegeben.")?;
    let minecraft: serde_json::Value = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")
        }))
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Minecraft-Kontoanmeldung fehlgeschlagen: {error}"))?;
    let access_token = minecraft
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .ok_or("Minecraft hat kein Zugriffstoken zurückgegeben.")?
        .to_owned();
    let profile: serde_json::Value = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&access_token)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| format!("Minecraft-Profil konnte nicht geladen werden: {error}"))?;
    Ok(MinecraftAccount {
        name: profile
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or("Minecraft-Profil enthält keinen Namen.")?
            .to_owned(),
        uuid: profile
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("Minecraft-Profil enthält keine UUID.")?
            .to_owned(),
        access_token,
    })
}

fn read_msa_client_id() -> Option<String> {
    if let Some(value) = config_value("ZEPHTOR_MSA_CLIENT_ID")
        && !value.trim().is_empty()
    {
        return Some(value.trim().to_owned());
    }
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?
        .join("zephtor-mc-launcher/microsoft-client-id.txt");
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn valid_version_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ids_cannot_escape_the_game_directory() {
        assert!(valid_version_id("1.21.1"));
        assert!(!valid_version_id("../outside"));
        assert!(!valid_version_id(""));
    }

    #[test]
    fn game_directory_is_separate_from_prism_data() {
        assert_eq!(
            game_directory_at(Path::new("/home/player/.local/share")),
            PathBuf::from("/home/player/.local/share/zephtor-mc-launcher/game")
        );
    }

    #[test]
    fn config_reads_public_app_id_and_ignores_comments() {
        let config = "# ZEPHTOR_MSA_CLIENT_ID=comment\nJAVA=/usr/bin/java\nZEPHTOR_MSA_CLIENT_ID=public-id\n";
        assert_eq!(
            parse_config_value(config, "ZEPHTOR_MSA_CLIENT_ID").as_deref(),
            Some("public-id")
        );
        assert_eq!(
            parse_config_value(config, "JAVA").as_deref(),
            Some("/usr/bin/java")
        );
    }

    #[test]
    fn linux_rules_exclude_other_platforms_and_demo_only_arguments() {
        let windows = serde_json::json!([{
            "action": "allow",
            "os": { "name": "windows" }
        }]);
        let linux = serde_json::json!([{
            "action": "allow",
            "os": { "name": "linux" }
        }]);
        let demo_only = serde_json::json!([{
            "action": "allow",
            "features": { "is_demo_user": true }
        }]);
        assert!(!rules_allow(Some(&windows)));
        assert!(rules_allow(Some(&linux)));
        assert!(!rules_allow(Some(&demo_only)));
    }

    #[test]
    fn extracts_native_files_without_archive_subdirectories() {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("zephtor-natives-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("natives.jar");
        let file = fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "linux/x64/org/lwjgl/liblwjgl.so",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"native").unwrap();
        archive.finish().unwrap();

        let destination = root.join("natives");
        extract_natives(&archive_path, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("liblwjgl.so")).unwrap(),
            b"native"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
