
import discord
from discord.ext import commands

import sqlite3

from typing import List

import logging
logging.basicConfig(level=logging.DEBUG)

from . import *


intents = discord.Intents().default()
intents.members = True
bot = commands.Bot(command_prefix="!", intents=intents)

DATA_FOLDER = "../data/"
DB_FNAME = DATA_FOLDER + "mafiabot.db"

LOBBY_FNAME = "/info.maflob"

## Database tables:
CHAT_TABLE = "chats"
LOBBY_TABLE = "lobbies"

MAFIABOT_CHAT_CATEGORY = discord.CategoryChannel()

NOKILL_STR = "peace"
SELFVOTE_STR = "me"

class MafiaUtil:

    @staticmethod
    async def isLobby(channel_id):
        con = sqlite3.connect(DB_FNAME)
        cur = con.cursor()
        resp = cur.execute(
            f"SELECT * FROM {LOBBY_TABLE} where lobby_id = {channel_id}")
        con.close()
        return len(resp) > 0

    @staticmethod
    async def getLobby(channel_id):
        if not MafiaUtil.isLobby(channel_id):
            raise ValueError(f"No lobby with channel_id {channel_id}")
        return MafiaLobby.load(DATA_FOLDER + str(channel_id) + LOBBY_FNAME)

    @staticmethod
    async def getChatInfo(channel_id):
        con = sqlite3.connect(DB_FNAME)
        cur = con.cursor()
        resp = cur.execute(
            f"SELECT * FROM {CHAT_TABLE} where chat_id = {channel_id}")
        con.close()
        if not len(resp > 0):
            raise ValueError(f"No chat with channel_id {channel_id}")
        return resp[0] # lobby-id, game_number, "MAIN"|"MAFIA"

    @staticmethod
    async def retrieveGame(lobby_id, game_number):
        fname = f"{DATA_FOLDER}{lobby_id}/Game{game_number}.maf"
        return GameState.load(fname)

    @staticmethod
    async def getNextGameNo(lobby_chat:ChatHandle) -> int:
        con = sqlite3.connect(DB_FNAME)
        cur = con.cursor()
        resp = cur.execute(
            f"SELECT game_number FROM {CHAT_TABLE} WHERE lobby_id = {lobby_chat.id} ORDER BY game_number DESC LIMIT 1"
        )
        if not len(resp >0):
            return 1
        return int(resp[0][0])

    @staticmethod
    async def createTextChannel(guild:discord.Guild, name:str, category:discord.CategoryChannel=None):
        private_channel = {
            guild.default_role: discord.PermissionOverwrite(read_messages=False),
            guild.me: discord.PermissionOverwrite(read_messages=True)}
        return await guild.create_text_channel(name, overwrites=private_channel, )

    @staticmethod
    async def getMentionID(message:discord.Message):
        for user in message.mentions:
            if not isinstance(user, discord.Member):
                continue
            return user.id


class MafiaBotLobby(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

        # TODO open current games
        
    # TODO: Add listener for member update state, to check games?
    

    @commands.command
    async def found_lobby(self, ctx):
        """Make this chat into a MafiaBot lobby.
        
        New games can be created from this chat.
        Does not work if this is a main chat or mafia chat"""
        pass

    @commands.command
    async def start(self, ctx:commands.Context):
        channel = ctx.channel
        if not isinstance(channel, discord.TextChannel):
            return
        try:
            lobby = await MafiaUtil.getLobby(channel.id)
        except ValueError as ve:
            logging.error(str(ve))
            return

        # TODO: get list of player IDs
        player_ids : List[PlayerID] = []
        role_list = roleGen(player_ids)
        game_number = await MafiaUtil.getNextGameNo(lobby.chat)

        game = await MafiaController.start(lobby.chat, game_number, role_list, lobby.rules)

        mafiabot_category = channel.category
        main_channel = await MafiaUtil.createTextChannel(
            ctx.guild, f"main-chat-{game_number}", mafiabot_category)
        mafia_channel = await MafiaUtil.createTextChannel(
            ctx.guild, f"mafia-chat-{game_number}", mafiabot_category)
        game.main_chat = ChatHandle(main_channel.id)
        game.mafia_chat = ChatHandle(mafia_channel.id)

class MafiaBotMain(commands.Cog):
    # TODO: How to find the game?
    # Have a database with channel_id -> game/lobby stuff

    def __init__(self, bot):
        self.bot = bot

    async def cog_command_error(self, ctx : commands.Context, error):
        if isinstance(error, MafiaFormatError):
            ctx.send(str(error))
        return await super().cog_command_error(ctx, error)

    @commands.command
    async def vote(self, ctx : commands.Context, votee : str):
        if not isinstance(ctx.channel, discord.TextChannel):
            return
        lobby_id, game_number, chat_type = await MafiaUtil.getChatInfo(ctx.channel.id)
        if not chat_type == "MAIN":
            raise MafiaFormatError("Can only vote in main-chat")
        game = await MafiaUtil.retrieveGame(lobby_id, game_number)
        if votee == NOKILL_STR:
            votee = PlayerID.NOTARGET
        elif votee == SELFVOTE_STR:
            votee = PlayerID(ctx.author.id)
        else:
            votee = MafiaUtil.getMentionID(ctx.message)
            if not votee:
                raise MafiaFormatError("Invalid vote target")
        

