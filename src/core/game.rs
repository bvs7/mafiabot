

use super::{Phase, Contracts, PIDs, RoleGen, GameRules};

// Entrants + RoleGen -> Players + Contracts

pub struct Game {
    game_id: usize,
    phase: Phase,
    contracts: Contracts,
    entrants: PIDs,
    rolegen: RoleGen,
    rules: GameRules,
}

// ***

// pub trait PlayerCheck {
//     fn check(&self, raw_pid: PID) -> Result<Pidx, InvalidActionError>;
// }

// impl PlayerCheck for Players {
//     fn check(&self, raw_pid: PID) -> Result<Pidx, InvalidActionError> {
//         self.iter()
//             .position(|p| p.user_id == raw_pid)
//             .ok_or_else(|| InvalidActionError::PlayerNotFound { pid: raw_pid })
//     }
// }


// impl Game {
//     pub fn new(
//         game_id: usize,
//         players: Players,
//         contracts: Vec<Contract>,
//         comm: Comm,
//     ) -> Self {
//         let mut game = Self {
//             game_id,
//             players: Vec::new(),
//             phase: Phase::Init,
//             contracts,
//             comm,
//         };

//         game.comm.tx(Event::Init {
//             game_id: game.game_id,
//         });

//         // Ensure no duplicate players
//         for player in players {
//             if game.players.check(player.user_id).is_err() {
//                 game.players.push(player.to_owned());
//             }
//         }

//         game
//     }
// }

// pub fn get_players_that(
//     players: &Players,
//     f: impl Fn((Pidx, &Player)) -> bool,
// ) -> impl Iterator<Item = (Pidx, &Player)> {
//     players.iter().enumerate().filter(move |(i, p)| f((*i, p)))
// }
// impl Game {
//     // TODO: Custom error?
//     // Handle if directory doesn't exist?
//     pub fn save_game(&self, fname: &str) -> Result<(), ()> {
//         let mut f = File::create(fname).map_err(|_| ())?;
//         serde_json::to_writer_pretty(&mut f, &self).map_err(|_| ())?;
//         Ok(())
//     }

//     pub fn start(&mut self) -> Result<(), ()> {
//         match self.phase {
//             Phase::Init => {}
//             _ => return Err(()),
//         }
//         if self.players.len() < 3 {
//             // self.comm.tx(Event::InvalidCommand(
//             //     "Can't start game with less than 3 players".to_string(),
//             // ));
//             return Err(());
//         }
//         if check_team_numbers(&self.players).is_some() {
//             // self.comm.tx(Event::InvalidCommand(
//             //     "Can't start game with given roles".to_string(),
//             // ));
//             return Err(());
//         }
//         let next_phase = match self.players.len() % 2 == 0 {
//             true => Phase::new_night(1),
//             false => Phase::new_day(1, Vec::new()),
//         };
//         self.comm.tx(Event::Start {
//             players: self.players.clone(),
//             contracts: self.contracts.clone(),
//             phase: next_phase.kind(),
//         });
//         self.phase.next_phase(next_phase, &self.players, &self.comm);
//         Ok(())
//     }

//     pub fn handle(&mut self, cmd: Action) -> Result<(), InvalidActionError> {
//         let result = match cmd {
//             Action::Vote { voter, ballot } => self.handle_vote(voter, ballot),
//             Action::Reveal { celeb } => self.handle_reveal(celeb),
//             Action::Target { actor, target } => self.handle_target(actor, target),
//             Action::Mark { killer, mark } => self.handle_mark(killer, mark),
//         };

//         // if let SaveStrategy::PerChange(fname) = &self.comm.save {
//         //     self.save_game(fname).expect("Saving game should work");
//         // };
//         result
//     }

//     fn handle_vote(&mut self, v: PID, c: Option<Choice>) -> Result<(), InvalidActionError> {
//         let day = self.phase.is_day()?;
//         let voter = self.players.check(v)?;
//         let choice = match c {
//             Some(Choice::Player(p)) => Some(Ballot::Player(self.players.check(p)?)),
//             Some(Choice::Abstain) => Some(Ballot::Abstain),
//             None => None,
//         };

//         // accept vote?
//         let day_resolution = day.resolve_vote(&self.players, voter, choice, &self.comm);

//         let next_phase: Phase = match day_resolution {
//             Some(DayResolution::Elected(elected, _electors, hammer, next_phase)) => {
//                 self.check_elect_contract(self.players[elected].user_id);
//                 self.eliminate(&[elected], hammer).unwrap_or(next_phase)
//             }
//             Some(DayResolution::NoKill(next_phase)) => next_phase,
//             None => return Ok(()),
//         };

//         self.phase.next_phase(next_phase, &self.players, &self.comm);
//         Ok(())
//     }

//     fn check_elect_contract(&mut self, elected: PID) {
//         for contract in &mut self.contracts {
//             if contract.get_charge() == elected {
//                 contract.charge_elected(&self.comm);
//             }
//         }
//     }

//     fn handle_reveal(&mut self, celeb: PID) -> Result<(), InvalidActionError> {
//         let day = self.phase.is_day()?;
//         let celeb = self.players.check(celeb)?;
//         if self.players[celeb].role != Role::CELEB {
//             return Err(InvalidActionError::InvalidRole {
//                 role: self.players[celeb].role.to_owned(),
//                 action: ActionKind::Reveal,
//             });
//         }

//         if day.blocked.contains(&celeb) {
//             self.comm.tx(Event::Block {
//                 blocked: self.players[celeb].to_owned(),
//             });
//             return Ok(());
//         }
//         self.comm.tx(Event::Reveal {
//             celeb: self.players[celeb].to_owned(),
//         });
//         Ok(())
//     }

//     fn handle_target(&mut self, a: PID, t: Choice) -> Result<(), InvalidActionError> {
//         let night = self.phase.is_night()?;
//         let actor = self.players.check(a)?;
//         let target = match t {
//             Choice::Player(p) => Choice::Player(self.players.check(p)?),
//             Choice::Abstain => Choice::Abstain,
//         };

//         let role = self.players[actor].role.to_owned();

//         let night_resolution = night.resolve_target(&self.players, actor, target, role, &self.comm);

//         self.handle_dawn(night_resolution);

//         Ok(())
//     }

//     fn handle_mark(&mut self, killer: PID, mark: Choice) -> Result<(), InvalidActionError> {
//         let night = self.phase.is_night()?;
//         let killer = self.players.check(killer)?;
//         let mut mark = match mark {
//             Choice::Player(p) => Choice::Player(self.players.check(p)?),
//             Choice::Abstain => Choice::Abstain,
//         };
//         let role = self.players[killer].role.to_owned();

//         match role {
//             Role::GOON => {
//                 mark = Choice::Abstain;
//             }
//             _ if role.team() == Team::Mafia => {}
//             _ => {
//                 return Err(InvalidActionError::InvalidRole {
//                     role,
//                     action: ActionKind::Mark,
//                 });
//             }
//         };

//         let night_resolution = night.resolve_mark(&self.players, killer, mark, &self.comm);

//         self.handle_dawn(night_resolution);

//         Ok(())
//     }

//     fn handle_dawn(&mut self, night_resolution: Option<NightResolution>) {
//         let next_phase = match night_resolution {
//             Some(NightResolution::Kill(killer, mark, phase)) => {
//                 self.eliminate(&[mark], killer).unwrap_or(phase)
//             }
//             Some(NightResolution::NoKill(phase)) => phase,
//             None => return,
//         };

//         self.phase.next_phase(next_phase, &self.players, &self.comm);
//     }

//     pub fn eliminate(&mut self, to_die: &[Pidx], proxy: Pidx) -> Option<Phase> {
//         let mut to_die = to_die.to_owned();
//         to_die.sort();

//         let mut to_die_ids = Vec::new();
//         let proxy_id = self.players[proxy].user_id;

//         // Remove from largest to smallest to avoid invalidating indices
//         for p in to_die.into_iter().rev() {
//             let player = self.players[p].to_owned();
//             to_die_ids.push(player.user_id);
//             self.comm.tx(Event::Eliminate { player });

//             self.players.remove(p);
//         }
//         // all Pidxs are now invalid...
//         self.phase.clear();

//         // Check contracts
//         for p_id in to_die_ids {
//             self.check_contracts(p_id, proxy_id)
//         }

//         let winner = check_team_numbers(&self.players);

//         if let Some(win) = winner {
//             let contract_results: Vec<_> = self.contracts.iter().map(|c| c.check_win()).collect();
//             return Some(Phase::End(win, contract_results));
//         }
//         None
//     }

//     fn check_contracts(&mut self, died: PID, proxy: PID) {
//         for contract in &mut self.contracts {
//             if died == contract.get_charge() {
//                 contract.charge_eliminated(&mut self.players, proxy, &self.comm);
//             }
//         }
//     }
// }

// fn check_team_numbers(players: &Players) -> Option<Team> {
//     let n_players = players.len();
//     let n_mafia = players
//         .iter()
//         .filter(|p| p.role.team() == Team::Mafia)
//         .count();

//     if n_mafia == 0 {
//         Some(Team::Town)
//     } else if n_mafia > (n_players - 1) / 2 {
//         Some(Team::Mafia)
//     } else {
//         None
//     }
// }
