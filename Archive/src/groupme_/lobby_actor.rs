use super::game_actor::*;

struct LobbyActor {
    lobby_chat: GroupMeGroup,
    lobby_cmd_rx: mpsc::Receiver<LobbyCmd>,
    actor_tx: ActorTx,
    games: HashMap<Gid, GameStatusRx>,
}
