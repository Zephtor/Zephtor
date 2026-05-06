const WebSocket = require('ws');
const wss = new WebSocket.Server({ port: process.env.WS_PORT || 8080 });

function broadcastStatus() {
  if (!client || !client.user) return;
  const payload = {
    status: client.presence?.status || 'online',
    activity: client.presence?.activities?.[0]?.name || null,
    updatedAt: new Date().toISOString()
  };
  const msg = JSON.stringify(payload);
  wss.clients.forEach(sock => { if (sock.readyState === WebSocket.OPEN) sock.send(msg); });
}

client.on('presenceUpdate', broadcastStatus);
client.once('ready', () => {
  broadcastStatus();
});
