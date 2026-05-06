const ws = new WebSocket('wss://your-domain.com:8080');
ws.addEventListener('message', ev => {
  const status = JSON.parse(ev.data);
  document.getElementById('bot-status').textContent = status.status;
  document.getElementById('bot-activity').textContent = status.activity || '—';
});
