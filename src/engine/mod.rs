mod game;
mod interface;
mod role;
mod state;

use game::Game;
use interface::{Action, ActionMsg, ActionRx, ActionTx, Error, Event, EventRx, EventTx};
use role::{Role, RoleKind, Team};
use state::{Phase, PhaseKind, Rules, State};
