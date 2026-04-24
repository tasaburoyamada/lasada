use crate::core::traits::{ExecutionEngine, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use crate::plugins::bash_executor::BashExecutor;
use crate::plugins::python_executor::PythonExecutor;
use crate::plugins::computer_executor::ComputerExecutor;
use crate::plugins::web_executor::WebExecutor;

pub struct ExecutionDispatcher {
    executors: HashMap<String, Box<dyn ExecutionEngine>>,
    default_executor: String,
}

impl ExecutionDispatcher {
    pub fn new() -> Self {
        let mut executors: HashMap<String, Box<dyn ExecutionEngine>> = HashMap::new();
        
        // Register default executors
        executors.insert("bash".to_string(), Box::new(BashExecutor::new()));
        executors.insert("sh".to_string(), Box::new(BashExecutor::new()));
        executors.insert("python".to_string(), Box::new(PythonExecutor::new()));
        executors.insert("python3".to_string(), Box::new(PythonExecutor::new()));
        executors.insert("computer".to_string(), Box::new(ComputerExecutor::new()));
        executors.insert("web".to_string(), Box::new(WebExecutor::new()));

        Self {
            executors,
            default_executor: "bash".to_string(),
        }
    }
}

#[async_trait]
impl ExecutionEngine for ExecutionDispatcher {
    async fn start_session(&mut self) -> Result<()> {
        for executor in self.executors.values_mut() {
            executor.start_session().await?;
        }
        Ok(())
    }

    async fn execute(&mut self, code: &str, language: &str) -> Result<String> {
        let lang = if language.is_empty() {
            &self.default_executor
        } else {
            language
        };

        if let Some(executor) = self.executors.get_mut(lang) {
            executor.execute(code, lang).await
        } else {
            // Fallback to bash if unknown language
            if let Some(executor) = self.executors.get_mut(&self.default_executor) {
                executor.execute(code, lang).await
            } else {
                Err(crate::core::traits::AppError::ExecutionError(format!("No executor for language: {}", lang)))
            }
        }
    }
}
