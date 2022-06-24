
import discord
from discord.ext import commands

import logging

logging.basicConfig(level=logging.INFO)

from .MafiaState import *
from .MafiaEvent import EventHandler

NOTARGET_NAME = "peace"

VOTE_MSG = "{voter_name} votes for {votee_name}: {count}/{thresh}"

class TestBot:

    def get_channel(self, id):
        class Member:
            def __init__(self, id):
                self.id = id
                self.nick = f"Name of {id}"

        class Channel:
            def __init__(self, id):
                self.id=id
                self.members = [Member(1), Member(2), Member(3)]
            def send(self, msg):
                logging.info(f"ID:{id}, {msg}")
        return Channel(id)

class DiscordResponse(EventHandler):

    def __init__(self, bot:commands.Bot):
        self.bot : commands.Bot = bot

    def getChannel(self, chat:ChatHandle):
        return self.bot.get_channel(chat.id)

    def VOTE(self, m:GameState=None, voter:PlayerID=PlayerID.NOTARGET, 
            votee:PlayerID=PlayerID.NOTARGET, thresh=0, count=0):
        channel : discord.TextChannel = self.getChannel(m.main_chat)
        voter_member = [m for m in channel.members if m.id == voter][0]
        voter_name = voter_member.nick
        # Collect number of votes for each candidate?

        n = len(m.players)
        if votee == PlayerID.NOTARGET:
            votee_name = "peace"
        else:
            votee_member = [m for m in channel.members if m.id == votee][0]
            votee_name = votee_member.nick
        msg = VOTE_MSG.format(**locals())
        channel.send(msg)

    # TARGET response not needed? Handled by callback

    # MAFIA_TARGET

    # REVEAL

        
        