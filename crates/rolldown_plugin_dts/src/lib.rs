mod utils;

use std::borrow::Cow;

use rolldown_common::ModuleType;
use rolldown_plugin::{HookTransformOutput, HookUsage, Plugin};
use rolldown_sourcemap::SourceMap;
use rolldown_utils::concat_string;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct DtsPlugin {
  pub respect_external: bool,
  pub tsconfig: string,
  pub compiler_options: Option<Value>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DtsPluginCompilerOptions {
  pub no_emit: bool,
  pub declaration: bool,
  pub declaration_map: bool,
  pub emit_declaration_only: bool,
}

impl Plugin for DtsPlugin {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed("builtin:dts")
  }

  async fn transform(
    &self,
    _ctx: rolldown_plugin::SharedTransformPluginContext,
    args: &rolldown_plugin::HookTransformArgs<'_>,
  ) -> rolldown_plugin::HookTransformReturn {
  }

  fn register_hook_usage(&self) -> rolldown_plugin::HookUsage {
    HookUsage::Transform
  }
}
