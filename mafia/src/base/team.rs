use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

impl From<Role> for Team {
    fn from(role: Role) -> Self {
        Team::from(role.kind())
    }
}

impl From<RoleKind> for Team {
    fn from(role: RoleKind) -> Self {
        use RoleKind::*;
        match role {
            TOWN | COP | DOCTOR | CELEB | MILKY | MILLER => Self::Town,
            MAFIA | STRIPPER | GODFATHER | GOON => Self::Mafia,
            IDIOT | SURVIVOR | GUARD | AGENT => Self::Rogue,
        }
    }
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Team::Town => write!(f, "Town"),
            Team::Mafia => write!(f, "Mafia"),
            Team::Rogue => write!(f, "Rogue"),
        }
    }
}

impl Role {
    fn description(&self) -> &'static str {
        use Role::*;
        match self {
            TOWN => {
                "The TOWN is a normal player in this game, the last line of defense against \
                the mafia scum. They sniff out who the mafia are and convince their fellow town \
                members to kill them during the day!"
            }
            COP => {
                "The COP can investigate Mafia. They investigate another player at Night and \
                learn if that player is Mafia"
            }
            DOCTOR => {
                "The DOCTOR's job is to save the townspeople from the Mafia's attacks. If you \
                target the same player the Mafia targets at Night, they will not die!"
            }
            CELEB => {
                "The CELEB is a celebrity. Everybody knows who they are, but everyone doesn't \
                recognize them right now. CELEB can authoritatively reveal their role during Day \
                by sending MODERATOR '/reveal'"
            }
            MILLER => {
                "The MILLER is pretty sus but they are actually on the side of Town... \
                If the cop investigates them, they show up as MAFIA..."
            }
            MILKY => {
                "The MILKY delivers milk in the morning. Pick a player to give milk to each Night"
            }
            MAFIA => {
                "The MAFIA is a basic Mafia Team member. During the Day, they try not to get \
                killed. During the Night, they choose somebody to kill!"
            }
            GODFATHER => {
                "The GODFATHER is a Mafia member who, when investigated, shows up as Not Mafia"
            }
            STRIPPER => {
                "The STRIPPER is a Mafia member who can distract another player each Night. If \
                that player tries to use an ability (targeting or revealing), they will be unable \
                to! They cannot perform both the distract action and the kill action in one night"
            }
            GOON => {
                "D'oh! The GOON is a member of the Mafia that cannot help target another player in \
                the mafia chat at night. They can /target none but cannot target another player..."
            }
            IDIOT => {
                "The IDIOT's goal is to get elected to die! If they are elected, they get \
                vengeance by picking one player who voted for them to die with them"
            }
            SURVIVOR => {
                "The SURVIVOR's goal is to get to the end of the game without dying. That's it!"
            }
            GUARD(_) => {
                "The GUARD is tasked with protecting a charge. You win if that player survives \
                until the end of the game."
            }
            AGENT(_) => {
                "The AGENT is tasked with inviting the death of a charge. Whether by election or \
                by directing murder, you win if your charge dies."
            }
        }
    }
}

impl Team {
    fn description(&self) -> &'static str {
        use Team::*;
        match self {
            Town => {
                "The Town's goal as a Team is to eliminate all Mafia players. Mostly by voting \
                them out during the Day"
            }
            Mafia => {
                "The Mafia's goal as a Team is to eliminate enough non-Mafia players that the \
                non-Mafia players cannot form a majority. They can target to kill one player \
                during the Night in the Mafia Chat"
            }
            Rogue => {
                "The Rogue Aligned players have no Team allegiances. Their goals vary and \
                their win conditions are separate from the contest between Town and Mafia"
            }
        }
    }
}
