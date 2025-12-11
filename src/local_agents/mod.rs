pub mod manager;
pub mod agent;
pub mod communication;

pub use manager::LocalAgentManager;
pub use agent::{LocalAgent, AgentStatus};
pub use communication::{LocalAgentClient};