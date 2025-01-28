use crate::prelude::*;

use api::{self, Member};

#[derive(Debug)]
pub struct GroupMeGroup {
    pub id: GroupId,
    pub names: HashMap<UserId, String>,
}

impl GroupMeGroup {
    pub async fn new(name: String, app_state: &Arc<RwLock<AppStatus>>) -> GroupId {
        let id = api::create_group(&name, true).await.unwrap();
        let group = Self { id: id.clone(), names: HashMap::new() };
        let mut w_app_state = app_state.write().await;
        w_app_state.groups.insert(id.clone(), group);
        drop(w_app_state);
        id
    }

    pub async fn add_members(&mut self, members: Vec<Member>) {
        let members = api::add_members(&self.id, members).await.unwrap();
        for member in members {
            self.names.insert(member.user_id, member.nickname);
        }
    }

    pub async fn update_name(&mut self, user: UserId, name: String) {
        self.names.insert(user, name);
    }
}
