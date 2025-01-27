use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::fmt::Debug;
use tokio::time::Duration;

/*
What rules are there?

1. Game function rules, or how the fundamental game mechanics work
    - election timer after a quorum is reached
    - dawn timer after all roles have acted
    - General Phase timer rules
    - Refocus rules for GUARD and AGENT
2. Information rules, or how information is distributed
    - start_roles: What Roles/Teams are known at the start of the game?
        - roles: number of each role kind is known
        - teams: number of each team is known
        - mafia: number of mafia and non-mafia is known
    - What info is revealed upon an election?
    - What info is revealed upon a kill?
    - What info is revealed upon a block?
    - What info is revealed upon a save?
    - What info is revealed upon an investigation?
3. Rolegen
    - What roles are available to the rolegen engine?
    - Other Input parameters for the rolegen engine

*/

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Rules {
    pub timer_rules: TimerRules,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerRules {
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub election_imminent_time: Duration,
    #[serde(default)]
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub dawn_imminent_time: Duration,
}

impl Default for TimerRules {
    fn default() -> Self {
        Self {
            election_imminent_time: Duration::from_secs_f64(10.0),
            dawn_imminent_time: Duration::from_secs_f64(10.0),
        }
    }
}

impl Rules {
    pub fn test() -> Self {
        let mut timer_rules = TimerRules::default();
        timer_rules.election_imminent_time = Duration::from_secs_f64(0.2);
        timer_rules.dawn_imminent_time = Duration::from_secs_f64(0.2);
        Self { timer_rules }
    }
}

mod test {

    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_rules_toml_serde() {
        let rules = Rules::default();
        let toml_str = toml::to_string_pretty(&rules).unwrap();

        println!("{}", toml_str);

        // Declare string to give to toml parser
        let toml_str = r#"
        [timer_rules]
        election_imminent_time = 10.0
        dawn_imminent_time = 10.0
        "#;

        let rules: Rules = toml::from_str(toml_str).unwrap();
        println!("{:?}", rules);
    }
}
