    const statusEl = document.getElementById('status');
    const activityEl = document.getElementById('activity');

    // Option: WebSocket (realtime) mit Fallback auf polling
    let ws;
    try {
      ws = new WebSocket((location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + location.host);
      ws.addEventListener('message', ev => {
        const s = JSON.parse(ev.data);
        statusEl.textContent = s.status + (s.online ? ' (online)' : '');
        activityEl.textContent = s.activity || '—';
      });
      ws.addEventListener('open', () => console.log('WS connected'));
      ws.addEventListener('error', () => { ws = null; startPolling(); });
    } catch (e) { startPolling(); }

    function startPolling() {
      async function fetchStatus() {
        try {
          const res = await fetch('/bot-status');
          const s = await res.json();
          statusEl.textContent = s.status + (s.online ? ' (online)' : '');
          activityEl.textContent = s.activity || '—';
        } catch (e) { statusEl.textContent = 'error'; }
      }
      fetchStatus();
      setInterval(fetchStatus, 15000);
    }


// Beim Empfang einer Nachricht (WebSocket) oder Polling-Antwort
ws.addEventListener('message', ev => {
  const s = JSON.parse(ev.data);
  updateBotStatus(s);
});