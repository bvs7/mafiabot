use crate::prelude::*;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub enum Act {
    Block,
    Save {
        /// Effective if Save targeted the same player as a Kill
        effective: bool,
    },
    Kill {
        saviors: Vec<Pid>,
    },
    Investigate,
    Milk,
}

impl Act {
    fn priority(&self) -> i32 {
        use Act::*;
        match self {
            Block => 2,
            Save { .. } => 1,
            Kill { .. } => 0,
            Investigate => -1,
            Milk => -2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NightAct {
    pub act: Act,
    pub actor: Pid,
    pub target: Pid,
    pub blockers: Vec<Pid>,
}

impl NightAct {
    // Assume role is a targeting role...
    pub fn from_target(role: Role, actor: Pid, target: Pid) -> Self {
        tracing::debug!("Night action from target: {:?}, {:?}, {:?}", role, actor, target);
        use Role::*;
        match role {
            STRIPPER => NightAct { act: Act::Block, actor, target, blockers: vec![] },
            DOCTOR => {
                NightAct { act: Act::Save { effective: false }, actor, target, blockers: vec![] }
            }
            COP => NightAct { act: Act::Investigate, actor, target, blockers: vec![] },
            MILKY => NightAct { act: Act::Milk, actor, target, blockers: vec![] },
            TOWN | CELEB | MILLER | MAFIA | GOON | GODFATHER | IDIOT | SURVIVOR | AGENT(_)
            | GUARD(_) => panic!("Expected a targeting role"),
        }
    }

    pub fn block(&mut self, blocker: Pid) {
        tracing::debug!("{:?} blocking {:?}", blocker, self);
        self.blockers.push(blocker);
    }

    pub fn save(&mut self, savior: Pid) {
        tracing::debug!("{:?} saving from {:?}", savior, self);
        match &mut self.act {
            Act::Kill { saviors, .. } => saviors.push(savior),
            _ => {}
        }
    }

    pub fn from_state(state: &State) -> Vec<Self> {
        let players = &state.players;
        let Phase::Night { targets, scheme, .. } = &state.phase else {
            panic!("To night actions during not night");
        };
        let mut night_actions: Vec<NightAct> = targets
            .into_iter()
            .flat_map(|(a, t)| t.map(|t| NightAct::from_target(players.get_role(*a), *a, t)))
            .collect();
        if let Some((killer, Some(target))) = scheme {
            night_actions.push(NightAct {
                act: Act::Kill { saviors: vec![] },
                actor: *killer,
                target: *target,
                blockers: vec![],
            })
        };

        // Compare enums, prioritized by order of NightAction
        // (shuffle before to ensure no ordering to things like milking)
        night_actions.shuffle(&mut rand::thread_rng());
        night_actions.sort_by(|a, b| b.act.priority().cmp(&a.act.priority()));

        for i in 0..night_actions.len() {
            let (earlier, rest) = night_actions.split_at_mut(i);
            let cur = &mut rest[0];
            for pre in earlier.iter_mut() {
                if cur.act.priority() >= pre.act.priority() {
                    // Acts with the same priority do not effect each other
                    continue;
                }
                match &mut pre.act {
                    Act::Block if pre.target == cur.actor => cur.block(pre.actor),
                    Act::Save { effective } if pre.target == cur.target => {
                        if matches!(cur.act, Act::Kill { .. }) {
                            *effective = true;
                        }
                        if pre.blockers.is_empty() {
                            cur.save(pre.actor);
                        }
                    }
                    _ => {}
                }
            }
        }
        return night_actions;
    }
}

impl State {
    pub fn apply_night_actions(&mut self, night_actions: Vec<NightAct>) -> Blocks {
        let mut blocks: HashMap<Pid, Vec<Pid>> = HashMap::new();
        let mut kills: HashMap<Pid, Pid> = HashMap::new();
        for na in night_actions {
            match na.act {
                Act::Block => {
                    blocks.entry(na.target).or_default().push(na.actor);
                }
                Act::Save { effective } => {
                    if effective && !na.blockers.is_empty() {
                        self.tx(Event::Block { blocked: na.actor, blockers: na.blockers.clone() });
                    }
                }
                Act::Kill { saviors } => {
                    if saviors.is_empty() {
                        let killer = na.actor;
                        let mark = na.target;
                        kills.insert(mark, killer);
                    } else {
                        self.tx(Event::Save { saved: na.target, saviors: saviors.clone() });
                    }
                }
                Act::Investigate => {
                    // Investigations do not occur if cop is dead
                    if kills.contains_key(&na.actor) {
                        continue;
                    }
                    let role = self.players.get_role(na.target);
                    if na.blockers.is_empty() {
                        self.tx(Event::Investigate {
                            cop: na.actor,
                            target: na.target,
                            appears_mafia: role.is_mafia(),
                        });
                    } else {
                        self.tx(Event::Block { blocked: na.actor, blockers: na.blockers.clone() });
                    }
                }
                Act::Milk => {
                    // Milk delivery does not happen if target is dead
                    if kills.contains_key(&na.target) {
                        continue;
                    }
                    if na.blockers.is_empty() {
                        self.tx(Event::Milk { milky: na.actor, target: na.target });
                    } else if !kills.contains_key(&na.actor) {
                        self.tx(Event::Block { blocked: na.actor, blockers: na.blockers.clone() });
                    }
                }
            }
        }
        if kills.is_empty() {
            self.tx(Event::NoKill);
        }
        for (mark, killer) in kills.into_iter() {
            self.tx(Event::Kill { killer, mark });
            self.kill(mark, killer);
        }
        blocks
    }
}
