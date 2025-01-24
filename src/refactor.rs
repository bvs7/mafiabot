enum Error {}

#[derive(Debug, Clone)]
struct Pid(u64);

type Choice = Option<Pid>;
type Ballot = Option<Choice>;

// These are using Pid, does this mean they are validated?

#[derive(Debug, Clone)]
struct Vote {
    voter: Pid,
    ballot: Ballot,
}

#[derive(Debug, Clone)]
struct Target {
    actor: Pid,
    choice: Choice,
}

#[derive(Debug, Clone)]
enum Action {
    Vote(Vote),
    Target(Target),
}

#[derive(Debug, Clone)]
struct Init {}

#[derive(Debug, Clone)]
struct Day {}

#[derive(Debug, Clone)]
struct Night {}

#[derive(Debug, Clone)]
struct Eclipse {}

#[derive(Debug, Clone)]
struct End {}

#[derive(Debug, Clone)]
enum Phase {
    Init,
    Day(Day),
    Night(Night),
    Eclipse(Eclipse),
    End(End),
}

struct Status {}

enum Input {
    Status(tokio::sync::oneshot::Sender<Status>),
    Action((Action, tokio::sync::oneshot::Sender<Result<(), Error>>)),
}

#[derive(Debug, Clone)]
enum Event {}

struct Game<P> {
    state: tokio::sync::RwLock<StateHolder>,
    input_rx: tokio::sync::mpsc::Receiver<Input>,
    input_tx: tokio::sync::mpsc::Sender<Input>,
    event_tx: tokio::sync::broadcast::Sender<Event>,
    alarm_time: Option<chrono::DateTime<chrono::Local>>,
}

// Is this even what we want? Or do we want a RwLock<State>?
// We could still do this with state...
// Status could be a method on Arc<RwLock<State>>
// And actions could still be submitted?

// We could have two tasks. One for reading and one for writing?
// Aka one for actions and one for status requests.
// Or we just have the Arc<RwLock<State>> passed into and returned from game?
// Then have the status function call on Arc<RwLock<State>>.read()

// So yes, have a wrapper for all these things...

// Do we need a wrapper anyway for the groupme implementation?

// Oh well. I like the idea of having a task running under the state, which listens
// for actions and updates the state accordingly.

// The inner state, again, how does it send out events?.....
// The function pointer? An option for a function pointer?
// Or we have some kind of initialization for State?
// State becomes... state with event_tx?
// It seems like state shouldn't know about the event_tx... But how could that be?
// Maybe events are queued in action handler, then read out in update? no...
// Oh, we just give it a clone of the event_tx. That's it.

// Idea:
// State has a generic Phase
// So we can define the state as having a specific phase, then only allow certain actions based on that
// Then we can have a function to transition to the next phase
// The only thing is we have to have a way to replace the state...
// Like some kind of interior mutability...

impl Game {
    async fn new(mut state: State<Init>) -> Self {
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (event_tx, _) = tokio::sync::broadcast::channel(100);
        state.event_tx = Some(event_tx.clone());
        let alarm_time = None;
        let game = Self {
            state: tokio::sync::RwLock::new(state),
            input_rx,
            input_tx,
            event_tx,
            alarm_time,
        };
        game
    }

    fn start(self) {
        tokio::spawn(self.run());
    }

    async fn run(self) {
        loop {
            // use tokio::timeout to also wait for next alarm time!
            // Eventually this allows end phase timers to be set.
            let action = todo!();
            let resp: tokio::sync::oneshot::Sender<Result<(), Error>> = todo!();
            let result = self.handle_action(action).await;
            let _ = resp.send(result);
        }
        todo!()
    }

    fn input_tx(&self) -> tokio::sync::mpsc::Sender<Input> {
        self.input_tx.clone()
    }

    fn event_rx(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }

    /// Check validity with just read, then try writing.
    async fn handle_action(&self, action: Action) -> Result<(), Error> {
        let rstate = self.state.read().await;
        rstate.validate_action(&action)?;
        drop(rstate);
        let mut wstate = self.state.write().await;
        wstate.validate_action(&action)?;
        wstate.action(action);
        Ok(())
    }

    async fn status(&self) -> Status {
        let rstate = self.state.read().await;
        let status = rstate.status();
        status
    }
}

// Could use typestate to have state based on phase better?

enum StateHolder {
    Init(State<Init>),
    Day(State<Day>),
    Night(State<Night>),
    Eclipse(State<Eclipse>),
    End(State<End>),
}

impl StateHolder {
    fn validate_action(&self, action: &Action) -> Result<(), Error> {
        todo!()
    }

    fn action(&mut self, action: Action) {
        let _ = std::mem::replace(self, StateHolder::End(State::<End>::default()));
        todo!()
    }
}

struct State<P> {
    day: u32,
    phase: P,
    event_tx: Option<tokio::sync::broadcast::Sender<Event>>,
}

impl Default for State<Eclipse> {
    fn default() -> Self {
        Self {
            day: 0,
            phase: Eclipse {},
            event_tx: None,
        }
    }
}

impl Default for State<End> {
    fn default() -> Self {
        Self {
            day: 0,
            phase: End {},
            event_tx: None,
        }
    }
}

impl<P> State<P> {
    fn status(&self) -> Status {
        todo!()
    }

    fn validate_action(&self, action: &Action) -> Result<(), Error> {
        todo!()
    }

    /// Action must not be able to fail. Allow panic on error in here?
    fn action(&mut self, action: Action) {
        todo!()
    }

    fn send(&self, event: Event) {
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(event);
        }
    }
}

impl State<Day> {
    fn vote(&mut self, vote: Vote) {
        todo!()
    }

    fn night(self) -> State<Night> {
        todo!()
    }
}
