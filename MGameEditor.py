import tkinter as tk
from tkinter import ttk

import json

import mafiabot

ALL_ROLES = mafiabot.MInfo.ALL_ROLES

TARGETING_ROLES = mafiabot.MInfo.TARGETING_ROLES

class MGameEditor:

    def __init__(self, mgame : mafiabot.MGame):
        self.mgame = mgame
        self.root = tk.Tk()

        self.root.title("MGame Editor")

        self.player_ids = list(range(5))

        tabControl = ttk.Notebook(self.root)

        tab_overview = ttk.Frame(tabControl)

        tab_overview_status = ttk.Frame(tab_overview)

        id_text = ttk.Label(tab_overview_status, text="id:")
        id_text.pack(side=tk.LEFT)
        id_entry = ttk.Entry(tab_overview_status, justify=tk.CENTER, width=3)
        id_entry.insert(0, self.mgame.state.id) # INPUT
        self.get_id = id_entry.get
        id_entry.pack(side=tk.LEFT)

        day_text = ttk.Label(tab_overview_status, text = "Day:")
        day_text.pack(side=tk.LEFT)
        day_spinbox = ttk.Spinbox(tab_overview_status, from_=0, to_=100,width=3)
        day_spinbox.set(self.mgame.state.day) # INPUT
        self.get_day = day_spinbox.get
        day_spinbox.pack(side=tk.LEFT)

        phase_text = ttk.Label(tab_overview_status, text="Phase:")
        phase_text.pack(side=tk.LEFT)
        phase_combobox = ttk.Combobox(tab_overview_status, values = ("DAY","NIGHT","DUSK"), width = 6)
        phase_combobox.set(self.mgame.state.phase.name) # INPUT
        self.get_phase = phase_combobox.get
        phase_combobox.pack(side=tk.LEFT)

        tab_overview_status.pack(side=tk.TOP)

        tab_overview_players = tk.Frame(tab_overview)

        players_label = tk.Label(tab_overview_players, text="Players:",bg="white")
        players_label.pack(side=tk.TOP, anchor=tk.NW, fill=tk.X)
        players_frames = []
        self.get_players = {}
        # self.show_DAY=[]
        # self.show_NIGHT=[]
        player_ids = mgame.state.player_order
        all_ids = player_ids + ["NOTARGET"] + [None]
        for player in player_ids:
            pframe = tk.Frame(tab_overview_players)
            player_name = ttk.Label(pframe,text="[NAME{}]".format(player))
            player_name.pack(side=tk.LEFT)
            player_id = ttk.Label(pframe,text="id:{}".format(player))
            player_id.pack(side=tk.LEFT)
            player_role = ttk.Combobox(pframe, values = ALL_ROLES, width = max([len(r) for r in ALL_ROLES])+3)

            p = self.mgame.state.players[player]
            role = p.role
            player_role.set(role) # input
            get_dict = {}
            get_dict['role'] = player_role.get
            player_role.pack(side=tk.LEFT)
            players_frames.append(pframe)

            player_votelabel = ttk.Label(pframe, text="Vote:")
            player_votelabel.pack(side=tk.LEFT)
            player_vote = ttk.Combobox(pframe, values=all_ids, width = max([len(str(r)) for r in all_ids])+1)
            if(p.vote == None):
                player_vote.set("None")
            else:
                player_vote.set(p.vote)
            get_dict['vote'] = player_vote.get

            # player_vote.pack_forget()
            # if self.mgame.state.phase == mafiabot.MPhase.DAY:
            player_vote.pack(side=tk.LEFT)

            player_targetlabel = ttk.Label(pframe, text="Target:")
            player_targetlabel.pack(side=tk.LEFT)
            player_target = ttk.Combobox(pframe, values=all_ids, width = max([len(str(r)) for r in all_ids])+1)
            if p.target == None:
                player_target.set("None")
            else:
                player_target.set(p.target)

            get_dict['target'] = player_target.get

            # player_target.pack_forget()
            # if self.mgame.state.phase == mafiabot.MPhase.NIGHT:
            #     if role in TARGETING_ROLES:
            player_target.pack(side=tk.LEFT)

            self.get_players[player] = get_dict

            # self.show_DAY.append( lambda: player_vote.pack(tk.LEFT))
            # self.show_DAY.append( player_target.pack_forget)

            # self.show_NIGHT.append(player_vote.pack_forget)
            # self.show_NIGHT.append( lambda: player_target.pack(tk.LEFT) )

        for pframe in players_frames:
            pframe.pack(side=tk.TOP, anchor=tk.NW) 

        tab_overview_players.pack(side=tk.TOP, anchor=tk.NW)

        tab_overview_update = ttk.Frame(tab_overview)

        update_button = ttk.Button(tab_overview_update, text="Update", command=self.update)
        update_button.pack(side=tk.RIGHT)

        save_button = ttk.Button(tab_overview_update, text="Save", command=self.save)
        save_button.pack(side=tk.RIGHT)

        tab_overview_update.pack(side=tk.TOP)

        tab_rules = ttk.Frame(tabControl)
        rules_text = tk.Label(tab_rules, text="Rules:")
        rules_text.pack(side=tk.TOP)

        tab_contracts = ttk.Frame(tabControl)
        tab_misc = ttk.Frame(tabControl)

        tabControl.add(tab_overview, text="Overview")
        tabControl.add(tab_rules, text="Rules")
        tabControl.add(tab_contracts, text="Contracts")
        tabControl.add(tab_misc, text="Misc.")

        tabControl.pack(fill=tk.BOTH)

    def update(self):
        self.mgame.state.id = self.get_id()
        self.mgame.state.day = int(self.get_day())
        phase_str = self.get_phase()
        phase = mafiabot.MPhase.INIT
        if phase_str == "DAY":
            phase = mafiabot.MPhase.DAY
        elif phase_str == "NIGHT":
            phase = mafiabot.MPhase.NIGHT
        elif phase_str == "DUSK":
            phase = mafiabot.MPhase.DUSK
        self.mgame.state.phase = phase

        # Now update players
        for p_id,p_dict in self.get_players.items():
            player = self.mgame.state.players[p_id]
            player.role = p_dict['role']()
            vote = p_dict['vote']()
            target = p_dict['target']()
            player.vote = None if vote==None else vote
            player.target = None if target==None else target
        
        for p_id in self.mgame.state.players:
            if not p_id in self.get_players:
                del self.mgame.state.players[p_id]
        
        # if phase == mafiabot.MPhase.DAY:
        #     for hook in self.show_DAY:
        #         hook()
        # elif phase == mafiabot.MPhase.NIGHT:
        #     for hook in self.show_NIGHT:
        #         hook()

        print("updated")
        
    def save(self):
        self.update
        # TODO: Validate
        f = open("data/game_4.maf",'w')
        self.mgame.writeGame(f)
        f.close()
f = open("data/game_4.maf",'r')
MGE = MGameEditor(mafiabot.MGame.from_json(f, mafiabot.TestMChat, mafiabot.TestMDM, print))
f.close()

MGE.root.mainloop()