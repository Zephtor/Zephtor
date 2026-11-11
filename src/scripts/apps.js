const APPS = [
    {
        id: 'anipager',
        title: 'AniPager',
        category: 'app',
        categoryLabel: 'Anime-Bibliothek',
        icon: 'A',
        tags: ['Anime', 'AniList', 'Watchlist', 'Bewertungen'],
        version: '0.1.0',
        releases: [{
            version: '0.1.0',
            date: '05.10.2026',
            notes: ['Erste öffentliche Version von AniPager.'],
            files: [{
                filename: 'AniPager.flatpak',
                path: '../assets/downloads/AniPager.flatpak',
                label: 'AniPager für Linux (Flatpak)',
                size: '77,4 MB',
                sha256: '8960d66f3312b6406c59527e437f5dd86333fa89fa53f8f8526ebbfb47cfe788'
            }]
        }],
        platform: 'Linux · x86_64',
        description: 'Entdecke Anime über AniList, bewerte Serien und verwalte deine Watchlist. Persönliche Daten bleiben lokal auf deinem Gerät.',
        banner: '../assets/showcase/anipager.svg',
        featured: true
    },
    {
        id: 'aod-studio',
        title: 'AOD Studio',
        category: 'app',
        categoryLabel: 'Always-on Display Editor',
        icon: 'AOD',
        tags: ['Always-on Display', 'HTML', 'CSS', 'JavaScript'],
        version: '0.1.0',
        releases: [{
            version: '0.1.0',
            date: '03.09.2026',
            notes: ['Erste öffentliche Version von AOD Studio.'],
            files: [{
                filename: 'io.github.Zephtor.AOD-0.1.0.flatpak',
                path: '../assets/downloads/io.github.Zephtor.AOD-0.1.0.flatpak',
                label: 'AOD Studio für Linux (Flatpak)',
                size: '28,1 KB',
                sha256: 'fece55761b6c4b16f03a7873a10be7fb7024381b0071512f45225deab4eb1f32'
            }]
        }],
        platform: 'Linux · x86_64',
        description: 'Gestalte ein persönliches Always-on Display mit HTML, CSS und JavaScript und prüfe Änderungen direkt in der Live-Vorschau.',
        banner: '../assets/showcase/aod-studio.svg',
        featured: true
    }
];

const featuredApps = APPS.filter((app) => app.featured);
let currentSort = 'featured';
let currentHeroIndex = 0;
let activeApp = null;
let activeRelease = null;
const activeWishlist = new Set();

const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (character) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
}[character]));

function findApp(id) {
    return APPS.find((app) => app.id === id);
}

function getReleases(app) {
    return [...(app.releases || [])].sort((a, b) => compareVersions(b.version, a.version));
}

function getLatestRelease(app) {
    return getReleases(app)[0] || { version: app.version, notes: [], files: [] };
}

function getVisibleApps() {
    const query = document.getElementById('appsSearch').value.trim().toLocaleLowerCase('de');
    const filtered = APPS.filter((app) => {
        const terms = [app.title, app.categoryLabel, app.description, app.platform, ...app.tags]
            .join(' ')
            .toLocaleLowerCase('de');
        return terms.includes(query);
    });

    if (currentSort === 'newest') return filtered.sort((a, b) => compareVersions(b.version, a.version));
    if (currentSort === 'name') return filtered.sort((a, b) => a.title.localeCompare(b.title, 'de'));
    return filtered.sort((a, b) => Number(Boolean(b.featured)) - Number(Boolean(a.featured)));
}

function compareVersions(first, second) {
    const a = first.split('.').map(Number);
    const b = second.split('.').map(Number);
    for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
        const difference = (a[index] || 0) - (b[index] || 0);
        if (difference) return difference;
    }
    return 0;
}

function renderApps() {
    const appsGrid = document.getElementById('appsGrid');
    const visibleApps = getVisibleApps();
    appsGrid.innerHTML = visibleApps.map((app) => `
        <article class="app-card">
            <div class="app-image">
                <img src="${escapeHtml(app.banner)}" alt="${escapeHtml(app.title)} – Vorschau der Benutzeroberfläche" loading="lazy">
                <span class="image-category">${escapeHtml(app.categoryLabel)}</span>
                <span class="app-icon" aria-hidden="true">${escapeHtml(app.icon)}</span>
            </div>
            <div class="app-content">
                <div class="app-title-row">
                    <h3>${escapeHtml(app.title)}</h3>
                    <button class="wishlist-button${activeWishlist.has(app.id) ? ' is-active' : ''}" type="button" data-wishlist="${escapeHtml(app.id)}" aria-label="${activeWishlist.has(app.id) ? 'Von Merkliste entfernen' : 'Auf Merkliste setzen'}">${activeWishlist.has(app.id) ? '♥' : '♡'}</button>
                </div>
                <div class="app-meta"><span>${escapeHtml(getLatestRelease(app).version)} · ${escapeHtml(app.platform)}</span><span class="app-rating">Download bereit</span></div>
                <p>${escapeHtml(app.description)}</p>
                <div class="tag-list">${app.tags.slice(0, 4).map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join('')}</div>
                <div class="card-actions">
                    <button class="button quiet-button" type="button" data-details="${escapeHtml(app.id)}">Details</button>
                    <button class="button" type="button" data-download="${escapeHtml(app.id)}">Datei herunterladen</button>
                </div>
            </div>
        </article>
    `).join('');
    document.getElementById('appsEmptyState').hidden = visibleApps.length > 0;
}

function renderHeroDots() {
    document.getElementById('appsHeroDots').innerHTML = featuredApps.map((app, index) => `
        <button class="carousel-dot${index === currentHeroIndex ? ' is-active' : ''}" type="button" data-hero-index="${index}" aria-label="${escapeHtml(app.title)}"${index === currentHeroIndex ? ' aria-current="true"' : ''}></button>
    `).join('');
}

function updateHero(index) {
    currentHeroIndex = index;
    const app = featuredApps[currentHeroIndex];
    document.getElementById('heroCategory').textContent = app.categoryLabel;
    document.getElementById('heroTitle').textContent = app.title;
    document.getElementById('heroRating').textContent = `${getLatestRelease(app).version} · ${app.platform}`;
    document.getElementById('heroDescription').textContent = app.description;
    document.getElementById('heroSpecs').innerHTML = app.tags.slice(0, 3)
        .map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`)
        .join('');
    const image = document.getElementById('heroImage');
    image.src = app.banner;
    image.alt = `${app.title} – Vorschau der Benutzeroberfläche`;
    document.getElementById('heroDownloadBtn').dataset.download = app.id;
    document.getElementById('heroDetailsBtn').dataset.details = app.id;
    renderHeroDots();
}

function setModalOpen(id, open) {
    const modal = document.getElementById(id);
    modal.hidden = !open;
    document.body.classList.toggle('is-locked', open || Boolean(document.querySelector('.modal:not([hidden])')));
}

function openAppDetails(id) {
    const app = findApp(id);
    if (!app) return;
    activeApp = app;
    document.getElementById('modalBanner').src = app.banner;
    document.getElementById('modalBanner').alt = `${app.title} – Projektmotiv`;
    document.getElementById('modalCategory').textContent = app.categoryLabel;
    document.getElementById('modalVersion').textContent = `${getLatestRelease(app).version} · Projektkatalog`;
    document.getElementById('modalTitle').textContent = app.title;
    document.getElementById('modalRating').textContent = 'Flatpak-Bundle · Download verfügbar';
    document.getElementById('modalPlatform').textContent = app.platform;
    document.getElementById('modalSize').textContent = 'Website-Download';
    document.getElementById('modalDescription').textContent = app.description;
    document.getElementById('modalTags').innerHTML = app.tags.map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join('');
    document.getElementById('modalWishlistBtn').textContent = activeWishlist.has(app.id) ? '♥ Gemerkt' : '♡ Merken';
    document.getElementById('modalDownloadBtn').dataset.download = app.id;
    setModalOpen('appModal', true);
    document.querySelector('#appModal [data-close-app]').focus();
}

function openDownloads(id) {
    const app = findApp(id);
    if (!app) return;
    activeApp = app;
    document.getElementById('downloadTitle').textContent = app.title;
    const releases = getReleases(app);
    const versionSelect = document.getElementById('releaseVersion');
    versionSelect.innerHTML = releases.map((release) => `
        <option value="${escapeHtml(release.version)}">${escapeHtml(release.version)}${release.version === getLatestRelease(app).version ? ' · neueste' : ''}</option>
    `).join('');
    activeRelease = releases[0] || getLatestRelease(app);
    renderSelectedRelease();
    document.getElementById('downloadDescription').textContent = 'Wähle die gewünschte Version und lade die Installationsdatei direkt von dieser Website herunter.';
    setModalOpen('downloadModal', true);
    document.querySelector('#downloadModal [data-close-download]').focus();
}

function renderSelectedRelease() {
    if (!activeApp || !activeRelease) return;
    const files = activeRelease.files || [];
    document.getElementById('selectedReleaseLabel').textContent = `Version ${activeRelease.version}${activeRelease.date ? ` · ${activeRelease.date}` : ''}`;
    document.getElementById('downloadFiles').innerHTML = files.map((file) => {
        const path = file.path || ['..', 'assets', 'downloads', activeApp.id, activeRelease.version, file.filename]
            .map((segment, index) => index < 3 ? segment : encodeURIComponent(segment))
            .join('/');
        const details = [file.size, file.sha256 ? `SHA-256 ${file.sha256}` : ''].filter(Boolean).join(' · ');
        return `
            <div class="release-file">
                <a class="resource-link download-file-link" href="${path}" download="${escapeHtml(file.filename)}">${escapeHtml(file.label || file.filename)} herunterladen ↓</a>
                ${details ? `<small>${escapeHtml(details)}</small>` : ''}
            </div>
        `;
    }).join('');
    document.getElementById('downloadUnavailable').hidden = files.length > 0;
    document.getElementById('downloadUnavailable').textContent = files.length > 0
        ? ''
        : `Für Version ${activeRelease.version} ist noch keine Installationsdatei auf dieser Website hinterlegt.`;
    const notes = activeRelease.notes || [];
    document.getElementById('releaseNotes').innerHTML = notes.map((note) => `<li>${escapeHtml(note)}</li>`).join('');
    document.getElementById('releaseNotes').hidden = notes.length === 0;
}

function renderWishlist() {
    const container = document.getElementById('wishlistContainer');
    const apps = APPS.filter((app) => activeWishlist.has(app.id));
    container.innerHTML = apps.length
        ? apps.map((app) => `
            <article class="drawer-item">
                <img src="${escapeHtml(app.banner)}" alt="" loading="lazy">
                <div><strong>${escapeHtml(app.title)}</strong><small>${escapeHtml(app.categoryLabel)} · ${escapeHtml(getLatestRelease(app).version)}</small></div>
                <button class="button quiet-button" type="button" data-download="${escapeHtml(app.id)}">Datei ↗</button>
            </article>
        `).join('')
        : '<p class="empty-state">Deine Merkliste ist noch leer.</p>';
}

function toggleWishlist(id) {
    if (!findApp(id)) return;
    if (activeWishlist.has(id)) activeWishlist.delete(id);
    else activeWishlist.add(id);
    renderApps();
    renderWishlist();
    if (activeApp?.id === id && !document.getElementById('appModal').hidden) {
        document.getElementById('modalWishlistBtn').textContent = activeWishlist.has(id) ? '♥ Gemerkt' : '♡ Merken';
    }
}

document.getElementById('appsGrid').addEventListener('click', (event) => {
    const target = event.target.closest('button');
    if (!target) return;
    if (target.dataset.details) openAppDetails(target.dataset.details);
    if (target.dataset.download) openDownloads(target.dataset.download);
    if (target.dataset.wishlist) toggleWishlist(target.dataset.wishlist);
});
document.getElementById('wishlistContainer').addEventListener('click', (event) => {
    const target = event.target.closest('[data-download]');
    if (!target) return;
    setModalOpen('wishlistModal', false);
    openDownloads(target.dataset.download);
});

document.getElementById('appsSearch').addEventListener('input', renderApps);
document.getElementById('appsSort').addEventListener('change', (event) => {
    currentSort = event.target.value;
    renderApps();
});
document.getElementById('releaseVersion').addEventListener('change', (event) => {
    if (!activeApp) return;
    activeRelease = getReleases(activeApp).find((release) => release.version === event.target.value);
    renderSelectedRelease();
});
document.getElementById('appsReset').addEventListener('click', () => {
    document.getElementById('appsSearch').value = '';
    renderApps();
});

document.getElementById('appsHeroDots').addEventListener('click', (event) => {
    const dot = event.target.closest('[data-hero-index]');
    if (dot) updateHero(Number(dot.dataset.heroIndex));
});
document.getElementById('heroDownloadBtn').addEventListener('click', (event) => openDownloads(event.currentTarget.dataset.download));
document.getElementById('heroDetailsBtn').addEventListener('click', (event) => openAppDetails(event.currentTarget.dataset.details));
document.getElementById('modalDownloadBtn').addEventListener('click', (event) => {
    setModalOpen('appModal', false);
    openDownloads(event.currentTarget.dataset.download);
});
document.getElementById('modalWishlistBtn').addEventListener('click', () => {
    if (activeApp) toggleWishlist(activeApp.id);
});
document.getElementById('openWishlistBtn').addEventListener('click', () => {
    renderWishlist();
    setModalOpen('wishlistModal', true);
    document.querySelector('#wishlistModal [data-close-wishlist]').focus();
});

document.querySelectorAll('[data-close-app], [data-close-wishlist], [data-close-download]').forEach((button) => {
    button.addEventListener('click', () => {
        const modal = button.closest('.modal');
        setModalOpen(modal.id, false);
    });
});
document.querySelectorAll('.modal').forEach((modal) => {
    modal.addEventListener('click', (event) => {
        if (event.target === modal) setModalOpen(modal.id, false);
    });
});
document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    document.querySelectorAll('.modal:not([hidden])').forEach((modal) => setModalOpen(modal.id, false));
});

document.getElementById('appsReset').setAttribute('aria-label', 'Suche und Kategoriefilter zurücksetzen');
updateHero(0);
renderApps();
