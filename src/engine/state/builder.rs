use super::*;
use anyhow::{anyhow, Result};
use rand::{rngs::ThreadRng, seq::SliceRandom};

trait NewRoleSource {
    fn roles(&mut self, n: usize, rng: &mut Option<rand::rngs::ThreadRng>) -> Result<Vec<Role>>;
}

impl std::fmt::Debug for dyn NewRoleSource + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NewRoleSource")
    }
}

#[derive(Debug)]
struct NewRoleSourceList {
    roles: Vec<Role>,
}

impl NewRoleSource for NewRoleSourceList {
    fn roles(&mut self, n: usize, rng: &mut Option<ThreadRng>) -> Result<Vec<Role>> {
        if n > self.roles.len() {
            bail!("Not enough ({n}) roles: {:?}", self.roles);
        }
        if let Some(rng) = rng {
            self.roles.shuffle(rng);
        }
        Ok(self.roles.clone())
    }
}

#[derive(Debug)]
struct NewRoleSourceRoleGen<R: RoleGen> {
    rolegen: R,
}

impl<R: RoleGen> NewRoleSource for NewRoleSourceRoleGen<R> {
    fn roles(&mut self, n: usize, rng: &mut Option<ThreadRng>) -> Result<Vec<Role>> {
        match rng {
            Some(rng) => self.rolegen.role_gen(n, rng),
            None => Err(anyhow!("RNG not provided")),
        }
    }
}

#[derive(Debug)]
struct InnerStateBuilder {
    game_id: Option<u64>,
    users: Option<Vec<u64>>,
    roles: Option<Box<dyn NewRoleSource>>,
    rng: Option<rand::rngs::ThreadRng>,
    rules: Option<Rules>,
}

impl InnerStateBuilder {
    fn new() -> Self {
        Self {
            game_id: None,
            users: None,
            roles: None,
            rng: None,
            rules: None,
        }
    }
    fn with_game_id(&mut self, game_id: u64) -> &mut Self {
        self.game_id = Some(game_id);
        self
    }
    fn with_users(&mut self, users: Vec<u64>) -> &mut Self {
        self.users = Some(users);
        self
    }
    fn with_rng(&mut self, rng: ThreadRng) -> &mut Self {
        self.rng = Some(rng);
        self
    }
    fn with_rules(&mut self, rules: Rules) -> &mut Self {
        self.rules = Some(rules);
        self
    }
    fn with_roles(&mut self, roles: Vec<Role>) -> &mut Self {
        self.roles = Some(Box::new(NewRoleSourceList { roles }));
        self
    }
    fn with_registry(&mut self, registry: impl IntoIterator<Item = (u64, Role)>) -> &mut Self {
        let (users, roles): (Vec<_>, Vec<_>) = registry.into_iter().unzip();
        self.rng = None; // No rng means roles are not shuffled
        self.with_users(users).with_roles(roles)
    }
    fn with_rolegen<R: RoleGen + 'static>(&mut self, rolegen: R) -> &mut Self {
        self.roles = Some(Box::new(NewRoleSourceRoleGen { rolegen }));
        self
    }
    fn build(&mut self) -> Result<InnerState> {
        // TODO: where to get default game_id?
        let game_id = self.game_id.take().unwrap_or(0);
        let users = self
            .users
            .take()
            .ok_or_else(|| anyhow!("no users provided"))?;
        let roles = match &mut self.roles {
            Some(roles) => roles.roles(users.len(), &mut self.rng),
            None => Err(anyhow!("No roles provided")),
        }?;
        let registry: Vec<_> = users.into_iter().zip(roles).collect();
        Ok(InnerState::new(
            game_id,
            &registry,
            self.rules.take().unwrap_or_default(),
        ))
    }
}

#[derive(Debug)]
enum StateSource<'a> {
    NewState(&'a mut InnerStateBuilder),
    FromState(InnerState),
}

/// Builder for State
/// ```
/// use Role::*;
/// let state = StateBuilder::from_inner_builder(
///     InnerStateBuilder::new()
///         .with_game_id(0)
///         .with_users(vec![1, 2, 3, 4])
///         .with_roles(vec![TOWN, COP, DOCTOR, MAFIA])
///         .with_rules(Rules {}),
/// ).build().await?;
/// ```
#[derive(Debug)]
struct StateBuilder<'a> {
    state_source: Option<StateSource<'a>>,
    rng: Option<ThreadRng>,
    event_tx: Option<EventTx>,
    action_channel: Option<(ActionTx, ActionRx)>,
}

impl<'a> Default for StateBuilder<'a> {
    fn default() -> Self {
        Self {
            state_source: None,
            rng: None,
            event_tx: None,
            action_channel: None,
        }
    }
}
impl<'a> StateBuilder<'a> {
    fn from_inner_builder(inner_builder: &'a mut InnerStateBuilder) -> Self {
        Self {
            state_source: Some(StateSource::NewState(inner_builder)),
            ..Self::default()
        }
    }
    fn from_inner_state(inner: InnerState) -> Self {
        Self {
            state_source: Some(StateSource::FromState(inner)),
            ..Self::default()
        }
    }
    fn with_event_tx(&mut self, tx: EventTx) -> &mut Self {
        self.event_tx = Some(tx);
        self
    }
    fn with_action_channel(&mut self, tx: ActionTx, rx: ActionRx) -> &mut Self {
        self.action_channel = Some((tx, rx));
        self
    }
    async fn build(&mut self) -> Result<Arc<State>> {
        let tx = match self.event_tx.take() {
            Some(tx) => tx,
            None => tokio::sync::broadcast::channel(100).0,
        };
        let (a_tx, a_rx) = match self.action_channel.take() {
            Some((tx, rx)) => (tx, rx),
            None => tokio::sync::mpsc::channel(100),
        };
        let inner = match self.state_source.take() {
            Some(StateSource::NewState(inner_builder)) => inner_builder.build(),
            Some(StateSource::FromState(inner)) => Ok(inner),
            None => return Err(anyhow!("No state source provided")),
        }?;
        State::new(inner, a_tx, a_rx, tx).await
    }
}
#[cfg(test)]
mod test {
    use tokio::sync::oneshot;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn build_new() -> Result<()> {
        let registry = vec![
            (1, Role::TOWN),
            (2, Role::COP),
            (3, Role::DOCTOR),
            (4, Role::MAFIA),
        ];
        let inner = InnerStateBuilder::new()
            .with_game_id(0)
            .with_registry(registry)
            .with_rules(Rules {})
            .build()?;
        let state = StateBuilder::from_inner_state(inner).build().await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        // Should be started?
        let a_tx = state.a_tx.clone();
        let mut event_rx = state.tx.subscribe();
        let (responder, response) = oneshot::channel();

        debug!(msg = "Sending start action", ?a_tx);

        a_tx.send((Action::Start, responder)).await?;
        let _ = response.await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let n = event_rx.len();
        debug!(n);
        assert!(event_rx.len() == 2);
        let start = event_rx.recv().await?;
        assert!(matches!(start, Event::Start { .. }));
        debug!(?start);

        state.quit().await;

        Ok(())
    }

    #[test]
    fn build_inner() -> Result<()> {
        assert!(InnerStateBuilder::new().build().is_err());
        assert!(InnerStateBuilder::new()
            .with_users(vec![1, 2, 3, 4])
            .build()
            .is_err());
        assert!(InnerStateBuilder::new()
            .with_users(vec![1, 2, 3, 4])
            .with_roles(vec![Role::TOWN, Role::COP, Role::DOCTOR, Role::MAFIA])
            .build()
            .is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn from_state() -> Result<()> {
        let state = StateBuilder::from_inner_state(InnerState::new(
            0,
            &vec![
                (1, Role::TOWN),
                (2, Role::COP),
                (3, Role::DOCTOR),
                (4, Role::MAFIA),
            ],
            Rules {},
        ))
        .build()
        .await?;

        state.quit().await;

        Ok(())
    }
}
