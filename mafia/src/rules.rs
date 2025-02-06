use crate::prelude::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub rolegen_config: RoleGenConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<usize>,
}
