pub mod tool;
pub mod registry;
pub mod context;
pub mod agent_loop;
pub mod tools;

pub use tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
pub use registry::ToolRegistry;
pub use agent_loop::{run_agent_loop, AgentEvent, AgentConfig};
