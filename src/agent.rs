use crate::api::{Agent, LLMProvider};
use crate::tools::AgentTool;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AgentRole {
    Architect,
    Coder,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultiAgentConfig {
    pub agents: Vec<AgentConfig>,
}

impl MultiAgentConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.agents.is_empty() {
            return Err("At least one agent must be configured.".into());
        }
        if self.agents.len() > 2 {
            return Err(format!(
                "Maximum number of agents allowed is 2, but found {}.",
                self.agents.len()
            ));
        }
        let mut has_architect = false;
        let mut has_coder = false;
        for agent in &self.agents {
            match agent.role {
                AgentRole::Architect => has_architect = true,
                AgentRole::Coder => has_coder = true,
            }
        }
        if self.agents.len() == 2 && (!has_architect || !has_coder) {
            return Err(
                "When 2 agents are configured, one must be Architect and one must be Coder.".into(),
            );
        }
        Ok(())
    }
}

pub struct MultiAgentOrchestrator {
    agents: Vec<Agent>,
    #[allow(dead_code)]
    workspace_root: PathBuf,
    config: MultiAgentConfig,
}

impl MultiAgentOrchestrator {
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        workspace_root: PathBuf,
        config: MultiAgentConfig,
    ) -> Result<Self, String> {
        config.validate()?;
        let mut agents = Vec::new();
        for agent_cfg in &config.agents {
            let mut agent = Agent::with_provider(
                Arc::clone(&provider),
                agent_cfg.model.clone(),
                workspace_root.clone(),
            );

            if agent_cfg.role == AgentRole::Architect {
                agent.restrict_to_read_only_tools();
            }

            if let Some(ref prompt) = agent_cfg.system_prompt {
                let custom_init = format!(
                    "[AGENT PERSONA: {} ({:?})]\n{}",
                    agent_cfg.name, agent_cfg.role, prompt
                );
                agent.inject_custom_system_context(custom_init);
            }
            agents.push(agent);
        }
        Ok(Self {
            agents,
            workspace_root,
            config,
        })
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn get_config(&self) -> &MultiAgentConfig {
        &self.config
    }

    pub async fn run_workflow(&mut self, prompt: &str, auto_approve: bool, plan_mode: bool) {
        if self.agents.len() == 1 {
            self.agents[0]
                .run_with_mode(prompt, auto_approve, plan_mode)
                .await;
            return;
        }

        println!(
            "{}",
            "=== Multi-Agent Workflow: Step 1 [Architect Analysis & Planning] ==="
                .cyan()
                .bold()
        );

        self.agents[0].run_with_mode(prompt, true, true).await;

        let architect_plan = self.agents[0]
            .get_last_assistant_text()
            .unwrap_or_else(|| "Proceed with standard execution based on user prompt.".to_string());

        println!(
            "\n{}",
            "=== Multi-Agent Workflow: Step 2 [Coder Execution & Implementation] ==="
                .cyan()
                .bold()
        );
        let coder_prompt = format!(
            "User Request:\n{prompt}\n\nArchitect Plan & Guidance:\n{architect_plan}\n\nPlease implement the plan and execute necessary changes."
        );

        self.agents[1]
            .run_with_mode(&coder_prompt, auto_approve, plan_mode)
            .await;
    }

    pub fn with_tool_on_all(mut self, tool: Arc<dyn AgentTool>) -> Self {
        for agent in &mut self.agents {
            let _ = agent;
            let _ = &tool;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_two_agents_validation() {
        let valid_config = MultiAgentConfig {
            agents: vec![
                AgentConfig {
                    name: "Architect".into(),
                    role: AgentRole::Architect,
                    model: "gemini-3.5-flash-lite".into(),
                    system_prompt: Some("You plan things.".into()),
                },
                AgentConfig {
                    name: "Coder".into(),
                    role: AgentRole::Coder,
                    model: "gpt-5.6-luna".into(),
                    system_prompt: Some("You code things.".into()),
                },
            ],
        };
        assert!(valid_config.validate().is_ok());

        let invalid_config = MultiAgentConfig {
            agents: vec![
                AgentConfig {
                    name: "A1".into(),
                    role: AgentRole::Architect,
                    model: "m1".into(),
                    system_prompt: None,
                },
                AgentConfig {
                    name: "A2".into(),
                    role: AgentRole::Coder,
                    model: "m2".into(),
                    system_prompt: None,
                },
                AgentConfig {
                    name: "A3".into(),
                    role: AgentRole::Coder,
                    model: "m3".into(),
                    system_prompt: None,
                },
            ],
        };
        let err_msg = invalid_config.validate().unwrap_err();
        assert!(err_msg.contains("Maximum number of agents allowed is 2"));
    }
}
