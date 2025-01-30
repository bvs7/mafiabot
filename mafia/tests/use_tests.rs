use mafia::state::State;

use mafia::game::Action;

use mafia::{Pid, Role, RoleKind, Team};

use mafia::game::Event;

fn test() {
    let action = Action::Vote { voter: 0, ballot: Some(Some(1)) };
}
