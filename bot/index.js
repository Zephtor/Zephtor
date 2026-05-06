require('dotenv').config();
const fs = require('fs');
const path = require('path');
const express = require('express');
const http = require('http');
const WebSocket = require('ws');
const { REST } = require('@discordjs/rest');
const { Routes } = require('discord-api-types/v10');
const { Client, Collection, GatewayIntentBits, Partials } = require('discord.js');

const TOKEN = process.env.DISCORD_TOKEN;
const CLIENT_ID = process.env.CLIENT_ID;
const GUILD_ID = process.env.GUILD_ID;
const PORT = process.env.PORT || 3000;

if (!TOKEN || !CLIENT_ID) {
  console.error('DISCORD_TOKEN und CLIENT_ID in .env erforderlich.');
  process.exit(1);
}

const app = express();
const server = http.createServer(app);
const wss = new WebSocket.Server({ server });

// Pfad zur Website-Datei (relativ zu Zephtor/bot)
//const websiteHtml = path.join(__dirname, '..', 'website', 'pages', 'api.html');
const websiteHtml = path.join(__dirname, '..', 'index.html');

// Optionales Verzeichnis für zusätzliche statische Assets (z.B. ../website/public)
const websiteStaticDir = path.join(__dirname, '..', 'website', 'public');

// Statische Assets (falls vorhanden)
if (fs.existsSync(websiteStaticDir)) {
  app.use('/static', express.static(websiteStaticDir));
}

// Root liefert deine api.html
app.get('/', (req, res) => {
  if (fs.existsSync(websiteHtml)) {
    res.sendFile(websiteHtml);
  } else {
    res.status(404).send('Website file not found');
  }
});

// ---- Discord client ----
// Präsenz-Intent nur aktivieren, wenn du Presence-Daten brauchst
const intents = [GatewayIntentBits.Guilds, GatewayIntentBits.GuildMessages];
if (process.env.ENABLE_MESSAGE_CONTENT === 'true') intents.push(GatewayIntentBits.MessageContent);
if (process.env.ENABLE_PRESENCE === 'true') intents.push(GatewayIntentBits.GuildPresences);

const client = new Client({
  intents,
  partials: [Partials.Channel],
});

client.commands = new Collection();
const commandsPath = path.join(__dirname, 'commands');
if (fs.existsSync(commandsPath)) {
  const commandFiles = fs.readdirSync(commandsPath).filter(f => f.endsWith('.js'));
  for (const file of commandFiles) {
    const filePath = path.join(commandsPath, file);
    const command = require(filePath);
    if (command?.data && command?.execute) {
      client.commands.set(command.data.name, command);
    } else {
      console.warn(`Ungültiges Kommando: ${file}`);
    }
  }
}

// register slash commands (guild if GUILD_ID set)
(async () => {
  try {
    const slashData = client.commands.map(c => c.data);
    if (slashData.length === 0) return;
    const rest = new REST({ version: '10' }).setToken(TOKEN);
    if (GUILD_ID) {
      await rest.put(Routes.applicationGuildCommands(CLIENT_ID, GUILD_ID), { body: slashData });
      console.log('Guild commands registriert.');
    } else {
      await rest.put(Routes.applicationCommands(CLIENT_ID), { body: slashData });
      console.log('Globale commands registriert.');
    }
  } catch (err) {
    console.error('Fehler beim Registrieren der Commands:', err);
  }
})();

// ---- status state ----
let botStatus = {
  online: false,
  status: 'offline',
  activity: null,
  updatedAt: new Date().toISOString()
};

function refreshStatus() {
  if (!client || !client.user) return;
  botStatus.online = client.ws.status === 0;
  botStatus.status = client.presence?.status || (botStatus.online ? 'online' : 'offline');
  const act = client.presence?.activities?.[0];
  botStatus.activity = act ? act.name : null;
  botStatus.updatedAt = new Date().toISOString();
}

// Discord events
client.once('ready', () => {
  refreshStatus();
  console.log(`Logged in as ${client.user.tag}`);
});

client.on('presenceUpdate', () => {
  refreshStatus();
  broadcastStatus();
});

// interaction handling (slash commands)
client.on('interactionCreate', async interaction => {
  if (!interaction.isChatInputCommand()) return;
  const command = client.commands.get(interaction.commandName);
  if (!command) return;
  try {
    await command.execute(interaction);
  } catch (err) {
    console.error('Command error:', err);
    if (interaction.replied || interaction.deferred) {
      await interaction.followUp({ content: 'Es ist ein Fehler aufgetreten.', ephemeral: true });
    } else {
      await interaction.reply({ content: 'Es ist ein Fehler aufgetreten.', ephemeral: true });
    }
  }
});

// ---- HTTP endpoints ----
app.get('/bot-status', (req, res) => {
  refreshStatus();
  res.json(botStatus);
});

// WebSocket: send status on connect and when updated
function broadcastStatus() {
  refreshStatus();
  const payload = JSON.stringify(botStatus);
  wss.clients.forEach(ws => {
    if (ws.readyState === WebSocket.OPEN) ws.send(payload);
  });
}
wss.on('connection', ws => {
  refreshStatus();
  ws.send(JSON.stringify(botStatus));
});

// start server + discord
server.listen(PORT, () => console.log(`Webserver läuft auf http://localhost:${PORT}`));
client.login(TOKEN).catch(err => console.error('Login fehlgeschlagen:', err));
