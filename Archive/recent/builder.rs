// use std::marker::PhantomData;

// use crate::engine::state::{role::Role, rules::Rules};

// use super::*;
// use anyhow::Result;
// use tokio::sync::{broadcast, mpsc};

// impl Game {
//     fn builder() -> GameBuilder<(), (), (), ()> {
//         GameBuilder::default()
//     }
// }

// struct Users(Vec<u64>);
// struct Roles(Vec<Role>);
// struct Other;
// struct State_(State);

// impl From<()> for Users {
//     fn from(_: ()) -> Self {
//         Users(Vec::new())
//     }
// }
// impl From<()> for Roles {
//     fn from(_: ()) -> Self {
//         Roles(Vec::new())
//     }
// }
// impl From<()> for Other {
//     fn from(_: ()) -> Self {
//         Other
//     }
// }
// impl From<()> for State_ {
//     fn from(_: ()) -> Self {
//         State_(State::default())
//     }
// }

// #[derive(Debug, Default)]
// struct GameBuilder<U, R, O, S> {
//     users: U,
//     roles: R,
//     state: S,
//     id: Option<u64>,
//     rules: Option<Rules>,
//     event_tx: Option<EventTx>,
//     actions: Option<(ActionTx, ActionRx)>,
//     _o: PhantomData<O>,
// }

// impl<U, R, O, S> GameBuilder<U, R, O, S> {
//     fn to<U2, R2, O2, S2>(self: GameBuilder<U, R, O, S>) -> GameBuilder<U2, R2, O2, S2>
//     where
//         U2: From<U>,
//         R2: From<R>,
//         O2: From<O>,
//         S2: From<S>,
//     {
//         GameBuilder::<U2, R2, O2, S2> {
//             users: self.users.into(),
//             roles: self.roles.into(),
//             state: self.state.into(),
//             id: self.id,
//             rules: self.rules,
//             event_tx: self.event_tx,
//             actions: self.actions,
//             _o: PhantomData::<O2>,
//         }
//     }
// }

// impl<R, O> GameBuilder<(), R, O, ()> {
//     fn users(self, users: impl IntoIterator<Item = u64>) -> GameBuilder<Users, R, O, ()> {
//         let mut s = self.to();
//         s.users = Users(users.into_iter().collect());
//         s
//     }
// }
// impl<U, O> GameBuilder<U, (), O, ()> {
//     fn roles(self, roles: impl IntoIterator<Item = Role>) -> GameBuilder<U, Roles, O, ()> {
//         let mut s = self.to();
//         s.roles = Roles(roles.into_iter().collect());
//         s
//     }
// }
// impl<O> GameBuilder<(), (), O, ()> {
//     fn registry(
//         self,
//         registry: impl IntoIterator<Item = (u64, Role)>,
//     ) -> GameBuilder<Users, Roles, O, ()> {
//         let (users, roles): (Vec<_>, Vec<_>) = registry.into_iter().unzip();
//         self.users(users).roles(roles)
//     }
// }
// impl<U, R, O> GameBuilder<U, R, O, ()>
// where
//     Other: From<O>,
// {
//     fn id(self, id: u64) -> GameBuilder<U, R, Other, ()> {
//         let mut s = self.to();
//         s.id = Some(id);
//         s
//     }
//     fn rules(self, rules: Rules) -> GameBuilder<U, R, Other, ()> {
//         let mut s = self.to();
//         s.rules = Some(rules);
//         s
//     }
// }
// impl GameBuilder<(), (), (), ()> {
//     fn state(self, state: State) -> GameBuilder<(), (), (), State_> {
//         let mut s = self.to();
//         s.state = State_(state);
//         s
//     }
// }

// impl<U, R, O, S> GameBuilder<U, R, O, S> {
//     fn event_tx(mut self, event_tx: EventTx) -> Self {
//         self.event_tx = Some(event_tx);
//         self
//     }
//     fn actions(mut self, actions: (ActionTx, ActionRx)) -> Self {
//         self.actions = Some(actions);
//         self
//     }
// }

// use super::State;

// impl<O> GameBuilder<Users, Roles, O, ()> {
//     fn build(self) -> Arc<Game> {
//         let (a_tx, a_rx) = self.actions.unwrap_or_else(|| mpsc::channel(100));
//         let e_tx = self.event_tx.unwrap_or_else(|| broadcast::channel(100).0);
//         let id = self.id.unwrap_or_default(); // TODO generate id?
//         let registry: Vec<_> = self.users.0.into_iter().zip(self.roles.0).collect();
//         let rules = self.rules.unwrap_or_default();
//         let state = State::new(id, &registry, rules);
//         Game::new(state, (a_tx, a_rx), e_tx)
//     }
// }
// impl GameBuilder<(), (), (), State_> {
//     fn build(self) -> Arc<Game> {
//         let (a_tx, a_rx) = self.actions.unwrap_or_else(|| mpsc::channel(100));
//         let e_tx = self.event_tx.unwrap_or_else(|| broadcast::channel(100).0);
//         let state = self.state.0;
//         Game::new(state, (a_tx, a_rx), e_tx)
//     }
// }

// fn test() {
//     let game = Game::builder()
//         .users(vec![1, 2, 3, 4])
//         .event_tx(broadcast::channel(100).0)
//         .roles(vec![Role::TOWN, Role::COP, Role::DOCTOR, Role::MAFIA])
//         .build();

//     let s = State::default();
//     let g2 = Game::builder().state(s).build();
// }

// #[cfg(test)]
// mod test {}
