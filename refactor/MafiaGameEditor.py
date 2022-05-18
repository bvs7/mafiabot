
from ast import Return
from typing import List

import tkinter as tk
from tkinter import ttk, filedialog, messagebox

from MafiaGameState import MafiaGameState, ChatHandle

CHAT_HANDLE_WIDTH = 20
GAME_NUMBER_WIDTH = 4

# class PromptToSaveDialog(simpledialog.Dialog):
#     def buttonbox(self):

#         box = tk.Frame(self)

#         w = tk.Button(box, text="Don't Save", width=10, command=self.nosave, default=tk.ACTIVE)
#         w.pack(side=tk.LEFT, padx=5, pady=5)
#         w = tk.Button(box, text="Save", width=10, command=self.ok, default=tk.ACTIVE)
#         w.pack(side=tk.LEFT, padx=5, pady=5)
#         w = tk.Button(box, text="Cancel", width=10, command=self.cancel)
#         w.pack(side=tk.LEFT, padx=5, pady=5)

#         self.bind("<Return>", self.nosave)
#         self.bind("<Escape>", self.cancel)

#         box.pack()
    
#     def nosave(self):
        

#     def save(self):
#         self.num = 1

class MafiaGameEditorTab(ttk.Frame):

    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)

        self.create()
        self.updateFields()

    @property
    def m(self) -> MafiaGameState:
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
        def validate(strvar:tk.StringVar,ch : ChatHandle):
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
        self.columnconfigure(0,weight=1)
        ttk.Label(self, text="Player Details", justify='center'
            ).grid(column=0,row=0, sticky="N",padx=100,pady=15)

        self.updateFields()


    def updateFields(self):

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

        playerTable.grid(column=0,row=1)
    
    def applyCommand(self):
        print("Player Apply!")

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
        self.m = MafiaGameState()
        if fname:
            self.m = MafiaGameState.load(fname)
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
        self.m = MafiaGameState()
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
            self.m = MafiaGameState.load(fname)
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

    def applyCommand(self):
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