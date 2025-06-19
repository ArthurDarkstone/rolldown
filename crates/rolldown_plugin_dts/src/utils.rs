// cSpell:disable
use std::fmt::Write as _;

use oxc::{
  allocator::{Allocator, Vec as OxcVec},
  ast::{
    ast::{
      ImportDeclaration, ImportDeclarationSpecifier, ImportSpecifier, TSImportType, TSTypeReference,
    },
    visit::VisitMut,
  },
  span::Atom,
};
use rolldown_utils::concat_string;
use serde_json::Value;

// Use `10kB` as a threshold for 'auto'
// https://v8.dev/blog/cost-of-javascript-2019#json
pub const THRESHOLD_SIZE: usize = 10 * 1000;

/// /\.json(?:$|\?)(?!commonjs-(?:proxy|external))/
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn is_json_ext(ext: &str) -> bool {
  if ext.ends_with(".json") {
    return true;
  }
  let Some(i) = memchr::memmem::rfind(ext.as_bytes(), b".json?") else {
    return false;
  };
  let postfix = &ext[i + 6..];
  postfix != "commonjs-proxy" && postfix != "commonjs-external"
}

/// SPECIAL_QUERY_RE = /[?&](?:worker|sharedworker|raw|url)\b/
pub fn is_special_query(ext: &str) -> bool {
  for i in memchr::memrchr2_iter(b'?', b'&', ext.as_bytes()) {
    let Some(after) = ext.get(i + 1..) else {
      continue;
    };

    let boundary = if after.starts_with("raw") || after.starts_with("url") {
      3usize
    } else if after.starts_with("worker") {
      6usize
    } else if after.starts_with("sharedworker") {
      12usize
    } else {
      continue;
    };

    // Test if match `\b`
    match after.get(boundary..=boundary).and_then(|c| c.bytes().next()) {
      Some(ch) if !ch.is_ascii_alphanumeric() && ch != b'_' => {
        return true;
      }
      None => return true,
      _ => {}
    }
  }
  false
}

#[inline]
pub fn strip_bom(code: &str) -> &str {
  code.strip_prefix("\u{FEFF}").unwrap_or(code)
}

#[inline]
fn serialize_value(value: &Value) -> Result<String, serde_json::Error> {
  let value_as_string = serde_json::to_string(value)?;
  if value_as_string.len() > THRESHOLD_SIZE && value.is_object() {
    let value = serde_json::to_string(&value_as_string)?;
    Ok(concat_string!("/*#__PURE__*/ JSON.parse(", value, ")"))
  } else {
    Ok(value_as_string)
  }
}

pub fn json_to_esm(data: &Value, named_exports: bool) -> String {
  if !named_exports || !data.is_object() {
    return concat_string!("export default ", data.to_string(), ";\n");
  }

  let data = data.as_object().unwrap();
  if data.is_empty() {
    return "export default {{}};\n".to_string();
  }

  let mut named_export_code = String::new();
  let mut default_object_code = String::new();
  for (key, value) in data {
    let value = serialize_value(value).expect("Invalid JSON value");
    if rolldown_utils::ecmascript::is_validate_assignee_identifier_name(key) {
      writeln!(named_export_code, "export const {key} = {value};").unwrap();
      writeln!(default_object_code, "  {key},").unwrap();
    } else {
      let key = serde_json::to_string(key).unwrap();
      writeln!(default_object_code, "  {key}: {value},").unwrap();
    }
  }

  // Remove the trailing ",\n"
  default_object_code.truncate(default_object_code.len() - 2);

  concat_string!(named_export_code, "export default {\n", default_object_code, "\n};")
}

/// 访问器用于收集类型导入
pub struct TypeImportVisitor<'a> {
  pub imported: OxcVec<'a, Atom<'a>>,
}

impl<'a> TypeImportVisitor<'a> {
  pub fn new(allocator: &'a Allocator) -> Self {
    Self { imported: OxcVec::new_in(allocator) }
  }
}

impl<'a> VisitMut<'a> for TypeImportVisitor<'a> {
  fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
    // 检查是否为类型导入
    if it.import_kind.is_type() {
      self.imported.push(it.source.value.clone());
      return;
    }

    // 检查是否有类型导入说明符
    let has_type_specifier = it.specifiers.as_ref().map_or(false, |specifiers| {
      specifiers.iter().any(|spec| match spec {
        ImportDeclarationSpecifier::ImportSpecifier(ImportSpecifier { import_kind, .. }) => {
          import_kind.is_type()
        }
        _ => false,
      })
    });

    if has_type_specifier {
      self.imported.push(it.source.value.clone());
    }
  }

  fn visit_ts_import_type(&mut self, it: &mut TSImportType<'a>) {
    if let Some(parameter) = &it.parameter {
      if let Some(literal) = parameter.as_ts_literal_type() {
        if let Some(string_literal) = literal.literal.as_string_literal() {
          self.imported.push(string_literal.value.clone());
        }
      }
    }
  }

  fn visit_ts_type_reference(&mut self, it: &mut TSTypeReference<'a>) {
    // 处理类型引用中可能的导入
    self.visit_ts_type_name(&mut it.type_name);
    if let Some(type_parameters) = &mut it.type_parameters {
      self.visit_ts_type_parameter_instantiation(type_parameters);
    }
  }
}

/// 检查文件扩展名是否为 TypeScript 文件
pub fn is_typescript_file(path: &str) -> bool {
  path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".d.ts")
}

/// 检查文件扩展名是否为声明文件
pub fn is_declaration_file(path: &str) -> bool {
  path.ends_with(".d.ts")
}

/// 从路径生成对应的声明文件路径
pub fn get_declaration_path(path: &str) -> String {
  if path.ends_with(".ts") {
    path.replace(".ts", ".d.ts")
  } else if path.ends_with(".tsx") {
    path.replace(".tsx", ".d.ts")
  } else {
    format!("{}.d.ts", path)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_typescript_file() {
    assert!(is_typescript_file("test.ts"));
    assert!(is_typescript_file("test.tsx"));
    assert!(is_typescript_file("test.d.ts"));
    assert!(!is_typescript_file("test.js"));
    assert!(!is_typescript_file("test.jsx"));
  }

  #[test]
  fn test_is_declaration_file() {
    assert!(is_declaration_file("test.d.ts"));
    assert!(!is_declaration_file("test.ts"));
    assert!(!is_declaration_file("test.tsx"));
  }

  #[test]
  fn test_get_declaration_path() {
    assert_eq!(get_declaration_path("test.ts"), "test.d.ts");
    assert_eq!(get_declaration_path("test.tsx"), "test.d.ts");
    assert_eq!(get_declaration_path("test.js"), "test.js.d.ts");
  }
}
