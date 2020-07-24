
import discord
import re
import asyncio

from mafiabot import *

with open("../.discord.token", 'r') as f:
  TOKEN = f.read().strip()

class DiscordHandler(MHandler):

  def raw_handle(self, message, client):
    if message.author == client.user:
      return

    content = message.content
    if content[0] == "/":
      command = content[1:].split()[0]
      print(f"command: {command}")

      kwargs = {}

      if command in self.REQUESTS:
        if command == "VOTE":
          if len(message.mentions) > 0:
            kwargs['votee_id'] = message.mentions[0].id
            kwargs['voter_id'] = message.author.id

        self.REQUESTS[command](**kwargs)


class DiscordComm:

  def __init__(self, client, channel):
    self.client = client
    self.channel = channel
    print("Creating Discord Comm!")

  def cast(self, msg, notarget="None"):
    msg = self.name_tags(msg, notarget)
    print(f"Casting! {msg}")
    self.client.loop.create_task(self.channel.send(msg))
  
  def send(self, msg, dest, notarget="None"):
    pass
    # msg = self.name_tags(msg, notarget)
    # member = self.channel.guild.get_member(int(dest))
    # self.client.loop.call_soon_threadsafe(self.send_process, member, msg)
    # print(f"Sending! {msg}")

  @staticmethod
  async def send_process(member,msg):
    await member.create_dm()
    await member.dm_channel.send(msg)

  def name_tags(self, msg, notarget):
    search_result = re.search(r'\[\d*\]', msg)
    while search_result:
      p_id_tag = search_result.group()
      p_id = int(p_id_tag[1:-1])
      msg.replace(p_id_tag, self.client.get_nickname(p_id, self.channel.guild.id))
    msg.replace(f'[{NOTARGET}]',notarget)
    return msg


class MDiscordClient(discord.Client):

  def get_nickname(self, p_id, g_id = None):
    try:
      guild_names = self.member_names[g_id]
    except KeyError:
      return "___"

    return guild_names[p_id]


  async def on_ready(self):

    self.member_names = {}

    print('{} has connected'.format(self.user))

    print('Guilds:')
    for guild in self.guilds:
      print('  {}(id: {})'.format(guild.name, guild.id))

      guild_names = {0:NOTARGET}
      for member in guild.members:
        guild_names[member.id] = member.name
        print('  - Collecting Display Name: {}|{}'.format(member.name,member.id))
      self.member_names[guild.id] = guild_names
    
    players = [n for n in range(0,5)]
    name_dict = {}
    for p in players:
      name_dict[str(p)] = chr(ord('A')+p)
    players = [str(p) for p in players]

    mrules = MRules()
    mrules['known_roles'] = "ROLE"
    mrules['reveal_on_death'] = "TEAM"
    mrules['know_if_saved_doc'] = "ON"
    mrules['know_if_saved_self'] = "ON"
    mrules['know_if_saved'] = "SAVED"
    mrules['cop_strength'] = "ROLE"

    mresp = MResp_Comm(
      DiscordComm(self,guild.text_channels[0]),
      TestMComm("MAFIA",name_dict),
      mrules,
    )

    m = MState.fromPlayers(
      players,
      mresp=mresp,
      mrules=mrules
    )

    self.handler = DiscordHandler()
    self.handler.mstate = m

  async def on_member_join(self, member):
    print(f'{member.name}|{member.id} joined server')
    await member.create_dm()
    await member.dm_channel.send(f'Hello, {member.name}, for commands, try /help')

  async def on_message(self, message):
    if message.author == self.user:
      return
    print(f'Message from {message.author.name} in {message.guild.name} (channel {message.channel.name}): {message.content}')
    self.handler.raw_handle(message, self)



def run_coroutine(f, *args, **kwargs):
    loop = asyncio.get_event_loop()
    result = loop.run_until_complete(f(*args, **kwargs))
    loop.close()
    return result

if __name__ == "__main__":
  client = MDiscordClient()
  try:
    client.run(TOKEN)
  except KeyboardInterrupt:
    pass
  finally:
    client.close()
