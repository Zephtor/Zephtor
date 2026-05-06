// status.js
// Usage: include this script after the DOM, it will look for #dc_status_indicator
// It uses WebSocket to location.host with fallback to polling /bot-status.
(function(){
  const root = document.getElementById('dc_status_indicator');
  if (!root) return;

  // build DOM
  root.classList.add('dc-status-card');
  root.innerHTML = `
    <div class="dc-status-dot dc-offline" aria-hidden="true"></div>
    <div style="display:flex;flex-direction:column">
      <span class="dc-status-label">lade…</span>
      <span class="dc-status-meta">Letztes Update: —</span>
    </div>
  `;
  const dot = root.querySelector('.dc-status-dot');
  const label = root.querySelector('.dc-status-label');
  const meta = root.querySelector('.dc-status-meta');

  function applyStatus(obj){
    const isOnline = !!obj.online && obj.status !== 'offline';
    if (isOnline) {
      dot.classList.remove('dc-offline'); dot.classList.add('dc-online');
      label.textContent = 'Online';
    } else {
      dot.classList.remove('dc-online'); dot.classList.add('dc-offline');
      label.textContent = 'Offline';
    }
    meta.textContent = 'Letztes Update: ' + (obj.updatedAt ? new Date(obj.updatedAt).toLocaleString() : '—') + (obj.activity ? ' • ' + obj.activity : '');
  }

  // WebSocket with fallback
  let ws;
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  try {
    ws = new WebSocket(protocol + '//' + location.host);
    ws.addEventListener('message', ev => {
      try { const s = JSON.parse(ev.data); applyStatus(s); } catch(e){}
    });
    ws.addEventListener('error', () => { ws.close(); ws = null; startPolling(); });
    ws.addEventListener('close', () => { if (!ws || ws.readyState !== WebSocket.OPEN) startPolling(); });
  } catch(e){ startPolling(); }

  function startPolling(){
    async function fetchStatus(){
      try {
        const res = await fetch('/bot-status', { cache: 'no-store' });
        if (!res.ok) throw new Error('fetch failed');
        const s = await res.json();
        applyStatus(s);
      } catch(e){
        applyStatus({ online:false, status:'offline', updatedAt: new Date().toISOString() });
      }
    }
    fetchStatus();
    setInterval(fetchStatus, 15000);
  }
})();
