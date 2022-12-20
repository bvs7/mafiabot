

# MafiaGame

Holds data for a mafia game...

## Ideas

Right now, because we can have different types of game states based on the current phase, we pass pieces of the game state to functions. This is awkward as it requires passing all data that gets used. While this makes sense, it would be easier to not have to do that, and to just use self inside functions.

## Solutions

Overview:

- Look up the phase at the beginning of every helper fn.
- Implement handle_day in Day struct. Pass in mutable players. Return type determines next phase.
    - No access to other game fields (rx,tx). Need to pass those in...
- Local votes/actions
    - No clear way to save votes/actions?
- Figure out std::cell?
    - Very hard to serialize...
- Pass everything in.
    - Players + Interface


### Raise the enum to the full Game

```rust
pub enum Game<U:RawPID>{
    Init{
        players: Players<U>,
    },
    Day{
        players: Players<U>,
        day_no: usize,
        votes: Votes,
    },
    Night{
        players: Players<U>,
        night_no: usize,
        actions: Actions
    },
    End{
        winner: Winner,
    }
}

```

This doesn't really seem to solve the problem... If I match Day, then call a subfn, that subfn doesn't know it is day.

### Raise the enum to the full Game with full phase structs

```rust

pub struct Init{
    players: Players<U>,
}
pub struct Day{
    players: Players<U>,
    day_no: usize,
    votes: Votes,
},
pub struct Night{
    players: Players<U>,
    night_no: usize,
    actions: Actions
}
pub struct End{
    winner: Winner,
}

pub enum Game<U:RawPID>{
    Init(Init),
    Day(Day),
    Night(Night),
    End(End),
}

```

Now, after matching the phase, we would call a function on another struct. So events that only happen for a specific phase are implemented there. (i.e. Day has handle_vote, handle_election, etc). But this means we lose access to the basic self. So any phase spanning functions can't be called from inside Day.

So what we want is...
- To match on a phase, then call a function that knows what that phase is.
- Be able to have a mutable reference to a "self" as well as a mutable reference to phase

### Phase is just an enum?
```rust
pub enum Phase{
    Init,
    Day,
    Night,
    End,
}

pub struct Game{
    players: Players<U>,
    actions: Actions, // Where actions can implement votes as well
    phase: Phase,
}

impl Game{
    pub fn thread(&mut self) {
        match phase {
            Day => self.handle_day()
        }
    }
}
```

This confusingly uses actions in two different ways. But it allows phase to be changed.

### What we have now:

```rust
pub enum Phase{
    Init,
    Day{
        day_no: usize,
        votes: Votes,
    },
    Night{
        night_no: usize,
        actions: Actions,
    },
    End,
}

pub struct Game{
    players: Players<U>,
    phase: Phase,
}

impl Game{
    pub fn thread(&mut self) {
        match phase {
            Day => self.handle_day()
        }
    }
}
```

Just fetch enum at the entrance of every function?

Fundamental needs:
- self reference always accessible?
- change between Day mutability and Phase mutability

### Phase is just local to thread?

```rust
pub enum Phase{
    Init,
    Day{
        day_no: usize,
        votes: Votes,
    },
    Night{
        night_no: usize,
        actions: Actions,
    },
    End,
}

pub struct Game{
    players: Players<U>,
    //phase: Phase,
}

impl Game{
    pub fn thread(&mut self) {
        let mut next_phase;
        loop {        
            let mut phase = Phase::From(next_phase); // Copy
            let mut next_phase: Option<Phase> = match phase {
                Phase::Day{day_no, votes} => {
                    self.handle_day(&mut votes)
                }
                Phase::Night{night_no, actions} => {
                    self.handle_night(night_no, &mut actions)
                }
            }

        }
    }
}
```

### Phase and PhaseData separate. PhaseData is a local in thread

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum PhaseData {
    Day(Votes),
    Night(Actions),
    End(Winner),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum Phase {
    Init,
    Day(usize),
    Night(usize),
    End,
}

type Players<U: RawPID> = Vec<Player<U>>;

#[derive(Debug, Serialize /*Deserialize*/)]
pub struct Game<U: RawPID, S: Source> {
    players: Players<U>,
    phase: Phase,
    #[serde(skip)]
    rx: Receiver<Request<U, S>>,
    #[serde(skip)]
    tx: Sender<Response<U, S>>,
}
...
impl<U: RawPID, S: Source> Game<U, S> {
    pub fn thread(&mut self, rx: Receiver<Request<U, S>>, tx: Sender<Response<U, S>>) {
        let mut phase_data = PhaseData::Day(Vec::new());
        loop {
            // Ends mut phase_data borrow and begins it right after execution.
            (self.phase, phase_data) = match (self.phase, phase_data) {
                (Phase::Day(day_no), PhaseData::Day(votes)) => self.handle_day(day_no, votes),
                (Phase::Night(night_no), PhaseData::Night(actions)) => {
                    self.handle_night(night_no, actions)
                }
                _ => {
                    break;
                }
            }
        }
    }

    pub fn handle_day(&mut self, day_no: usize, votes: Votes) -> (Phase, PhaseData) {
        let cmd = self.rx.recv().unwrap();

        return (Phase::Day(day_no), PhaseData::Day(votes.clone()));
    }

    pub fn handle_night(&mut self, night_no: usize, actions: Actions) -> (Phase, PhaseData) {
        let cmd = self.rx.recv().unwrap();

        return (Phase::Night(night_no), PhaseData::Night(actions.clone()));
    }
}
```

Separate, but slightly awk...

Needs:
- **Game does not own ScratchPad!**
    - Scratchpad can be passed to subfn.
    - Notably, a mut Scratchpad can be passed.
    - Do that by taking Game's reference while matching on phase?
- Or if game does own scratchpad... it can be copied? Then rewritten?


### HashMap of actions:

Idea: scratchpad is a ~~HashMap~~ Vec of Phase to actions? Record all of a games actions here
Just need to figure out serializing the Vecs

```rust 
pub type Votes = Vec<(Pidx, Ballot<Pidx>)>;
pub type Actions = Vec<(Actor<Pidx>, Target<Pidx>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Phase {
    Init,
    Day(usize),
    Night(usize),
    End(Winner),
}

type Players<U> = Vec<Player<U>>;

#[derive(Debug, Serialize /*Deserialize*/)]
pub struct Game<U: RawPID, S: Source> {
    players: Players<U>,
    phase: Phase,
    #[serde(skip)]
    rx: Receiver<Request<U, S>>,
    #[serde(skip)]
    tx: Sender<Response<U, S>>,
}
...
impl<U: RawPID + 'static, S: 'static + Source> Game<U, S> {
    pub fn start(
        mut self,
        rx: Receiver<Request<U, S>>,
        tx: Sender<Response<U, S>>,
    ) -> JoinHandle<()> {
        // Start game thread
        thread::spawn(move || self.thread(rx, tx))
    }
}

type VoteMap = Vec<Votes>;
type ActionMap = Vec<Actions>;

impl<U: RawPID, S: Source> Game<U, S> {
    pub fn thread(&mut self, rx: Receiver<Request<U, S>>, tx: Sender<Response<U, S>>) {
        self.rx = rx;
        self.tx = tx;
        let mut vote_map = Vec::new();
        let mut action_map = Vec::new();
        self.phase = Phase::Day(1);
        vote_map.push(Vec::new());
        loop {
            self.phase = match self.phase {
                Phase::Init => Phase::Init,
                Phase::Day(day_no) => {
                    let mut votes = vote_map.get_mut(day_no - 1).unwrap();
                    self.handle_day(day_no, votes)
                }
                Phase::Night(night_no) => {
                    let mut actions = action_map.get_mut(night_no - 1).unwrap();
                    self.handle_night(night_no, actions)
                }
                _ => Phase::End(Winner::Team(Team::Mafia)),
            }
        }
    }

    pub fn handle_day(&mut self, day_no: usize, votes: &mut Votes) -> Phase {
        let cmd = self.rx.recv().unwrap();

        return Phase::Day(day_no);
    }

    pub fn handle_night(&mut self, night_no: usize, actions: &mut Actions) -> Phase {
        let cmd = self.rx.recv().unwrap();

        return Phase::Night(night_no);
    }
}
```
Advantages:
- Game does not own votes or actions, and they are stored between rounds...
- simple handle_day/handle_night dispatch.

Disadvantages:
- need to ensure consistency by adding a new Votes or Actions in next_phase
- Votes is not part of game...

### Impl in Phase

```rust
impl Day {
    pub fn handle_day<U: RawPID>(
        &mut self,
        players: &mut Players<U>,
        cmd: Command<U>,
    ) -> (Phase, Vec<Event<U>>) {
        //let cmd = self.rx.recv().unwrap();

        return (
            Phase::Night(Night {
                night_no: self.day_no + 1,
                actions: Vec::new(),
            }),
            Vec::new(),
        );
    }
}
```
- pass mutable players into sub?

This doesn't work...