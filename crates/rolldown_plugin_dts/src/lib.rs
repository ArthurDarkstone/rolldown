mod utils;

use std::{borrow::Cow, path::Path};

use arcstr::ArcStr;
use itertools::Itertools as _;
use oxc::{
  allocator::IntoIn,
  ast_visit::VisitMut,
  codegen::Codegen,
  isolated_declarations::{IsolatedDeclarations, IsolatedDeclarationsOptions},
};
use rolldown_common::{ModuleType, ResolvedExternal};
use rolldown_error::{BuildDiagnostic, Severity};
use rolldown_plugin::{HookUsage, Plugin, PluginHookMeta, PluginOrder};
use rolldown_utils::stabilize_id::stabilize_id;
use serde_json::Value;
use sugar_path::SugarPath;

use crate::utils::TypeImportVisitor;

#[derive(Debug, Default)]
pub struct DtsPlugin {
  pub respect_external: bool,
  pub tsconfig: Option<String>,
  pub compiler_options: Option<DtsPluginCompilerOptions>,
  pub strip_internal: bool,
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

  async fn transform_ast(
    &self,
    ctx: &rolldown_plugin::PluginContext,
    mut args: rolldown_plugin::HookTransformAstArgs<'_>,
  ) -> rolldown_plugin::HookTransformAstReturn {
    // 只处理 TypeScript 文件
    if !matches!(args.module_type, ModuleType::Ts | ModuleType::Tsx) {
      return Ok(args.ast);
    }

    // 检查编译器选项，如果禁用了声明文件生成则跳过
    if let Some(compiler_options) = &self.compiler_options {
      if compiler_options.no_emit || !compiler_options.declaration {
        return Ok(args.ast);
      }
    }

    // 收集类型导入说明符
    let type_import_specifiers = args.ast.program.with_mut(|fields| {
      let mut visitor = TypeImportVisitor { imported: vec![].into_in(fields.allocator) };
      visitor.visit_program(fields.program);
      visitor.imported
    });

    // 解析类型导入的依赖
    for specifier in type_import_specifiers {
      let resolved_id = ctx.resolve(&specifier, Some(args.id), None).await??;
      if matches!(resolved_id.external, ResolvedExternal::Bool(false)) {
        ctx.load(&resolved_id.id, None).await?;
      }
    }

    // 生成 TypeScript 声明文件
    let ret = args.ast.program.with_mut(|fields| {
      IsolatedDeclarations::new(
        fields.allocator,
        IsolatedDeclarationsOptions { strip_internal: self.strip_internal },
      )
      .build(fields.program)
    });

    // 处理错误
    if !ret.errors.is_empty() {
      let errors = BuildDiagnostic::from_oxc_diagnostics(
        ret.errors,
        &ArcStr::from(ret.program.source_text),
        &stabilize_id(args.id, ctx.cwd()),
        &Severity::Error,
      )
      .iter()
      .map(|error| error.to_diagnostic().with_kind(self.name().into_owned()).to_color_string())
      .join("\n\n");
      return Err(anyhow::anyhow!("\n{errors}"));
    }

    // 代码生成
    let codegen_ret = Codegen::new().build(&ret.program);

    // 确定输出文件路径
    let mut emit_dts_path = Path::new(args.stable_id).to_path_buf();
    emit_dts_path.set_extension("d.ts");

    // 生成 .d.ts 文件
    ctx.emit_file(
      rolldown_common::EmittedAsset {
        name: None,
        original_file_name: None,
        file_name: Some(emit_dts_path.to_slash_lossy().into()),
        source: codegen_ret.code.into(),
      },
      None,
      None,
    );

    Ok(args.ast)
  }

  // 确保在类型剥离之前运行
  fn transform_ast_meta(&self) -> Option<PluginHookMeta> {
    Some(PluginHookMeta { order: Some(PluginOrder::Post) })
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::TransformAst
  }
}
