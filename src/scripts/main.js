const PROJECTS = [
    {
        id: 'zephtor-craft',
        title: 'ZephTor Craft Ultra',
        category: 'minecraft',
        categoryLabel: 'Minecraft Modpack',
        icon: '🧱',
        image: './src/assets/showcase/zephcraft.svg',
        description: 'Ein leistungsoptimiertes Quest-Modpack mit Technik, Erkundung und angepasster deutscher Übersetzung.',
        version: 'v2.4.0',
        tags: ['Minecraft', 'Modpack', 'Quest', 'Performance'],
        features: ['Quest-basierter Fortschritt', 'Für Forge, Fabric und NeoForge konzipiert', 'Optimierte Einstellungen für bessere Performance'],
        command: 'prism-launcher --import zephtor-craft-v2.4.mrpack',
        resources: [{ label: 'Auf CurseForge suchen', url: 'https://www.curseforge.com/minecraft/search?search=ZephTor' }]
    },
    {
        id: 'zephguard-bot',
        title: 'ZephGuard Bot Core',
        category: 'discord',
        categoryLabel: 'Discord Bot',
        icon: '◈',
        image: './src/assets/showcase/zephguard.svg',
        description: 'Ein modularer Discord-Bot für Moderation, Community-Verwaltung und Server-Automatisierung.',
        version: 'v4.0.1',
        tags: ['Discord', 'Moderation', 'Bot', 'Community'],
        features: ['Moderations- und Logging-Werkzeuge', 'Anpassbare Community-Funktionen', 'Für Discord-Server entwickelt'],
        resources: [{ label: 'Bot-Showcase ansehen', url: './src/pages/bots.html' }]
    },
    {
        id: 'anipager',
        title: 'AniPager',
        category: 'flatpak',
        categoryLabel: 'Anime-Bibliothek',
        icon: 'A',
        image: './src/assets/showcase/anipager.svg',
        description: 'Entdecke Anime über AniList, bewerte Serien und verwalte deine Watchlist. Persönliche Daten bleiben lokal.',
        version: 'v0.1.0',
        tags: ['Anime', 'AniList', 'Watchlist', 'Linux'],
        features: ['Anime suchen und entdecken', 'Serien mit Sternen bewerten', 'Persönliche Watchlist lokal verwalten'],
        resources: [{ label: 'AniPager (Flatpak) direkt herunterladen', url: './src/assets/downloads/AniPager.flatpak', download: true }]
    },
    {
        id: 'aod-studio',
        title: 'AOD Studio',
        category: 'flatpak',
        categoryLabel: 'Always-on Display Editor',
        icon: 'AOD',
        image: './src/assets/showcase/aod-studio.svg',
        description: 'Gestalte ein persönliches Always-on Display mit HTML, CSS und JavaScript und prüfe Änderungen in der Live-Vorschau.',
        version: 'v0.1.0',
        tags: ['Always-on Display', 'HTML', 'CSS', 'JavaScript'],
        features: ['HTML, CSS und JavaScript bearbeiten', 'Änderungen in der Live-Vorschau prüfen', 'Eigenes Always-on Display gestalten'],
        resources: [
            { label: 'AOD Studio (Flatpak) direkt herunterladen', url: './src/assets/downloads/io.github.Zephtor.AOD-0.1.0.flatpak', download: true },
            { label: 'AOD Studio auf GitHub', url: 'https://github.com/Zephtor/AOD' }
        ]
    }
];

let activeCategory = 'all';
let activeProject = null;
let toastTimer;

const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (character) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
}[character]));

function findProject(id) {
    return PROJECTS.find((project) => project.id === id);
}

function renderProjects() {
    const grid = document.getElementById('projectsGrid');
    const query = document.getElementById('searchInput').value.trim().toLocaleLowerCase('de');
    const visibleProjects = PROJECTS.filter((project) => {
        const categoryMatches = activeCategory === 'all' || project.category === activeCategory;
        const searchable = [project.title, project.categoryLabel, project.description, ...project.tags]
            .join(' ')
            .toLocaleLowerCase('de');
        return categoryMatches && searchable.includes(query);
    });

    grid.innerHTML = visibleProjects.map((project) => `
        <article class="project-card">
            <div class="project-image">
                <img src="${escapeHtml(project.image)}" alt="${escapeHtml(project.title)} – Projektvorschau" loading="lazy">
                <span class="image-category">${escapeHtml(project.categoryLabel)}</span>
            </div>
            <div class="project-content">
                <div class="project-title-row">
                    <h3>${escapeHtml(project.title)}</h3>
                    <span class="project-version">${escapeHtml(project.version)}</span>
                </div>
                <p>${escapeHtml(project.description)}</p>
                <div class="tag-list">${project.tags.map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join('')}</div>
                <div class="card-actions">
                    <button class="button quiet-button" type="button" data-open-project="${escapeHtml(project.id)}">Details</button>
                    <button class="button" type="button" data-open-resources="${escapeHtml(project.id)}">Projekt ansehen ↗</button>
                </div>
            </div>
        </article>
    `).join('');

    if (visibleProjects.length === 0) {
        grid.innerHTML = '<p class="empty-state">Keine Projekte gefunden. Ändere den Suchbegriff oder setze den Filter zurück.</p>';
    }
}

function filterShowcase(category) {
    activeCategory = ['all', 'minecraft', 'discord', 'flatpak'].includes(category) ? category : 'all';
    document.querySelectorAll('.filter-btn').forEach((button) => {
        const selected = button.id === `tab-${activeCategory}`;
        button.classList.toggle('is-active', selected);
        button.setAttribute('aria-pressed', String(selected));
    });
    renderProjects();
    document.getElementById('projekte')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function setModalOpen(id, open) {
    const modal = document.getElementById(id);
    if (!modal) return;
    modal.hidden = !open;
    document.body.classList.toggle('is-locked', open || Boolean(document.querySelector('.modal:not([hidden])')));
}

function openProjectDetails(id) {
    const project = findProject(id);
    if (!project) return;
    activeProject = project;
    document.getElementById('modalTitle').textContent = project.title;
    document.getElementById('modalCategory').textContent = project.categoryLabel;
    document.getElementById('modalDescription').textContent = project.description;
    document.getElementById('modalFeatures').innerHTML = project.features
        .map((feature) => `<li>${escapeHtml(feature)}</li>`)
        .join('');
    const commandBox = document.getElementById('modalCommandBox');
    commandBox.hidden = !project.command;
    document.getElementById('modalCommandText').textContent = project.command || '';
    setModalOpen('detailModal', true);
    document.querySelector('#detailModal .modal-close').focus();
}

function openProjectResources(id) {
    const project = findProject(id);
    if (!project) return;
    activeProject = project;
    document.getElementById('resourceTitle').textContent = project.title;
    document.getElementById('resourceDescription').textContent = project.command
        ? 'Hier findest du weiterführende Projektseiten und den Installationshinweis.'
        : 'Öffne die Projektseite oder lade die Datei direkt von dieser Website herunter.';
    const links = [
        ...project.resources.map((resource) => `<a class="resource-link" href="${escapeHtml(resource.url)}" ${resource.url.startsWith('http') ? 'target="_blank" rel="noopener noreferrer"' : ''} ${resource.download ? `download="${escapeHtml(resource.url.split('/').pop())}"` : ''}>${escapeHtml(resource.label)} ${resource.download ? '↓' : '↗'}</a>`),
        ...(project.command ? [`<button class="resource-link" type="button" data-copy-resource="${escapeHtml(project.id)}">Installationsbefehl kopieren</button>`] : [])
    ];
    document.getElementById('resourceLinks').innerHTML = links.join('');
    setModalOpen('resourcesModal', true);
    document.querySelector('#resourcesModal .modal-close').focus();
}

function closeModal(id) {
    setModalOpen(id, false);
}

function showToast(title, message) {
    const toast = document.getElementById('siteToast');
    document.getElementById('toastTitle').textContent = title;
    document.getElementById('toastMessage').textContent = message;
    toast.hidden = false;
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => { toast.hidden = true; }, 3200);
}

async function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
        try {
            await navigator.clipboard.writeText(text);
            showToast('Kopiert', 'Der Text liegt jetzt in deiner Zwischenablage.');
            return;
        } catch {
            // Use the browser fallback when clipboard permission is unavailable.
        }
    }
    const field = document.createElement('textarea');
    field.value = text;
    field.setAttribute('readonly', '');
    field.style.position = 'fixed';
    field.style.opacity = '0';
    document.body.appendChild(field);
    field.select();
    const copied = document.execCommand('copy');
    field.remove();
    if (!copied) throw new Error('Die Zwischenablage ist in diesem Browser nicht verfügbar.');
    showToast('Kopiert', 'Der Text liegt jetzt in deiner Zwischenablage.');
}

function appendTerminalOutput(message, kind = '') {
    const output = document.getElementById('terminalOutput');
    const line = document.createElement('p');
    if (kind) line.className = `terminal-${kind}`;
    line.textContent = message;
    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

function handleTerminalCommand(event) {
    event.preventDefault();
    const input = document.getElementById('terminalInput');
    const command = input.value.trim().toLowerCase();
    if (!command) return;
    appendTerminalOutput(`$ ${command}`);
    input.value = '';

    const commands = {
        help: () => appendTerminalOutput('Befehle: help · status · modpacks · bots · linux · clear', 'good'),
        status: () => appendTerminalOutput(`${PROJECTS.length} Projekte im Katalog. Die lokale Website ist bereit.`, 'good'),
        modpacks: () => appendTerminalOutput(PROJECTS.filter((project) => project.category === 'minecraft').map((project) => project.title).join(', ')),
        bots: () => appendTerminalOutput(PROJECTS.filter((project) => project.category === 'discord').map((project) => project.title).join(', ')),
        linux: () => appendTerminalOutput(PROJECTS.filter((project) => project.category === 'flatpak').map((project) => project.title).join(', ')),
        clear: () => { document.getElementById('terminalOutput').replaceChildren(); }
    };
    (commands[command] || (() => appendTerminalOutput(`Befehl nicht gefunden: ${command}. Tippe „help“.`, 'error')))();
}

function testSystemPing() {
    const online = navigator.onLine;
    document.getElementById('connectionStatus').textContent = online
        ? 'Browser meldet eine aktive Netzwerkverbindung'
        : 'Browser meldet keine Netzwerkverbindung';
    showToast(online ? 'Verbindung verfügbar' : 'Offline', online
        ? 'Dies prüft nur den Browserstatus, keine externen Server-Pings.'
        : 'Die Website kann gerade keine Netzwerkverbindung erkennen.');
}

async function runSystemDiagnostics() {
    const modal = document.getElementById('diagnosticsModal');
    const results = document.getElementById('diagResultsList');
    const spinner = document.getElementById('diagStatusSpinner');
    const status = document.getElementById('diagStatusText');
    const score = document.getElementById('diagScore');
    results.replaceChildren();
    spinner.hidden = false;
    status.textContent = 'Lokale Website-Funktionen werden geprüft …';
    score.textContent = '0/5 bestanden';
    setModalOpen('diagnosticsModal', true);

    const checks = [
        ['Projektkatalog', PROJECTS.length > 0],
        ['Projektübersicht', Boolean(document.getElementById('projectsGrid'))],
        ['Suche und Filter', Boolean(document.getElementById('searchInput') && document.querySelector('.filter-btn'))],
        ['Projektfenster', Boolean(document.getElementById('detailModal') && document.getElementById('resourcesModal'))],
        ['Browser-Netzwerkstatus', navigator.onLine]
    ];
    let passed = 0;
    for (const [label, success] of checks) {
        await new Promise((resolve) => window.setTimeout(resolve, 120));
        if (success) passed += 1;
        const row = document.createElement('li');
        const value = success ? 'Bereit' : label === 'Browser-Netzwerkstatus' ? 'Offline' : 'Fehlt';
        row.innerHTML = `<span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong>`;
        results.appendChild(row);
        score.textContent = `${passed}/5 bestanden`;
    }
    spinner.hidden = true;
    status.textContent = 'Prüfung abgeschlossen. Externe Dienste werden nicht simuliert.';
}

function initializePage() {
    renderProjects();
    document.getElementById('searchInput').addEventListener('input', renderProjects);
    document.getElementById('projectsGrid').addEventListener('click', (event) => {
        const details = event.target.closest('[data-open-project]');
        const resources = event.target.closest('[data-open-resources]');
        if (details) openProjectDetails(details.dataset.openProject);
        if (resources) openProjectResources(resources.dataset.openResources);
    });
    document.getElementById('resourceLinks').addEventListener('click', (event) => {
        const copyButton = event.target.closest('[data-copy-resource]');
        if (!copyButton) return;
        const project = findProject(copyButton.dataset.copyResource);
        if (project?.command) copyText(project.command).catch((error) => showToast('Kopieren fehlgeschlagen', error.message));
    });
    document.getElementById('copyCommandButton').addEventListener('click', () => {
        if (activeProject?.command) copyText(activeProject.command).catch((error) => showToast('Kopieren fehlgeschlagen', error.message));
    });
    document.getElementById('modalResourcesButton').addEventListener('click', () => {
        if (!activeProject) return;
        closeModal('detailModal');
        openProjectResources(activeProject.id);
    });
    document.querySelectorAll('[data-close]').forEach((button) => {
        button.addEventListener('click', () => closeModal(button.dataset.close));
    });
    document.querySelectorAll('.modal').forEach((modal) => {
        modal.addEventListener('click', (event) => {
            if (event.target === modal) closeModal(modal.id);
        });
    });
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            document.querySelectorAll('.modal:not([hidden])').forEach((modal) => closeModal(modal.id));
        }
    });
    document.getElementById('mobileMenuBtn').addEventListener('click', (event) => {
        const button = event.currentTarget;
        const menu = document.getElementById('mobileMenu');
        const open = button.getAttribute('aria-expanded') !== 'true';
        button.setAttribute('aria-expanded', String(open));
        menu.classList.toggle('is-open', open);
    });
    document.querySelectorAll('#mobileMenu a').forEach((link) => {
        link.addEventListener('click', () => {
            document.getElementById('mobileMenu').classList.remove('is-open');
            document.getElementById('mobileMenuBtn').setAttribute('aria-expanded', 'false');
        });
    });
    document.getElementById('terminalForm').addEventListener('submit', handleTerminalCommand);
    document.getElementById('pingTestBtn').addEventListener('click', testSystemPing);
    document.getElementById('statModules').textContent = String(PROJECTS.length);
}

window.filterShowcase = filterShowcase;
window.runSystemDiagnostics = runSystemDiagnostics;

document.addEventListener('DOMContentLoaded', initializePage, { once: true });
