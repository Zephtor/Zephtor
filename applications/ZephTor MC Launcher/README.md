# ZephTor MC Launcher

Ein leichter Linux-Desktop-Launcher in Rust mit nativer GTK-4-Oberfläche. PrismLauncher ist **keine Build-Abhängigkeit** und soll nur eine optionale Import-/Kompatibilitätsquelle sein.

> **Aktueller Entwicklungsstand:** Der Standalone-Launcher lädt offizielle Versionen, Bibliotheken, Linux-Natives und Assets, unterstützt Microsoft-Geräteanmeldung und kann Vanilla-Versionen direkt über Java starten. Prism bleibt nur eine optionale Import-/Kompatibilitätsfunktion.

## Voraussetzungen

- Rust/Cargo zum Bauen
- GTK 4-Entwicklungsbibliotheken
- Java-Runtime passend zur Minecraft-Version
- PrismLauncher als Systeminstallation oder Flatpak nur für die optionale Kompatibilität
- Für tatsächliches Minecraft-Spielen ein gültiges Minecraft-Konto

Der Standalone-Modus verwaltet Vanilla-Dateien selbst und speichert Microsoft-Zugriffstoken nicht dauerhaft. Der optionale Prism-Kompatibilitätsmodus delegiert Modloader und importierte Instanzen an PrismLauncher.

## Entwickeln und starten

```sh
cargo run
```

Die Standarddatenordner von PrismLauncher und MultiMC werden für den optionalen Import automatisch durchsucht. Mit **Prism-Ordner importieren** lässt sich ein eigener Launcher-Datenordner auswählen.

Die eigenständige Versionsbasis lässt sich über die CLI prüfen und verwenden:

```sh
cargo run -- --list-versions
cargo run -- --install-version 1.21.11
cargo run -- --launch-version 1.21.11
```

Die Version wird im Flatpak unter `~/.var/app/org.zephtor.MCLauncher/data/zephtor-mc-launcher/game` abgelegt; außerhalb des Flatpaks unter `~/.local/share/zephtor-mc-launcher/game`. Offizielle SHA-1-Prüfsummen werden geprüft. Java muss auf dem Host installiert sein; trage den Java-Pfad (im Flatpak typischerweise `/usr/lib/jvm/.../bin/java`) in `launcher.env` ein. Für die offizielle Microsoft-Anmeldung benötigst du eine öffentliche OAuth-App-ID mit aktivierter Gerätecode-Anmeldung. Hinterlege sie in **Microsoft-Anmeldung** oder in `launcher.env` (Vorlage: `launcher-config.example`). Zugriffstoken werden nicht dauerhaft gespeichert.

## Flatpak

```sh
./build-flatpak.sh
flatpak install --user dist/ZephTor-MC-Launcher.flatpak
```

Das Bundle liegt danach unter `dist/ZephTor-MC-Launcher.flatpak`. Es verwendet die Freedesktop-26.08-Laufzeit; Java wird bewusst nicht mitgebündelt, damit die App klein bleibt.

## Tests

```sh
cargo test
```

## Desktop-Menü

```sh
cargo build --release
./target/release/zephtor-mc-launcher --install-desktop
```

Der Befehl kopiert den Launcher nach `~/.local/bin` und legt einen Eintrag im Anwendungsmenü an.

## Funktionen

- PrismLauncher-Instanzen optional mit Minecraft-Version und Modloader importieren
- Importierte Instanzen derzeit über PrismLauncher starten
- Eigenständige Mojang-Versionsliste und Downloads von Client, Bibliotheken, Linux-Natives und Assets
- Minecraft-Release-Installation über die Oberfläche sowie CLI
- Eigenständiger Java-Start und Microsoft-/Xbox-/Minecraft-Geräteanmeldung
- Konfigurierbare öffentliche Microsoft OAuth-App-ID für den Gerätecode-Login
- Instanzordner im Dateimanager öffnen
- Eigener Launcher-Icon im Desktop-Menü
- Native und Flatpak-Installationen von PrismLauncher erkennen
- Eigenen Prism-Datenordner dauerhaft speichern
