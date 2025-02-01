use std::collections::{HashMap, HashSet};

use crate::prelude::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    id: GroupId,
    members: Vec<Member>,
    #[serde(skip)]
    names: HashMap<UserId, String>,
}

impl Group {
    pub async fn new(name: String) -> Self {
        let id = api::create_group(&name, false).await.unwrap();
        Self { id, members: Vec::new(), names: HashMap::new() }
    }

    pub async fn from_id(id: GroupId) -> Self {
        let group_resp = api::get_group(&id).await.unwrap();
        let mut group = Self { id, members: group_resp.members, names: HashMap::new() };
        for member in group.members.iter() {
            group.names.insert(member.user_id, member.nickname.clone());
        }
        group
    }

    pub fn id(&self) -> GroupId {
        self.id
    }

    pub fn name(&self, user_id: &UserId) -> Option<String> {
        self.names.get(user_id).cloned()
    }

    pub fn names(&self) -> HashMap<UserId, String> {
        self.names.clone()
    }

    pub fn get_member(&self, user_id: &UserId) -> Option<&Member> {
        self.members.iter().find(|m| &m.user_id == user_id)
    }

    pub async fn add_members(&mut self, members: Vec<Member>) -> Vec<Member> {
        for member in &members {
            self.names.insert(member.user_id, member.nickname.clone());
        }
        api::add_members(&self.id, members).await.unwrap()
    }

    pub async fn update_names(&mut self) {
        let group = api::get_group(&self.id).await.unwrap();
        let mut ids = HashSet::new();
        for member in group.members {
            self.names.insert(member.user_id, member.nickname);
            ids.insert(member.user_id);
        }
        // self.names.retain(|k, _| ids.contains(k));
    }
}
