const { SlashCommandBuilder } = require('discord.js');

module.exports = {
  data: new SlashCommandBuilder()
    .setName('ping')
    .setDescription('Antwortet mit Pong und Latenz.'),
  async execute(interaction) {
    const sent = await interaction.reply({ content: 'Pinge...', fetchReply: true });
    const latency = sent.createdTimestamp - interaction.createdTimestamp;
    await interaction.editReply(`Pong! Client-WS: ${Math.round(interaction.client.ws.ping)}ms | Roundtrip: ${latency}ms`);
  },
};
