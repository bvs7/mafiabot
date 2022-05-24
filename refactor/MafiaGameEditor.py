

import tkinter as tk
from tkinter import ttk, filedialog, messagebox, simpledialog
from collections import OrderedDict
from typing import List
import logging

import MafiaGameState as mgs

logging.basicConfig(level=logging.INFO)

CHAT_HANDLE_WIDTH = 20
GAME_NUMBER_WIDTH = 4
PLAYER_ID_WIDTH = 10
PLAYER_NAME_WIDTH = 10
ROLE_WIDTH = 10

class EditPlayerDialog(simpledialog.Dialog):
    def __init__(self, parent, player):
        self.player = player
        super().__init__(parent, "Edit Player")

    def body(self, frame):
        ttk.Label(frame, text="ID").pack()
        w = ttk.Entry(frame, state=tk.DISABLED)
        w.insert(0,str(self.player.id))
        w.pack()

class MafiaGameEditorTab(ttk.Frame):

    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)

        self.create()
        self.updateFields()

    @property
    def m(self) -> mgs.MafiaGameState:
        return self.master.master.m

    def create(self):
        raise NotImplementedError

    def updateFields(self):
        raise NotImplementedError
    
    def applyCommand(self):
        raise NotImplementedError

class InfoTab(MafiaGameEditorTab):

    def create(self):
        self.columnconfigure(0,weight=1)
        self.columnconfigure(1,weight=1)
        ttk.Label(self, text = "General Game State Info", justify='center'
            ).grid(column=0,row=0,columnspan=2, sticky="N",padx=100,pady=15)

        self.lobby_chat_id = self.addEntry("Lobby Chat ID:", CHAT_HANDLE_WIDTH)
        self.game_number   = self.addEntry("Game Number:", GAME_NUMBER_WIDTH)
        self.main_chat_id = self.addEntry("Main Chat ID:", CHAT_HANDLE_WIDTH)
        self.mafia_chat_id = self.addEntry("Mafia Chat ID:", CHAT_HANDLE_WIDTH)


    def addEntry(self, prompt, width=None):
        PADX=30
        PADY=8
        ttk.Label(self, text=prompt).grid(column=0,row=self.grid_size()[1],sticky="w",padx=PADX,pady=PADY)
        strvar = tk.StringVar()
        ttk.Entry(self, textvariable=strvar, width=width, justify="center",
            ).grid(column=1,row=self.grid_size()[1]-1,sticky="e",padx=PADX,pady=PADY)
        return strvar

    def applyCommand(self):
        # Validate
        def validate(strvar:tk.StringVar,ch : mgs.ChatHandle):
            s = strvar.get()
            try:
                ch.id = int(s)
            except ValueError:
                strvar.set(ch.id)

        validate(self.lobby_chat_id, self.m.lobby_chat)
        validate(self.main_chat_id, self.m.main_chat)
        validate(self.mafia_chat_id, self.m.mafia_chat)

        try:
            gn = int(self.game_number.get())
            if not gn >= 0:
                raise ValueError(f"Entered invalid game number: {gn}")
            self.m.game_number = gn
        except ValueError:
            self.game_number.set(self.m.game_number)

    def updateFields(self):
        self.lobby_chat_id.set(str(self.m.lobby_chat.id))
        self.game_number.set(str(self.m.game_number))
        self.main_chat_id.set(str(self.m.main_chat.id))
        self.mafia_chat_id.set(str(self.m.mafia_chat.id))

class PlayerTab(MafiaGameEditorTab):

    def create(self):
        ttk.Label(self, text="Player Details", justify='center'
            ).pack(padx=100,pady=15)#grid(column=0,row=0, sticky="N",padx=100,pady=15)

        self.playerTable = self.createPlayerTable()

        self.playerTable.pack()

        ttk.Button(self,command=self.editPlayer, text="Edit Player"
            ).pack(side=tk.BOTTOM)


        # self.players = OrderedDict()
        # self.playerData = ttk.Frame(self)

        # self.playerData.columnconfigure(0,weight=2)
        # self.playerData.columnconfigure(1,weight=2)
        # self.playerData.columnconfigure(2,weight=1)
        # self.playerData.columnconfigure(3,weight=1)
        # self.playerData.columnconfigure(4,weight=2)

        # ## Header Row TODO
        # ttk.Label(self.playerData, text="ID", justify=tk.CENTER).grid(column=0,row=0)
        # ttk.Label(self.playerData, text="Name", justify=tk.CENTER).grid(column=1,row=0)
        # ttk.Label(self.playerData, text="Role", justify=tk.CENTER).grid(column=2,row=0)
        # ttk.Label(self.playerData, text="Team", justify=tk.CENTER).grid(column=3,row=0)
        # ttk.Label(self.playerData, text="...", justify=tk.CENTER).grid(column=4,row=0)

        # for player in sorted(self.m.players, key=lambda p:p.role):
        #     self.addPlayer(player)

        # self.playerData.pack()

        # self.updateFields()

    def createPlayerTable(self):
        cols = ("ID", "Name", "Role", "...")

        playerTable = ttk.Treeview(self)
        playerTable['columns'] = [col.lower() for col in cols]
        playerTable.column("#0", width=0, stretch=tk.NO)
        for col in cols:
            playerTable.column(col.lower(), anchor=tk.CENTER, width = 80)

        playerTable.heading("#0",text="",anchor=tk.CENTER)
        for col in cols:
            playerTable.heading(col.lower(), text=col, anchor=tk.CENTER)

        for n,player in enumerate(sorted(self.m.players, key=lambda x:x.role)):
            playerTable.insert(parent="",index='end', iid=n,text="",values = player.to_tuple())

        logging.info(playerTable.__dict__)

        return playerTable

    def editPlayer(self):
        playerRow = self.playerTable.selection()[0]
        logging.info(f"playerRow, self.playerTable.item(playerRow): {playerRow, self.playerTable.item(playerRow)}")

    # def sortPlayers(self):
    #     logging.debug(f"Sorting Players: {self.players.keys()}")
    #     self.players = OrderedDict([(k,self.players[k]) for k in sorted(self.players)])
    #     logging.debug(f"Done: {self.players.keys()}")
    #     for i,(player,(ws,vars)) in enumerate(self.players.items()):
    #         for j, w in enumerate(ws):
    #             w.grid(column=j,row=i+1)

    # def addPlayer(self, player:mgs.Player):
    #     r = row=self.playerData.grid_size()[1]

    #     logging.debug(f"Adding player: {player}")

    #     def update(event):
    #         logging.info(str(event))
    #         return self.updateExtraWidget(player)

    #     idVar = tk.StringVar(value = player.id)
    #     idEntry = ttk.Entry(self.playerData, textvariable=idVar, width=PLAYER_ID_WIDTH, justify=tk.CENTER)
    #     idEntry.grid(column=0, row=r)
    #     nameVar = tk.StringVar(value = "name")
    #     nameEntry = ttk.Entry(self.playerData, textvariable=nameVar, width=PLAYER_NAME_WIDTH, justify=tk.CENTER)
    #     nameEntry.grid(column=1, row=r)
    #     roleVar = tk.StringVar(value = player.role.name)
    #     roleCombobox = ttk.Combobox(self.playerData, textvariable=roleVar, justify=tk.CENTER, width=ROLE_WIDTH,
    #         values=list(mgs.Role._member_map_.keys()))
    #     roleCombobox.bind("<<ComboboxSelected>>", update)
    #     roleCombobox.grid(column=2, row=r)
    #     teamVar = tk.StringVar(value = player.role.team)
    #     teamLabel = ttk.Entry(self.playerData, textvariable=teamVar, state=tk.DISABLED, justify=tk.CENTER)
    #     teamLabel.grid(column=3, row=r)
    #     extraVar = tk.StringVar(value="-")
    #     extraWidget = ttk.Entry(self.playerData,textvariable=extraVar, state=tk.DISABLED, justify=tk.CENTER)
    #     extraWidget.grid(column=4, row=r)

    #     self.players[player] = ((idEntry, nameEntry, roleCombobox, teamLabel, extraWidget),
    #                              (idVar,   nameVar,   roleVar,      teamVar,   extraVar))
        
    #     self.updateExtraWidget(player)

    # def updateExtraWidget(self, player):
    #     ((idEntry, nameEntry, roleCombobox, teamLabel, extraWidget),
    #     (idVar,nameVar,roleVar,teamVar,extraVar)) = self.players[player]
    #     role = mgs.Role(roleVar.get())
    #     c = extraWidget.grid_info()['column']
    #     r = extraWidget.grid_info()['row']
    #     extraWidget.destroy()
    #     extraVar.set("None")
    #     if role.contracting:
    #         extraWidget = ttk.Combobox(self.playerData,textvariable=extraVar, width = PLAYER_ID_WIDTH, 
    #             justify=tk.CENTER, values = ([None]+[p.id for p in self.players]))
    #     else:
    #         extraWidget = ttk.Entry(self.playerData, textvariable=extraVar, width = PLAYER_ID_WIDTH,
    #             justify=tk.CENTER, state= tk.DISABLED)
    #     extraWidget.grid(column=c,row=r)
    #     self.players[player] = ((idEntry, nameEntry, roleCombobox, teamLabel, extraWidget),
    #         (idVar,nameVar,roleVar,teamVar,extraVar))

        

    # def updatePlayerData(self, player):
    #     ((idEntry, nameEntry, roleCombobox, teamLabel, extraWidget),
    #     (idVar,nameVar,roleVar,teamVar,extraVar)) = self.players[player]
    #     idVar.set(player.id)
    #     nameVar.set("Name_")
    #     roleVar.set(player.role.name)
    #     teamVar.set(player.role.team)
    #     if player.role.contracting:
    #         extraVar.set(player.charge)
    #         extraWidget.configure(state=tk.NORMAL)
    #     else:
    #         extraVar.set("-")
    #         extraWidget.configure(state=tk.DISABLED)

    # def updatePlayer(self, player):
    #     (ws,(idVar,nameVar,roleVar,teamVar,extraVar)) = self.players[player]

    #     del self.players[player]

    #     player.id = int(idVar.get())
    #     player.role = mgs.Role(roleVar.get())

    #     if player.role.contracting:
    #         player.charge = int(extraVar.get())
    #     else:
    #         if hasattr(player, "charge"):
    #             del player.charge

    #     self.players[player] = (ws,(idVar,nameVar,roleVar,teamVar,extraVar))

    # def removePlayer(self, player):
    #     (ws,(idVar,nameVar,roleVar,teamVar,extraVar)) = self.players[player]
    #     for w in ws:
    #         w.destroy()

    #     del self.players[player]

    # def getChangedPlayerData(self):
    #     changed = set()
    #     for player in self.players:
    #         (ws,(idVar,nameVar,roleVar,teamVar,extraVar)) = self.players[player]
    #         if (not player.id == idVar.get() or
    #             not player.role.name == roleVar.get() or
    #             (player.role.contracting and not player.charge == extraVar.get())):
    #             changed.add(player)
    #     return changed


    def updateFields(self):
        self.playerTable.destroy()
        self.playerTable = self.createPlayerTable()
        self.playerTable.pack()
        # for player in list(self.players):
        #     self.remove(player)
 
        # for player in self.m.players:
        #     self.addPlayer(player)

        # self.sortPlayers()
    
    def applyCommand(self):
        pass

        # for player in list(self.m.players):
        #     self.m.players.remove(player)
        
        # for player in list(self.players):
        #     self.updatePlayer(player)
        #     self.m.players.add(player)
        
        # self.sortPlayers()



class RoundTab(MafiaGameEditorTab):

    def create(self):
        self.columnconfigure(0,weight=1)
        ttk.Label(self, text="Current Round and Specific State", justify='center'
            ).grid(column=0,row=0, sticky="N",padx=100,pady=15)

    def updateFields(self):
        print("Round Update!")
    
    def applyCommand(self):
        print("Round Apply!")

class MafiaGameEditor(tk.Tk):

    def __init__(self, fname=None):
        super().__init__()
        self.m = mgs.MafiaGameState()
        if fname:
            self.m = mgs.MafiaGameState.load(fname)
        self.fname = fname
        self.modified = False

        self.create_widget()
        self.create_menu()

    def promptSaveBefore(self):
        
        confirm = messagebox.askyesnocancel("Mafia Game Editor", "Save unsaved work?")
        if confirm: #yes
            self.saveGame()
            return True
        elif confirm is not None: #no
            return True
        else:
            return False


    def newGame(self, e=None):
        if self.modified:
            if not self.promptSaveBefore():
                return
        self.m = mgs.MafiaGameState()
        self.updateFields()

    def openGame(self, e=None):
        if self.modified:
            if not self.promptSaveBefore():
                return

        fname = filedialog.askopenfilename(
            initialdir=".",
            title="Open file...",
            initialfile=self.fname,
            defaultextension = ".maf",
            filetypes = (("Mafia Game files","*.maf"),("all files","*.*"))
        )

        if fname == "":
            return

        try:
            self.m = mgs.MafiaGameState.load(fname)
            self.fname = fname
            self.updateFields()
        except FileNotFoundError:
            # TODO error dialog
            pass
        
    def saveAsGame(self, e=None):
        fname = filedialog.asksaveasfilename(
            initialdir=".",
            title="Save file as...",
            initialfile=self.fname,
            defaultextension = ".maf",
            filetypes = (("Mafia Game files","*.maf"),("all files","*.*"))
        )
        if fname == "":
            return
        self.fname = fname
        self.saveGame()
        

    def saveGame(self, e=None):
        if not self.fname or self.fname == "":
            self.saveAsGame()

        try:
            self.m.save(self.fname)
            self.modified = False
            return True
        except FileNotFoundError:
            # TODO error dialog
            self.fname = None
            return False

    def updateFields(self):
        tabs : List[MafiaGameEditorTab] = self.tabControl.winfo_children()
        [tab.updateFields() for tab in tabs]
        self.modified = False

    def applyCommand(self, e=None):
        tabs : List[MafiaGameEditorTab] = self.tabControl.winfo_children()
        [tab.applyCommand() for tab in tabs]
        self.modified = True

    def create_menu(self):
        menubar = tk.Menu(self)
        filemenu = tk.Menu(menubar, tearoff=0)
        filemenu.add_command(label="New", command=self.newGame)
        filemenu.add_command(label="Open", command=self.openGame)
        filemenu.add_command(label="Save", command=self.saveGame)
        filemenu.add_command(label="Save As", command=self.saveAsGame)

        menubar.add_cascade(label="File", menu=filemenu)

        self.config(menu=menubar)


    def create_widget(self):
        self.title("Mafia Game Editor")

        self.columnconfigure(0, weight=1)
        self.tabControl = ttk.Notebook(self)

        self.infoTab = InfoTab(self.tabControl)
        self.playerTab = PlayerTab(self.tabControl)
        self.roundTab = RoundTab(self.tabControl)

        self.tabControl.add(self.infoTab, text="General")
        self.tabControl.add(self.playerTab, text="Players")
        self.tabControl.add(self.roundTab, text="Current Round")

        self.tabControl.pack()
        
        ttk.Button(self, command=self.applyCommand, text="Apply"
            ).pack()

        self.bind("<Return>", self.applyCommand)
        self.bind("<Control-s>", self.saveGame)
        self.bind("<Control-Shift-s>", self.saveAsGame)
        self.bind("<Control-o>", self.openGame)
        self.bind("<Control-n>", self.newGame)

if __name__ == "__main__":
    mge = MafiaGameEditor()
    mge.mainloop()