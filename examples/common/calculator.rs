#![allow(dead_code)]

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters, wrapper::Json},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SubRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    #[schemars(description = "the right hand side number")]
    pub b: i32,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CalculatorResult {
    #[schemars(description = "the result of the operation")]
    pub value: i32,
}

impl From<i32> for CalculatorResult {
    fn from(value: i32) -> Self {
        Self { value }
    }
}

#[derive(Debug, Clone)]
pub struct Calculator {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Calculator {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(
        &self,
        Parameters(SumRequest { a, b }): Parameters<SumRequest>,
    ) -> Json<CalculatorResult> {
        Json(CalculatorResult::from(a + b))
    }

    #[tool(description = "Calculate the difference of two numbers")]
    fn sub(
        &self,
        Parameters(SubRequest { a, b }): Parameters<SubRequest>,
    ) -> Json<CalculatorResult> {
        Json(CalculatorResult::from(a - b))
    }
}

#[tool_handler]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
