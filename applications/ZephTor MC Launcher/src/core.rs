use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const PRISM_FLATPAK_ID: &str = "org.prismlauncher.PrismLauncher";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrismCommand {
    pub executable: PathBuf,
    pub flatpak: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinecraftInstance {
    pub instance_id: String,
    pub name: String,
    pub data_root: PathBuf,
    pub minecraft_version: String,
    pub loader: String,
}

pub fn default_data_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/PrismLauncher"),
        home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"),
        home.join(".local/share/multimc"),
    ]
}

pub fn normalize_data_root(path: &Path) -> PathBuf {
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("instances"))
    {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn value_for_key(config: &str, key: &str) -> Option<String> {
    let mut in_general_section = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_general_section = line[1..line.len() - 1].eq_ignore_ascii_case("General");
            continue;
        }
        if !in_general_section {
            continue;
        }
        let Some((candidate, value)) = line.split_once('=') else {
            continue;
        };
        if candidate.trim().eq_ignore_ascii_case(key) {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn read_instance(folder: &Path, data_root: &Path) -> Option<MinecraftInstance> {
    let instance_id = folder.file_name()?.to_string_lossy().into_owned();
    let config = std::fs::read_to_string(folder.join("instance.cfg")).unwrap_or_default();
    let name = value_for_key(&config, "name")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| instance_id.clone());
    let minecraft_version = value_for_key(&config, "MinecraftVersion")
        .or_else(|| value_for_key(&config, "IntendedVersion"));
    let loader = [
        ("NeoForgeVersion", "NeoForge"),
        ("ForgeVersion", "Forge"),
        ("FabricVersion", "Fabric"),
        ("QuiltVersion", "Quilt"),
    ]
    .into_iter()
    .find_map(|(key, label)| {
        value_for_key(&config, key)
            .filter(|value| !value.is_empty())
            .map(|_| label)
    })
    .unwrap_or_default()
    .to_owned();

    Some(MinecraftInstance {
        instance_id,
        name,
        data_root: data_root.to_path_buf(),
        minecraft_version: minecraft_version.unwrap_or_default(),
        loader,
    })
}

pub fn find_instances(data_roots: &[PathBuf]) -> Vec<MinecraftInstance> {
    let mut instances = Vec::new();
    let mut seen = HashSet::new();

    for candidate in data_roots {
        let data_root = normalize_data_root(candidate);
        let instances_dir = data_root.join("instances");
        let Ok(entries) = std::fs::read_dir(instances_dir) else {
            continue;
        };

        let mut folders: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
        folders.sort_by_key(|path| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });
        for folder in folders {
            if !folder.is_dir() || !folder.join("instance.cfg").is_file() {
                continue;
            }
            let Some(instance) = read_instance(&folder, &data_root) else {
                continue;
            };
            if seen.insert((data_root.clone(), instance.instance_id.clone())) {
                instances.push(instance);
            }
        }
    }

    instances
}

fn find_program(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        let candidate = directory.join(name);
        candidate.is_file().then_some(candidate)
    })
}

pub fn find_prism_command() -> Option<PrismCommand> {
    for name in ["prismlauncher", "prismlauncher-qt", "PrismLauncher"] {
        if let Some(executable) = find_program(name) {
            return Some(PrismCommand {
                executable,
                flatpak: false,
            });
        }
    }

    let executable = find_program("flatpak")?;
    let output: Output = Command::new(&executable)
        .args(["info", PRISM_FLATPAK_ID])
        .output()
        .ok()?;
    output.status.success().then_some(PrismCommand {
        executable,
        flatpak: true,
    })
}

pub fn build_launch_args(
    prism: &PrismCommand,
    instance_id: &str,
    data_root: &Path,
    server: Option<&str>,
) -> Vec<OsString> {
    let mut args = Vec::new();
    if prism.flatpak {
        args.push("run".into());
        args.push(PRISM_FLATPAK_ID.into());
    }
    args.push("--dir".into());
    args.push(data_root.as_os_str().to_owned());
    args.push("--launch".into());
    args.push(instance_id.into());
    if let Some(server) = server.filter(|address| !address.trim().is_empty()) {
        args.push("--server".into());
        args.push(server.trim().into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalizes_instances_directory_to_launcher_root() {
        assert_eq!(
            normalize_data_root(Path::new("/tmp/PrismLauncher/instances")),
            PathBuf::from("/tmp/PrismLauncher")
        );
    }

    #[test]
    fn reads_instance_name_version_and_mod_loader() {
        let root =
            std::env::temp_dir().join(format!("zephtor-launcher-test-{}", std::process::id()));
        let instance_dir = root.join("instances/Sky Factory");
        fs::create_dir_all(&instance_dir).unwrap();
        fs::write(
            instance_dir.join("instance.cfg"),
            "[General]\nname=Sky Factory 4\nMinecraftVersion=1.20.1\nFabricVersion=0.15.7\n",
        )
        .unwrap();

        let instances = find_instances(std::slice::from_ref(&root));
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "Sky Factory");
        assert_eq!(instances[0].name, "Sky Factory 4");
        assert_eq!(instances[0].minecraft_version, "1.20.1");
        assert_eq!(instances[0].loader, "Fabric");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_directories_without_instance_config() {
        let root = std::env::temp_dir().join(format!("zephtor-empty-test-{}", std::process::id()));
        fs::create_dir_all(root.join("instances/not-an-instance")).unwrap();
        assert!(find_instances(std::slice::from_ref(&root)).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn launches_native_prism_with_instance_and_server() {
        let prism = PrismCommand {
            executable: PathBuf::from("/usr/bin/prismlauncher"),
            flatpak: false,
        };
        let args = build_launch_args(
            &prism,
            "My World",
            Path::new("/home/player/Prism Data"),
            Some("play.example.net"),
        );
        assert_eq!(
            args,
            [
                "--dir",
                "/home/player/Prism Data",
                "--launch",
                "My World",
                "--server",
                "play.example.net"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn launches_flatpak_prism_without_using_a_shell() {
        let prism = PrismCommand {
            executable: PathBuf::from("/usr/bin/flatpak"),
            flatpak: true,
        };
        let args = build_launch_args(&prism, "Vanilla", Path::new("/tmp/Prism"), None);
        assert_eq!(
            args,
            [
                "run",
                PRISM_FLATPAK_ID,
                "--dir",
                "/tmp/Prism",
                "--launch",
                "Vanilla"
            ]
            .map(OsString::from)
        );
    }
}
