use crate::prelude::*;

use rand::{
    distributions::{Distribution, Open01, Standard},
    rngs::ThreadRng,
    seq::SliceRandom,
    Rng, RngCore,
};
use serde::de;
use statrs::distribution::Poisson;

use super::{GenRole, RoleGen};

const MIN_P: f64 = 0.2;
const MAX_P: f64 = 0.5;

const TARGET_P: f64 = 0.35;

const ROGUES: [RoleKind; 4] =
    [RoleKind::IDIOT, RoleKind::SURVIVOR, RoleKind::GUARD, RoleKind::AGENT];
const POWERS: [RoleKind; 5] =
    [RoleKind::COP, RoleKind::DOCTOR, RoleKind::CELEB, RoleKind::MILKY, RoleKind::STRIPPER];
const MISLEAD: [RoleKind; 2] = [RoleKind::MILLER, RoleKind::GODFATHER];

const BASE_POWER_HIGH_P: [(Adjustment, f64); 4] = [
    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::DOCTOR)), 1.0),
    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::COP)), 1.0),
    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::CELEB)), 0.75),
    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::MILKY)), 0.25),
];

const BASE_POWER_LOW_P: [(Adjustment, f64); 3] = [
    (Adjustment::AddMafia, 0.5),
    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::MAFIA)), 0.5),
    (Adjustment::ReplaceMAFIA(TempGenRole::Role(RoleKind::STRIPPER)), 0.5),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TempGenRole {
    Role(RoleKind),
    GuardCharged(Team),
    AgentCharged(Team),
}

impl TempGenRole {
    fn from_kind(kind: RoleKind) -> Self {
        Self::Role(kind)
    }
    fn from_charge(kind: RoleKind, team: Team) -> Self {
        match kind {
            RoleKind::GUARD => Self::GuardCharged(team),
            RoleKind::AGENT => Self::AgentCharged(team),
            _ => panic!("Cannot charge non-chargable role"),
        }
    }
    pub fn kind(&self) -> RoleKind {
        match self {
            Self::Role(kind) => *kind,
            Self::GuardCharged(_) => RoleKind::GUARD,
            Self::AgentCharged(_) => RoleKind::AGENT,
        }
    }

    // Difference in power from a common TOWN? which would be -0.5?
    // Positive is pro-mafia, negative is pro-town
    // This number is subtracted from sigma when the role is substituted for TOWN
    fn sigma_adj(&self, n: usize) -> f64 {
        match self {
            Self::Role(RoleKind::TOWN) => 0.0,
            Self::Role(RoleKind::IDIOT) => 0.75,
            Self::Role(RoleKind::SURVIVOR) => 0.25,
            Self::AgentCharged(Team::Town) => 0.25,
            Self::AgentCharged(Team::Mafia) => -0.75,
            Self::GuardCharged(Team::Town) => -0.25,
            Self::GuardCharged(Team::Mafia) => 0.75,
            Self::Role(RoleKind::COP) => -0.5,
            Self::Role(RoleKind::DOCTOR) => -0.5,
            Self::Role(RoleKind::CELEB) => -0.5,
            Self::Role(RoleKind::MILKY) => -0.25,
            Self::Role(RoleKind::STRIPPER) => 1.5,
            Self::Role(RoleKind::GODFATHER) => 1.0,
            Self::Role(RoleKind::MILLER) => 0.0,
            Self::Role(RoleKind::MAFIA) => 1.0,
            _ => {
                warn!("No sigma adjustment for observed {:?}", self);
                0.0
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Adjustment {
    ReplaceTOWN(TempGenRole),
    ReplaceMAFIA(TempGenRole),
    AddMafia,
}

fn p(n_: usize, m_: usize) -> f64 {
    let m = m_ as f64;
    return expected_p(n_, m);
}

fn expected_p(n_: usize, sigma: f64) -> f64 {
    let n = n_ as f64;
    return 2.0 * sigma / (n + 1.0);
}

fn get_sigma_maf(n_: usize, rng: &mut impl Rng) -> f64 {
    let x_0 = TARGET_P / 2.0 * (n_ as f64 + 1.0);
    let sigma = sigmoid_rv(x_0, rng);
    return sigma;
}
fn sigmoid_rv(x_0: f64, rng: &mut impl Rng) -> f64 {
    let x: f64 = rng.sample(Open01);
    let s = (-f64::ln(1.0 / x - 1.0) / 10.0 + 1.0) * x_0;
    return s;
}

fn get_n_rogue(n: usize, rng: &mut impl Rng) -> isize {
    let rate = n as f64 / 25.0;
    let poisson = Poisson::new(rate).unwrap();
    let n_rogue: u64 = poisson.sample(rng);
    return n_rogue as isize;
}

fn get_n_power(n: usize, rng: &mut impl Rng) -> isize {
    let rate = n as f64 * 3.0 / 10.0;
    let poisson = Poisson::new(rate).unwrap();
    let n_power: u64 = poisson.sample(rng);
    debug!("n_power: {:?}", n_power);
    return n_power as isize;
}

// TODO: construct these based on allowed_roles
fn get_rogue_adjust(e_p: f64, rng: &mut impl Rng) -> Adjustment {
    let choices = if e_p > TARGET_P {
        vec![
            (Adjustment::ReplaceMAFIA(TempGenRole::Role(RoleKind::IDIOT)), 0.75),
            (Adjustment::ReplaceTOWN(TempGenRole::GuardCharged(Team::Town)), 0.20),
            (Adjustment::ReplaceTOWN(TempGenRole::AgentCharged(Team::Mafia)), 0.05),
        ]
    } else {
        vec![
            (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::IDIOT)), 0.70),
            (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::SURVIVOR)), 0.05),
            (Adjustment::ReplaceTOWN(TempGenRole::GuardCharged(Team::Town)), 0.05),
            (Adjustment::ReplaceTOWN(TempGenRole::AgentCharged(Team::Town)), 0.20),
        ]
    };
    return choices.choose_weighted(rng, |x| x.1).unwrap().0;
}

fn get_power_adjust(
    e_p: f64,
    odds_high_p: impl IntoIterator<Item = (Adjustment, f64)>,
    odds_low_p: impl IntoIterator<Item = (Adjustment, f64)>,
    rng: &mut impl Rng,
) -> Adjustment {
    let odds_high_p = odds_high_p.into_iter().collect::<Vec<_>>();
    let odds_low_p = odds_low_p.into_iter().collect::<Vec<_>>();
    debug!("odds: {:?}, {:?}", odds_high_p, odds_low_p);
    let choices = if e_p > TARGET_P { odds_high_p } else { odds_low_p };
    return choices.choose_weighted(rng, |x| x.1).unwrap().0;
}

pub struct StandardRoleGen<'a, R> {
    n: usize,
    rules: Rules,
    sigma: f64,
    roles: Vec<TempGenRole>,
    rng: &'a mut R,
}

impl<'a, R> std::fmt::Debug for StandardRoleGen<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardRoleGen")
            .field("n", &self.n)
            .field("sigma", &self.sigma)
            .field("roles", &self.roles)
            .finish()
    }
}

impl<'a, R: Rng> StandardRoleGen<'a, R> {
    fn new(n: usize, rules: Rules, rng: &'a mut R) -> Self {
        let mut s = StandardRoleGen { n, rules, sigma: 0.0, roles: Vec::new(), rng };
        s.initialize();
        s
    }

    fn initialize(&mut self) {
        self.sigma = get_sigma_maf(self.n, &mut self.rng);
        debug!("Initial sigma: {:?}", self.sigma);
        self.roles = vec![TempGenRole::from_kind(RoleKind::TOWN); self.n];
        while self.sigma > 0.5 {
            self.roles.pop();
            self.roles.insert(0, TempGenRole::from_kind(RoleKind::MAFIA));
            self.sigma -= 1.0;
        }
        debug!("Initial roles: {:?}", self.roles);
    }

    fn add_rogue(&mut self) {
        let mut n_rogue = get_n_rogue(self.n, &mut self.rng);
        // First check for guaranteed roles
        for role in ROGUES.iter() {
            if let Some(n_role) = self.rules.guaranteed_roles.get(role) {
                for _ in 0..*n_role {
                    if role.team() == Team::Mafia {
                        self.apply(Adjustment::ReplaceMAFIA(TempGenRole::from_kind(*role)));
                    } else {
                        self.apply(Adjustment::ReplaceTOWN(TempGenRole::from_kind(*role)));
                    }
                    n_rogue -= 1;
                }
            }
        }
        for _ in 0..n_rogue {
            let e_p = self.get_e_p();
            let adjust = get_rogue_adjust(e_p, &mut self.rng);
            self.apply(adjust);
        }
    }

    fn add_power(&mut self) {
        let mut n_power = get_n_power(self.n, &mut self.rng);
        for role in POWERS.iter() {
            if let Some(n_role) = self.rules.guaranteed_roles.get(role) {
                for _ in 0..*n_role {
                    if role.team() == Team::Mafia {
                        self.apply(Adjustment::ReplaceMAFIA(TempGenRole::from_kind(*role)));
                    } else {
                        self.apply(Adjustment::ReplaceTOWN(TempGenRole::from_kind(*role)));
                    }
                    n_power -= 1;
                }
            }
        }
        for _ in 0..n_power {
            let e_p = self.get_e_p();
            let (odds_hi, odds_lo) = self.get_power_odds();
            let adjust = get_power_adjust(e_p, odds_hi, odds_lo, &mut self.rng);
            self.apply(adjust);
        }
    }

    fn get_power_odds(&self) -> (HashMap<Adjustment, f64>, HashMap<Adjustment, f64>) {
        let mut odds_high_p = BASE_POWER_HIGH_P.iter().copied().collect::<HashMap<_, _>>();
        let mut odds_low_p = BASE_POWER_HIGH_P.iter().copied().collect::<HashMap<_, _>>();

        for role in self.roles.iter() {
            match role {
                TempGenRole::Role(rk)
                    if odds_high_p
                        .contains_key(&Adjustment::ReplaceTOWN(TempGenRole::Role(*rk))) =>
                {
                    odds_high_p
                        .entry(Adjustment::ReplaceTOWN(TempGenRole::Role(*rk)))
                        .and_modify(|e| *e *= 0.5);
                }

                TempGenRole::Role(RoleKind::STRIPPER) => {
                    odds_low_p
                        .entry(Adjustment::ReplaceMAFIA(TempGenRole::Role(RoleKind::STRIPPER)))
                        .and_modify(|e| *e *= 0.5);
                    odds_low_p.entry(Adjustment::AddMafia).and_modify(|e| *e *= 0.75);
                }
                TempGenRole::Role(RoleKind::MAFIA) => {
                    odds_low_p.entry(Adjustment::AddMafia).and_modify(|e| *e *= 0.75);
                    odds_low_p
                        .entry(Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::MAFIA)))
                        .and_modify(|e| *e *= 0.5);
                }

                _ => {}
            }
        }

        return (odds_high_p, odds_low_p);
    }

    fn add_mislead(&mut self) {
        let x: f64 = self.rng.sample(Open01);
        if x < (self.rules.mislead as f64 / 100.0) {
            let mut n_mislead: u64 =
                self.rng.sample::<u64, _>(Poisson::new(self.n as f64 / 20.0).unwrap());
            if n_mislead == 0 {
                n_mislead = 1;
            }
            for role in MISLEAD.iter() {
                for _ in 0..n_mislead {
                    if role.team() == Team::Mafia {
                        self.apply(Adjustment::ReplaceMAFIA(TempGenRole::from_kind(*role)));
                    } else {
                        self.apply(Adjustment::ReplaceTOWN(TempGenRole::from_kind(*role)));
                    }
                    n_mislead -= 1;
                }
            }
            for _ in 0..n_mislead {
                let n_town =
                    self.roles.iter().filter(|r| r.kind() == RoleKind::TOWN).count() as f64;
                let choices = [
                    (Adjustment::ReplaceTOWN(TempGenRole::Role(RoleKind::MILLER)), n_town),
                    (Adjustment::ReplaceMAFIA(TempGenRole::Role(RoleKind::GODFATHER)), self.sigma),
                ];
                let (adj, _) = choices.choose_weighted(&mut self.rng, |x| x.1).unwrap();
                self.apply(*adj);
            }
        }
    }

    fn balance_maf(&mut self) -> f64 {
        for _ in 0..5 {
            if self.get_e_p() < MIN_P {
                debug!("Adding Mafia at finish");
                self.apply(Adjustment::ReplaceTOWN(TempGenRole::from_kind(RoleKind::MAFIA)));
            }
        }
        for _ in 0..5 {
            if self.get_e_p() > MAX_P {
                debug!("Adding Town at finish");
                self.apply(Adjustment::ReplaceMAFIA(TempGenRole::from_kind(RoleKind::TOWN)));
            }
        }
        self.sigma = 0.0;
        debug!("After finishing: adj_sigma: {:?}, e_p: {:?}", self.get_adj_sigma(), self.get_e_p());
        self.get_e_p()
    }

    fn find_team(&mut self, team: Team) -> usize {
        let idxs: Vec<usize> = self
            .roles
            .iter()
            .enumerate()
            .filter_map(|(i, r)| (r.kind().team() == team).then_some(i))
            .collect();
        match idxs.choose(self.rng) {
            Some(idx) => *idx,
            None => self.find_team_none(),
        }
    }

    fn find_team_none(&mut self) -> usize {
        let idxs: Vec<usize> = (0..self.roles.len()).collect();
        *idxs.choose(self.rng).unwrap()
    }

    fn assign_charges(&mut self) -> Vec<GenRole> {
        self.roles.shuffle(self.rng);
        let role_copy = self.roles.clone();
        let mut gen_roles = Vec::new();
        for role in role_copy.into_iter() {
            match role {
                TempGenRole::GuardCharged(team) => {
                    gen_roles.push(GenRole::GuardCharged(self.find_team(team)));
                }
                TempGenRole::Role(RoleKind::GUARD) => {
                    gen_roles.push(GenRole::GuardCharged(self.find_team_none()));
                }
                TempGenRole::AgentCharged(team) => {
                    gen_roles.push(GenRole::GuardCharged(self.find_team(team)));
                }
                TempGenRole::Role(RoleKind::AGENT) => {
                    gen_roles.push(GenRole::GuardCharged(self.find_team_none()));
                }
                TempGenRole::Role(rk) => {
                    gen_roles.push(GenRole::Role(rk));
                }
            }
        }
        return gen_roles;
    }

    fn get_adj_sigma(&self) -> f64 {
        let mut sigma = self.sigma;
        for role in self.roles.iter() {
            sigma += role.sigma_adj(self.n);
        }
        return sigma;
    }

    fn get_e_p(&self) -> f64 {
        let sigma = self.get_adj_sigma();
        return expected_p(self.n, sigma);
    }

    fn apply(&mut self, adjust: Adjustment) {
        debug!(
            "Before adjustment: sigma: {:?}, adj_sigma: {:?} e_p: {:?}, {:?}",
            self.sigma,
            self.get_adj_sigma(),
            self.get_e_p(),
            self.roles
        );
        debug!("Applying adjustment: {:?}", adjust);
        match adjust {
            Adjustment::ReplaceTOWN(role) => {
                // remove the first TOWN role if it exists
                for i in 0..self.roles.len() {
                    if self.roles[i].kind() == RoleKind::TOWN {
                        self.roles[i] = role;
                        // self.sigma -= role.sigma_adj(self.n);
                        break;
                    }
                }
            }
            Adjustment::ReplaceMAFIA(role) => {
                for i in 0..self.roles.len() {
                    if self.roles[i].kind() == RoleKind::MAFIA {
                        self.roles[i] = role;
                        // self.sigma -= role.sigma_adj(self.n);
                        break;
                    }
                }
            }
            Adjustment::AddMafia => {
                // make sigma lower, so more mafia are added later
                self.sigma -= 0.5;
            }
        }
    }

    fn add_role(&mut self, role: TempGenRole) {
        self.roles.insert(0, role);
        self.sigma -= role.sigma_adj(self.n);
    }

    fn pop_role(&mut self) -> TempGenRole {
        let role = self.roles.pop().unwrap();
        self.sigma += role.sigma_adj(self.n);
        return role;
    }
}

impl<'a, R: Rng> RoleGen for StandardRoleGen<'a, R> {
    type RNG = R;
    fn generate_roles(n: usize, rules: &Rules, rng: &mut Self::RNG) -> Vec<GenRole> {
        let mut gen = StandardRoleGen::new(n, rules.clone(), rng);
        let mut i = 0;
        loop {
            gen.initialize();
            // info!("After choosing Sigma: {:?}", gen);
            gen.add_rogue();
            // info!("After choosing Rogue: {:?}", gen);
            gen.add_power();
            // info!("After choosing Power: {:?}", gen);
            gen.add_mislead();
            // info!("After choosing Mislead: {:?}", gen);
            let final_e_p = gen.balance_maf();
            if (final_e_p >= MIN_P && final_e_p <= MAX_P) || i >= 15 {
                gen.roles.shuffle(gen.rng);
                return gen.assign_charges();
            }
            debug!("Re-rolling... {:?}", gen);
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::Distribution;

    use statrs::function::gamma::gamma;

    use super::*;

    use super::super::assign_roles;

    fn poisson_pdf(x: f64, lambda: f64) -> f64 {
        return lambda.powf(x) * (-lambda).exp() / gamma(x + 1.0) as f64;
    }

    #[test]
    fn poisson() {
        let n = 11.0;
        let lambda = n / 25.0;
        println!("Lambda = {}", lambda);
        let max = 20;
        let end = 3;
        for i in 0..max {
            let x = i as f64 / max as f64 * end as f64;
            let p = poisson_pdf(x, lambda);
            println!("Poisson({:.2}) = {:.3}", x, p);
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn try_std_role_gen() {
        let mut rules = Rules::default();
        // rules.guaranteed_roles.remove(&RoleKind::COP);
        // rules.guaranteed_roles.remove(&RoleKind::DOCTOR);

        let mut rng = rand::thread_rng();
        let n = 7;
        let roles = StandardRoleGen::generate_roles(n, &rules, &mut rng);
        let users = (0..n).map(|i| Pid::from(i as u64)).collect::<Vec<_>>();
        let registry = assign_roles(users, roles, &mut rng);

        println!("{:?}", registry);
    }
}
