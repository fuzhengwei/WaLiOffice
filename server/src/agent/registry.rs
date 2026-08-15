use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::tool::{DynTool, OfficeTool};
use crate::llm::FunctionDef;
use crate::llm::FunctionSpec;

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, DynTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, tool: DynTool) {
        let name = tool.name().to_string();
        self.tools.write().await.insert(name, tool);
    }

    pub async fn get(&self, name: &str) -> Option<DynTool> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<DynTool> {
        let tools = self.tools.read().await;
        let mut list: Vec<DynTool> = tools.values().cloned().collect();
        list.sort_by(|a, b| a.name().cmp(b.name()));
        list
    }

    pub async fn to_function_defs(&self) -> Vec<FunctionDef> {
        self.list()
            .await
            .iter()
            .map(|t| FunctionDef {
                def_type: "function".to_string(),
                function: FunctionSpec {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters(),
                },
            })
            .collect()
    }
}

use once_cell::sync::Lazy;
pub static REGISTRY: Lazy<ToolRegistry> = Lazy::new(ToolRegistry::new);
