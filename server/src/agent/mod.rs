pub mod agent_loop;
pub mod context;
pub mod intent;
pub mod registry;
pub mod tool;
pub mod tools;

pub use agent_loop::{run_agent_loop, AgentConfig, AgentEvent};
pub use intent::IntentAnalyzer;
pub use registry::ToolRegistry;
pub use tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
