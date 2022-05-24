

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

NOTARGET_MSG = "No Kill"

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

class EditPlayerWindow(tk.Toplevel):

    window_title = "Edit Player"

    id_entry_state = tk.DISABLED
    def __init__(self, master, player, callback):
        self.player : mgs.Player = player
        self.callback = callback
        super().__init__(master)

        self.create()

    def create(self):
        self.title = self.window_title

        r = 0
        self.frame = ttk.Frame(self)
        self.frame.columnconfigure(0, weight=1)
        self.frame.columnconfigure(1, weight=1)
        ttk.Label(self.frame, text="ID:").grid(column=0,row=r)
        self.id_entry = ttk.Entry(self.frame)
        self.id_entry.insert(0,str(self.player.id))
        self.id_entry.configure(state=self.__class__.id_entry_state)
        self.id_entry.grid(column=1,row=r)
        r += 1

        ttk.Label(self.frame, text="Name:").grid(column=0,row=r)
        self.name_entry = ttk.Entry(self.frame)
        self.name_entry.insert(0,"name")
        self.name_entry.grid(column=1,row=r)
        r+=1

        ttk.Label(self.frame, text="Role:").grid(column=0,row=r)
        self.role_combobox = ttk.Combobox(self.frame, values=list(mgs.Role._member_map_.keys()))
        self.role_combobox.set(self.player.role.name)
        self.role_combobox.grid(column=1,row=r)
        r+=1

        self.extra_r = r
        self.updateExtra()
        self.role_combobox.bind("<<ComboboxSelected>>", self.updateExtra)
        r+=1

        ttk.Button(self.frame, text="Done", command=self.editPlayerAndReturn).grid(column=0, columnspan=2, row=r)

        self.frame.pack()

    def editPlayerAndReturn(self):
        role = mgs.Role(self.role_combobox.get())
        if role.contracting:
            charge = self.extra_entry.get().strip("|")[0]
            role = (role, mgs.PlayerID(charge))
        self.player.role = role
        self.callback(self.player)
        self.destroy()


    def updateExtra(self, e=None):
        if hasattr(self,"extra_label"):
            self.extra_label.destroy()
        if hasattr(self, "extra_entry"):
            self.extra_entry.destroy()
        
        role = mgs.Role(self.role_combobox.get())
        if role.contracting:
            self.extra_label = ttk.Label(self.frame, text="Charge:")
            self.extra_label.grid(column=0,row=self.extra_r)
            charges = [f"{p.id}|name" for p in self.master.m.players]
            self.extra_entry = ttk.Combobox(self.frame, values = charges)
            if self.player.role.contracting:
                self.extra_entry.set(f"{self.player.charge}|name")
            self.extra_entry.grid(column=1, row=self.extra_r)


class AddPlayerWindow(EditPlayerWindow):

    window_title = "Add Player"

    id_entry_state = tk.NORMAL

    def __init__(self, master, callback):
        super().__init__(master, mgs.Player("-"),callback)

    def editPlayerAndReturn(self):
        self.player = mgs.Player(mgs.PlayerID(self.id_entry.get()))
        super().editPlayerAndReturn()

class PlayerTab(MafiaGameEditorTab):

    def create(self):
        ttk.Label(self, text="Player Details", justify='center'
            ).pack(padx=100,pady=15)#grid(column=0,row=0, sticky="N",padx=100,pady=15)

        self.playerTable = self.createPlayerTable()

        self.playerTable.pack()

        ttk.Button(self,command=self.editPlayer, text="Edit Player"
            ).pack(side=tk.BOTTOM)
        ttk.Button(self,command=self.addPlayer, text="Add Player"
            ).pack(side=tk.BOTTOM)

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

        return playerTable

    def editPlayer(self):
        try:
            playerRow = self.playerTable.selection()[0]
        except IndexError:
            return
        p_id = self.playerTable.item(playerRow)['values'][0]
        p = self.m.getPlayer(p_id)
        edit_player = EditPlayerWindow(self,p, self.updateFields)

    def addPlayer(self):
        def add_p_callback(p):
            self.m.players.add(p)
            self.updateFields()
        AddPlayerWindow(self, add_p_callback)


    def updateFields(self, p=None):
        self.playerTable.destroy()
        self.playerTable = self.createPlayerTable()
        self.playerTable.pack()

    def applyCommand(self):
        pass


class RoundTab(MafiaGameEditorTab):

    def create(self):
        self.columnconfigure(0,weight=1)

        r = 0

        ttk.Label(self, text="Current Round and Specific State", justify='center'
            ).grid(column=0,columnspan=2,row=r, sticky="N",padx=100,pady=15)
        r+=1

        # Day
        ttk.Label(self, text="Day:").grid(column=0,row=r)
        self.day_entry = ttk.Spinbox(self, from_=0)
        self.day_entry.grid(column=1,row=r)
        r+=1

        # Phase
        ttk.Label(self, text="Phase:").grid(column=0,row=r)
        self.phase_entry = ttk.Combobox(self, values=list(mgs.Phase._member_map_.keys()))
        self.phase_entry.grid(column=1, row=r)
        r+=1

        self.phase_frame = ttk.Frame(self)
        self.phase_frame_r = r
        self.phase_frame.grid(column=0, columnspan=2, row=r)
        self.phase_entry.bind("<<ComboboxSelected>>", self.updatePhase)
        r+=1

        # Start
        ttk.Label(self, text="Round Start Time:").grid(column=0,row=r)
        self.start_entry_var = tk.StringVar(self)
        ttk.Entry(self, textvariable=self.start_entry_var).grid(column=1,row=r)
        r+=1

        self.updateFields()


    def updatePhase(self, e=None):
        phase = mgs.Phase(self.phase_entry.get())
        self.m.round.phase = phase
        self.phase_frame.destroy()
        self.phase_frame = ttk.Frame(self)
        self.phase_frame.grid(column=0, columnspan=2, row=self.phase_frame_r)

        if phase == mgs.Phase.DAY:
            self.createVotesTable()
        elif phase == mgs.Phase.NIGHT:
            targets = [f"{p.id} | name of {p.id}" for p in self.m.players] + [NOTARGET_MSG] + [None]
            self.mafia_target_box = ttk.Combobox(self.phase_frame, values=targets)
            

    def createVotesTable(self):
        frame = self.phase_frame
        frame.columnconfigure(0, weight=1)
        frame.columnconfigure(1, weight=1)
        
        ttk.Label(frame, text="Voter:").grid(column=0,row=0)
        ttk.Label(frame, text="Votee:").grid(column=1,row=0)
        for (n,p) in enumerate(self.m.players):
            ttk.Label(frame, text=f"{p.id} | name of {p.id}").grid(column=0,row=1+n)
            
            votees = [f"{p.id} | name of {p.id}" for p in self.m.players] + [NOTARGET_MSG] + [None]
            self.votee_dict = {}
            w=ttk.Combobox(frame, values = votees)
            w.player_id = p.id
            w.grid(column=1, row = 1+n)
            
            if p in self.m.round.votes:
                votee = self.m.round.votes[p]
                if votee == mgs.NOTARGET:
                    w.set(NOTARGET_MSG)
                else:
                    w.set(f"{votee} | name of {votee}")
            else:
                w.set("None")

            def applyVotee(e):
                v = e.widget.get()
                p_id = e.widget.player_id
                if v in [None, "None"]:
                    if p_id in self.m.round.votes:
                        del self.m.round.votes[p_id]
                else:
                    if v==NOTARGET_MSG:
                        votee_id = mgs.NOTARGET
                    else:
                        votee_id = mgs.PlayerID(v.split("|")[0].strip())
                    self.m.round.votes[p_id] = votee_id
                return self.updateFields()

            w.bind("<<ComboboxSelected>>", applyVotee)
            self.votee_dict[p] = w


    def updateFields(self):
        self.day_entry.set(self.m.round.day)
        self.phase_entry.set(self.m.round.phase.name)
        self.updatePhase()
        self.start_entry_var.set(self.m.round.start)

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
            initialfile=self.fname.split("/")[-1],
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

        def tabChanged(e):
            self.tabControl.nametowidget(self.tabControl.select()
                ).updateFields()

        self.tabControl.bind("<<NotebookTabChanged>>", tabChanged)
        
        ttk.Button(self, command=self.applyCommand, text="Apply"
            ).pack()

        self.bind("<Return>", self.applyCommand)
        self.bind("<Control-s>", self.saveGame)
        self.bind("<Control-Shift-s>", self.saveAsGame)
        self.bind("<Control-o>", self.openGame)
        self.bind("<Control-n>", self.newGame)

if __name__ == "__main__":
    mge = MafiaGameEditor("test.maf")
    mge.mainloop()