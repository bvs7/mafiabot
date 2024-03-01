#![allow(unused_imports)]
use super::*;
use tokio::join;
use tokio::time::Duration;

impl ID for u32 {}

async fn start_print_event_handler(
    mut event_rx: mpsc::Receiver<Event<u32>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let event = event_rx.recv().await.expect("Event to receive");
            println!("EVENT: {:?}", event);
            tokio::time::sleep(Duration::from_millis(10)).await;
            if let Event::Close { .. } = event {
                break;
            }
        }
    })
}

fn get_players(n: u8) -> HashMap<u32, Role<u32>> {
    let mut players = HashMap::new();
    let role_list = vec![
        Role::TOWN,         // 1
        Role::TOWN,         // 2
        Role::MAFIA,        // 3
        Role::COP,          // 4
        Role::DOCTOR,       // 5
        Role::STRIPPER,     // 6
        Role::CELEB,        // 7
        Role::IDIOT(false), // 8
        Role::SURVIVOR,     // 9
        Role::AGENT(1),     // 10
        Role::GUARD(1),     // 11
    ];
    for i in 1..=n {
        players.insert(i as u32, role_list[i as usize - 1]);
    }
    players
}

async fn vote(
    cmd_tx: &CommandTx<u32>,
    voter: u32,
    choice: Choice<u32>,
) -> Result<(), CoreError<u32>> {
    let action = Action::Vote { voter, choice };
    Interface::send_action(&cmd_tx, action).await
}

async fn votes(
    cmd_tx: &CommandTx<u32>,
    voters: Vec<u32>,
    choice: Choice<u32>,
) -> Result<(), CoreError<u32>> {
    for voter in voters {
        vote(&cmd_tx, voter, choice).await?;
    }
    Ok(())
}

async fn target(
    cmd_tx: &CommandTx<u32>,
    actor: u32,
    target: Choice<u32>,
) -> Result<(), CoreError<u32>> {
    let action = Action::Target { actor, target };
    Interface::send_action(&cmd_tx, action).await
}

async fn scheme(
    cmd_tx: &CommandTx<u32>,
    actor: u32,
    mark: Choice<u32>,
) -> Result<(), CoreError<u32>> {
    let action = Action::Scheme { actor, mark };
    Interface::send_action(&cmd_tx, action).await
}

async fn wait() {
    tokio::time::sleep(Duration::from_millis(250)).await;
}

async fn beat() {
    tokio::time::sleep(Duration::from_millis(15)).await;
}

#[tokio::test]
async fn test_targeting() -> Result<(), CoreError<u32>> {
    // env::set_var("RUST_BACKTRACE", "1");

    let players = get_players(7);

    let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;

    let event_handler_join = start_print_event_handler(event_rx).await;

    Interface::send_action(&cmd_tx, Action::Start).await?;

    let state = Interface::send_status(&cmd_tx).await?;
    // println!("{:#?}", state);

    assert_eq!(state.day_no, 1);
    assert_eq!(state.players.len(), 7);
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 1, Choice::Player(3)).await?;
    vote(&cmd_tx, 2, Choice::Player(3)).await?;
    vote(&cmd_tx, 3, Choice::Player(3)).await?;
    vote(&cmd_tx, 4, Choice::Player(3)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    // println!("{:#?}", state);
    assert_eq!(state.day_no, 1);
    assert_eq!(state.players.len(), 6);
    assert_eq!(state.phase.kind(), PhaseKind::Night);

    scheme(&cmd_tx, 6, Choice::Player(1)).await?;

    assert_eq!(
        target(&cmd_tx, 6, Choice::Player(4)).await,
        Err(CoreError::StripperOverload { actor: 6 })
    );

    scheme(&cmd_tx, 6, Choice::Abstain).await?;
    target(&cmd_tx, 6, Choice::Player(4)).await?;

    target(&cmd_tx, 4, Choice::Player(1)).await?;
    target(&cmd_tx, 5, Choice::Player(5)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;

    // println!("{:#?}", state);

    assert_eq!(state.day_no, 2);
    assert_eq!(state.players.len(), 6);
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 7, Choice::Player(1)).await?;
    vote(&cmd_tx, 6, Choice::Player(1)).await?;
    vote(&cmd_tx, 1, Choice::Player(1)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 1, Choice::Abstain).await?;
    vote(&cmd_tx, 2, Choice::Abstain).await?;
    assert_eq!(
        vote(&cmd_tx, 3, Choice::Abstain).await,
        Err(CoreError::InvalidPlayer { player: 3 })
    );
    vote(&cmd_tx, 4, Choice::Abstain).await?;

    Interface::send_action(&cmd_tx, Action::Unvote { voter: 4 }).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 7, Choice::Abstain).await?;
    vote(&cmd_tx, 6, Choice::Abstain).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Night);

    target(&cmd_tx, 4, Choice::Player(6)).await?;
    target(&cmd_tx, 5, Choice::Player(5)).await?;

    target(&cmd_tx, 6, Choice::Abstain).await?;
    scheme(&cmd_tx, 6, Choice::Player(1)).await?;

    target(&cmd_tx, 4, Choice::Player(1)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 7, Choice::Player(2)).await?;
    vote(&cmd_tx, 6, Choice::Player(2)).await?;
    vote(&cmd_tx, 2, Choice::Player(2)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;

    //4-COP, 5-DOCTOR, 6-STRIPPER, 7-CELEB
    assert_eq!(state.phase.kind(), PhaseKind::Night);
    assert_eq!(state.players.len(), 4);

    scheme(&cmd_tx, 6, Choice::Abstain).await?;
    target(&cmd_tx, 6, Choice::Player(4)).await?;
    target(&cmd_tx, 4, Choice::Player(5)).await?;
    target(&cmd_tx, 5, Choice::Player(4)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    vote(&cmd_tx, 7, Choice::Abstain).await?;
    vote(&cmd_tx, 6, Choice::Abstain).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Night);

    target(&cmd_tx, 6, Choice::Player(7)).await?;
    target(&cmd_tx, 4, Choice::Player(7)).await?;
    target(&cmd_tx, 5, Choice::Player(5)).await?;
    scheme(&cmd_tx, 6, Choice::Abstain).await?;

    wait().await;

    Interface::send_action(&cmd_tx, Action::Reveal { player: 7 }).await?;

    vote(&cmd_tx, 7, Choice::Player(6)).await?;
    vote(&cmd_tx, 5, Choice::Player(6)).await?;
    vote(&cmd_tx, 4, Choice::Player(6)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::End);

    assert!(matches!(state.phase, Phase::End { winner: Team::Town }));

    Interface::send_close(&cmd_tx).await;

    let _ = join!(core_join, event_handler_join);

    return Ok(());
}

#[tokio::test]
async fn test_contracts() -> Result<(), CoreError<u32>> {
    // 1-TOWN, 2-TOWN, 3-MAFIA, 4-COP, 5-DOCTOR, 6-STRIPPER,
    // 7-CELEB, 8-IDIOT, 9-SURVIVOR, 10-AGENT(1), 11-GUARD(1)

    let players = get_players(11);
    let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;
    let event_handler_join = start_print_event_handler(event_rx).await;

    Interface::send_action(&cmd_tx, Action::Start).await?;

    votes(&cmd_tx, vec![2, 3, 4, 5, 6, 7, 8], Choice::Player(1)).await?;

    beat().await;

    votes(&cmd_tx, vec![2, 7], Choice::Abstain).await?;

    vote(&cmd_tx, 7, Choice::Player(1)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Night);
    assert!(matches!(state.players[&10], Role::GUARD(7)));
    assert!(matches!(state.players[&11], Role::AGENT(7)));

    target(&cmd_tx, 4, Choice::Player(3)).await?;
    target(&cmd_tx, 5, Choice::Player(5)).await?;
    scheme(&cmd_tx, 3, Choice::Player(2)).await?;
    target(&cmd_tx, 6, Choice::Player(10)).await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Day);

    votes(&cmd_tx, vec![3, 4, 5, 6, 7], Choice::Player(8)).await?;

    wait().await;
    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    assert_eq!(state.phase.kind(), PhaseKind::Eclipse);

    Interface::send_action(
        &cmd_tx,
        Action::Avenge {
            avenger: 8,
            victim: Choice::Player(7),
        },
    )
    .await?;

    wait().await;

    let state = Interface::send_status(&cmd_tx).await?;
    println!("{:#?}", state);
    assert_eq!(state.phase.kind(), PhaseKind::Night);

    Interface::send_close(&cmd_tx).await;

    let _ = join!(core_join, event_handler_join);

    Ok(())
}

#[tokio::test]
async fn test_serialize() -> Result<(), CoreError<u32>> {
    // Setup a game (in the middle of Election Imminent state)

    let players = get_players(11);
    let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;
    let event_handler_join = start_print_event_handler(event_rx).await;

    Interface::send_action(&cmd_tx, Action::Start).await?;

    votes(&cmd_tx, vec![2, 3, 4, 5, 6, 7, 8], Choice::Player(1)).await?;

    beat().await;

    // Try to serialize the game state

    let saved_game = Interface::send_serialize(&cmd_tx)
        .await
        .expect("Failed to serialize game");

    Interface::send_close(&cmd_tx).await;

    println!("\nGame ID:\n---\n{}", saved_game.id);
    println!("\nGame State:\n---\n{}", saved_game.state);
    println!("\nGame Rules:\n---\n{}", saved_game.rules);

    let _ = join!(core_join, event_handler_join);

    Ok(())
}
