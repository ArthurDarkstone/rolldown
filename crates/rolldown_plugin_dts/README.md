# Rolldown DTS Plugin

A Rolldown plugin for generating TypeScript declaration files (`.d.ts`) from TypeScript source files.

## Features

- Generates `.d.ts` files from TypeScript (`.ts` and `.tsx`) files
- Handles type imports and dependencies
- Supports stripping internal types (marked with `@internal`)
- Configurable compiler options
- Built on top of OXC's isolated declarations

## Usage

```rust
use rolldown_plugin_dts::{DtsPlugin, DtsPluginCompilerOptions};

let dts_plugin = DtsPlugin {
    respect_external: true,
    tsconfig: Some("tsconfig.json".to_string()),
    compiler_options: Some(DtsPluginCompilerOptions {
        declaration: true,
        declaration_map: false,
        emit_declaration_only: false,
        no_emit: false,
    }),
    strip_internal: true,
};
```

## Configuration Options

### `respect_external`

- Type: `boolean`
- Default: `false`
- Description: Whether to respect external dependencies when generating declarations

### `tsconfig`

- Type: `Option<String>`
- Default: `None`
- Description: Path to the TypeScript configuration file

### `compiler_options`

- Type: `Option<DtsPluginCompilerOptions>`
- Default: `None`
- Description: TypeScript compiler options for declaration generation

#### DtsPluginCompilerOptions

- `declaration`: Enable declaration file generation
- `declaration_map`: Generate declaration map files
- `emit_declaration_only`: Only emit declaration files
- `no_emit`: Disable file output

### `strip_internal`

- Type: `boolean`
- Default: `false`
- Description: Remove declarations marked with `@internal` JSDoc tag

## How it works

1. The plugin processes TypeScript files (`.ts` and `.tsx`)
2. It collects type imports and resolves dependencies
3. Uses OXC's isolated declarations to generate type definitions
4. Emits the generated `.d.ts` files alongside the bundled output

## Requirements

- TypeScript source files must be compatible with isolated declarations
- All type dependencies must be resolvable
- Files should follow TypeScript's declaration generation requirements

## Limitations

- Requires TypeScript files to be compatible with isolated declarations
- Does not support all TypeScript features (similar to TypeScript's `isolatedDeclarations` flag)
- Type imports must be explicitly declared
