use std::collections::HashMap;

use mafia::{game::Action, rules::Rules, state::State, Role, Team};

use tracing::debug;

#[test]
#[tracing_test::traced_test]
fn basic_game() {
    // let registry = vec![(1, Role::TOWN), (2, Role::TOWN), (3, Role::MAFIA)];
    // let rules = Rules::default();
    // let mut state = State::new(registry, rules);

    // let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    // state.event_tx = Some(event_tx);

    // state.start();
    // let va = state.validate_action(Action::Vote { voter: 1, ballot: Some(Some(3)) }).unwrap();
    // state.perform_action(va.into());
    // state.update();

    // let s = state.status(HashMap::<u64, String>::new());
    // debug!("{:?}", s);

    // let va = state.validate_action(Action::Vote { voter: 2, ballot: Some(Some(3)) }).unwrap();
    // state.perform_action(va.into());
    // state.update();

    // let s = state.status(HashMap::<u64, String>::new());
    // debug!("{:?}", s);

    // debug!("{:?}", state);

    // std::thread::sleep(std::time::Duration::from_secs(11));

    // state.update();

    // debug!("{:?}", state);

    // while let Ok(event) = event_rx.try_recv() {
    //     debug!("{:?}", event);
    // }
}
