// A walled off state. This means it does everything with message passing

enum Error {}

enum Input {
    Status(mpsc::Sender<Status>),
    Action((Action, mpsc::Sender<Result<()>, Error>)),
    Save,
    Quit,
}

enum End {
    InputClosed,
    GameOver(Team),
    SaveAndQuit,
}

// How does the inner state send out events?
// It could have a channel
// It could have a function pointer?
// It could have a trait object? (I like this one)
//      What else could this trait object do? Hmmm

struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    input_rx: std::sync::mpsc::Receiver<Input>,
    next_time: Option<DateTime<Local>>,
    event_tx: tokio::sync::broadcast::Sender<Event>,
}

impl State {
    fn new(
        registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>,
        rules: Rules,
        input_rx: mpsc::Receiver<Input>,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        let state = State::new(registry, rules, event_tx.send);
    }

    fn start(self) {
        tokio::spawn(move || {
            self.run();
        });
    }

    fn run(mut self) {
        loop {
            if let Some(time) = self.next_time {
                if time < Local::now() {
                    // Update the state before recv actions.
                    self.next_time = self.state.update();
                }
            }

            match self.input_rx.try_recv() {
                Ok(Status(tx)) => {
                    tx.send(self.status()).unwrap();
                }
                Ok(Action((act, tx))) => {
                    tx.send(self.handle_action(act)).unwrap();
                    if let Some(time) = self.state.update() {
                        self.next_time = time;
                    }
                    // if game is over... then end_tx
                }
                Err(TryRecvError::Empty) => {
                    // No more inputs
                }
                Err(TryRecvError::Closed) => {
                    // TODO: Save the game and quit. Send end message
                    break;
                }
            }
        }
    }
}
