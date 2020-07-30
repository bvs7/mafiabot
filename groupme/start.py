#!/usr/bin/python3
from mafiabot import MController
from GroupMeChat import GroupMeChat, GroupMeDM
from GroupMeServer import GroupMeServer

ctrl = MController(GroupMeChat, GroupMeDM, GroupMeServer)
