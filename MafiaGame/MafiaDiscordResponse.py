
import discord
from discord.ext import commands

from .MafiaState import *
from .MafiaEvent import Event

NOTARGET_NAME = "peace"

VOTE_MSG = "{voter_name} votes for {votee_name}: {vote_count}/{thresh}"

class DiscordResponse(Event):

    def __init__(self, bot:commands.Bot):
        self.bot : commands.Bot = bot

    def getChannel(self, chat:ChatHandle):
        return self.bot.get_channel(chat.id)

    async def vote(self, m:GameState, voter:PlayerID, votee:PlayerID):
        channel : discord.TextChannel = self.getChannel(m.main_chat)
        voter_member = [m for m in channel.members if m.id == voter][0]
        voter_name = voter_member.nick
        # Collect number of votes for each candidate?
        vote_count = 0
        for (voter,v) in m.round.votes.items():
            if v == votee:
                vote_count += 1
        n = len(m.players)
        if votee == PlayerID.NOTARGET:
            thresh = (n+1) // 2
            votee_name = "peace"
        else:
            thresh = (n // 2) + 1
            votee_member = [m for m in channel.members if m.id == votee][0]
            votee_name = votee_member.nick
        msg = VOTE_MSG.format(**locals())
        channel.send(msg)
        super().vote(m, voter,votee)


        
        