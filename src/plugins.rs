//! # WASM Plugin System
//!
//! Extends RavenClaws with WebAssembly plugins — load `.wasm` binaries at
//! runtime and register their exported tools into the agent's `ToolRegistry`.
//!
//! ## Plugin ABI (v1)
//!
//! Every WASM plugin **must** export the following functions:
//!
//! | Export | Signature | Description |
//! |--------|-----------|-------------|
//! | `plugin_name` | `() -> i32` | Pointer to null-terminated UTF-8 name string in linear memory |
//! | `plugin_version` | `() -> i32` | Pointer to null-terminated version string |
//! | `plugin_description` | `() -> i32` | Pointer to null-terminated description string |
//! | `plugin_tools_count` | `() -> i32` | Number of tools this plugin exposes |
//! | `plugin_tool_name` | `(i32) -> i32` | Given tool index, returns pointer to name string |
//! | `plugin_tool_description` | `(i32) -> i32` | Given tool index, returns pointer to description string |
//! | `plugin_tool_execute` | `(i32, i32) -> i32` | Given tool index + input pointer, returns pointer to output string |
//!
//! Strings are communicated via **pointer into the plugin's exported linear
//! memory**. The host reads them by inspecting the memory at the returned offset.

use std::sync::Arc;

use wasmtime::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::tools::{ToolDefinition, ToolImpl, ToolRegistry, ToolResult, ToolResultValue};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during plugin loading, inspection, or execution.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// The WASM binary could not be compiled.
    #[error("Failed to compile WASM module: {0}")]
    Compile(String),

    /// The WASM module could not be instantiated.
    #[error("Failed to instantiate WASM module: {0}")]
    Instantiate(String),

    /// A required export is missing from the plugin.
    #[error("Missing required export '{export}' in plugin")]
    MissingExport { export: String },

    /// An export has the wrong type signature.
    #[error("Export '{export}' has wrong type: {details}")]
    WrongExportType { export: String, details: String },

    /// A tool call failed at runtime.
    #[error("Plugin tool '{tool}' execution failed: {details}")]
    ToolExecution { tool: String, details: String },

    /// The plugin version is not compatible with this host.
    #[error("Plugin version '{version}' is not compatible with host ABI v1")]
    VersionMismatch { version: String },

    /// General I/O or other error.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ---------------------------------------------------------------------------
// PluginTool — a tool exposed by a WASM plugin
// ---------------------------------------------------------------------------

/// A single tool exposed by a WASM plugin.
#[derive(Debug, Clone)]
pub struct PluginTool {
    /// Human-readable name (e.g. "hello", "echo").
    pub name: String,
    /// Short description of what the tool does.
    pub description: String,
    /// Index of this tool within the plugin (0-based).
    pub index: u32,
}

// ---------------------------------------------------------------------------
// WasmPlugin — a loaded plugin instance
// ---------------------------------------------------------------------------

/// A loaded WASM plugin with its store, instance, and exported tools.
pub struct WasmPlugin {
    /// Plugin name (from `plugin_name` export).
    pub name: String,
    /// Plugin version (from `plugin_version` export).
    pub version: String,
    /// Plugin description (from `plugin_description` export).
    pub description: String,
    /// Tools exported by this plugin.
    pub tools: Vec<PluginTool>,
    /// The store holds the plugin's linear memory and runtime state.
    store: Store<()>,
    /// Exported memory reference.
    memory: Memory,
    /// Cached typed functions.
    tool_execute: TypedFunc<(i32, i32), i32>,
}

impl std::fmt::Debug for WasmPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmPlugin")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("description", &self.description)
            .field("tools", &self.tools)
            .finish_non_exhaustive()
    }
}

impl WasmPlugin {
    /// Load a plugin from a `.wasm` file on disk.
    pub fn load(path: &std::path::Path) -> Result<Self, PluginError> {
        let wasm_bytes = std::fs::read(path).map_err(|e| {
            PluginError::Other(anyhow::anyhow!("Failed to read {}: {}", path.display(), e))
        })?;
        Self::load_from_bytes(&wasm_bytes)
    }

    /// Load a plugin from an in-memory WASM binary.
    pub fn load_from_bytes(wasm_bytes: &[u8]) -> Result<Self, PluginError> {
        let engine = Engine::default();
        let module =
            Module::new(&engine, wasm_bytes).map_err(|e| PluginError::Compile(e.to_string()))?;

        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::Instantiate(e.to_string()))?;

        // ---- read required exports ----
        let name = Self::read_export_string(&instance, &mut store, "plugin_name")?;
        let version = Self::read_export_string(&instance, &mut store, "plugin_version")?;
        let description = Self::read_export_string(&instance, &mut store, "plugin_description")?;

        // ---- version check ----
        if version.is_empty() {
            return Err(PluginError::VersionMismatch {
                version: version.clone(),
            });
        }

        // ---- tool count ----
        let tools_count: TypedFunc<(), i32> = instance
            .get_export(&mut store, "plugin_tools_count")
            .and_then(|e| e.into_func())
            .ok_or_else(|| PluginError::MissingExport {
                export: "plugin_tools_count".into(),
            })?
            .typed(&store)
            .map_err(|e| PluginError::WrongExportType {
                export: "plugin_tools_count".into(),
                details: e.to_string(),
            })?;
        let count: u32 =
            tools_count
                .call(&mut store, ())
                .map_err(|e| PluginError::ToolExecution {
                    tool: "plugin_tools_count".into(),
                    details: e.to_string(),
                })? as u32;

        // ---- tool name / description functions ----
        let tool_name_fn: TypedFunc<i32, i32> = instance
            .get_export(&mut store, "plugin_tool_name")
            .and_then(|e| e.into_func())
            .ok_or_else(|| PluginError::MissingExport {
                export: "plugin_tool_name".into(),
            })?
            .typed(&store)
            .map_err(|e| PluginError::WrongExportType {
                export: "plugin_tool_name".into(),
                details: e.to_string(),
            })?;

        let tool_desc_fn: TypedFunc<i32, i32> = instance
            .get_export(&mut store, "plugin_tool_description")
            .and_then(|e| e.into_func())
            .ok_or_else(|| PluginError::MissingExport {
                export: "plugin_tool_description".into(),
            })?
            .typed(&store)
            .map_err(|e| PluginError::WrongExportType {
                export: "plugin_tool_description".into(),
                details: e.to_string(),
            })?;

        // ---- tool_execute ----
        let tool_execute: TypedFunc<(i32, i32), i32> = instance
            .get_export(&mut store, "plugin_tool_execute")
            .and_then(|e| e.into_func())
            .ok_or_else(|| PluginError::MissingExport {
                export: "plugin_tool_execute".into(),
            })?
            .typed(&store)
            .map_err(|e| PluginError::WrongExportType {
                export: "plugin_tool_execute".into(),
                details: e.to_string(),
            })?;

        // ---- memory ----
        let memory: Memory = instance
            .get_export(&mut store, "memory")
            .and_then(|e| e.into_memory())
            .ok_or_else(|| PluginError::MissingExport {
                export: "memory".into(),
            })?;

        // ---- enumerate tools ----
        let mut tools = Vec::with_capacity(count as usize);
        for i in 0..count {
            let idx = i as i32;
            let name_ptr =
                tool_name_fn
                    .call(&mut store, idx)
                    .map_err(|e| PluginError::ToolExecution {
                        tool: "plugin_tool_name".into(),
                        details: e.to_string(),
                    })?;
            let desc_ptr =
                tool_desc_fn
                    .call(&mut store, idx)
                    .map_err(|e| PluginError::ToolExecution {
                        tool: "plugin_tool_description".into(),
                        details: e.to_string(),
                    })?;

            let t_name = Self::read_string_from_memory(&memory, &store, name_ptr as u32);
            let t_desc = Self::read_string_from_memory(&memory, &store, desc_ptr as u32);

            tools.push(PluginTool {
                name: t_name,
                description: t_desc,
                index: i,
            });
        }

        Ok(Self {
            name,
            version,
            description,
            tools,
            store,
            memory,
            tool_execute,
        })
    }

    /// Execute a tool by index, passing `input` as the argument string.
    /// Returns the plugin's output string.
    pub fn execute_tool(&mut self, tool_index: u32, input: &str) -> Result<String, PluginError> {
        // Write input into plugin memory at a known offset (after any existing data).
        let input_offset = self.reserve_memory(input.len())?;
        self.memory
            .write(&mut self.store, input_offset, input.as_bytes())
            .map_err(|e| PluginError::ToolExecution {
                tool: self.tools[tool_index as usize].name.clone(),
                details: format!("Failed to write input to plugin memory: {e}"),
            })?;

        let result_ptr = self
            .tool_execute
            .call(&mut self.store, (tool_index as i32, input_offset as i32))
            .map_err(|e| PluginError::ToolExecution {
                tool: self.tools[tool_index as usize].name.clone(),
                details: e.to_string(),
            })?;

        let output = Self::read_string_from_memory(&self.memory, &self.store, result_ptr as u32);
        Ok(output)
    }

    /// Register all plugin tools into a `ToolRegistry`.
    pub fn register_into_registry(&mut self, registry: &mut ToolRegistry) {
        let plugin_name = self.name.clone();
        for i in 0..self.tools.len() as u32 {
            let tool = self.tools[i as usize].clone();
            let wrapper = WasmPluginToolWrapper {
                plugin_name: plugin_name.clone(),
                tool_index: i,
                tool_name: tool.name.clone(),
                tool_description: tool.description.clone(),
            };
            registry.register(Arc::new(wrapper));
        }
    }

    // ---- helpers ----

    /// Read a null-terminated UTF-8 string from the plugin's linear memory.
    fn read_string_from_memory(memory: &Memory, store: &Store<()>, offset: u32) -> String {
        let data = memory.data(store);
        let mut end = offset as usize;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        String::from_utf8_lossy(&data[offset as usize..end]).to_string()
    }

    /// Call a zero-argument export that returns an `i32` pointer into memory,
    /// then read the string at that pointer.
    fn read_export_string(
        instance: &Instance,
        store: &mut Store<()>,
        export_name: &str,
    ) -> Result<String, PluginError> {
        let func: TypedFunc<(), i32> = instance
            .get_export(&mut *store, export_name)
            .and_then(|e| e.into_func())
            .ok_or_else(|| PluginError::MissingExport {
                export: export_name.into(),
            })?
            .typed(&*store)
            .map_err(|e| PluginError::WrongExportType {
                export: export_name.into(),
                details: e.to_string(),
            })?;

        let ptr = func
            .call(&mut *store, ())
            .map_err(|e| PluginError::ToolExecution {
                tool: export_name.into(),
                details: e.to_string(),
            })?;

        // Get memory to read the string
        let memory: Memory = instance
            .get_export(&mut *store, "memory")
            .and_then(|e| e.into_memory())
            .ok_or_else(|| PluginError::MissingExport {
                export: "memory".into(),
            })?;

        Ok(Self::read_string_from_memory(&memory, &*store, ptr as u32))
    }

    /// Find a free region in plugin memory large enough for `size` bytes.
    /// Uses a simple bump-allocator approach starting after the first 64 KB.
    fn reserve_memory(&mut self, size: usize) -> Result<usize, PluginError> {
        let current_size = self.memory.data_size(&self.store);
        let needed = current_size + size;
        let pages_needed = needed.div_ceil(65536);
        let current_pages = self.memory.size(&self.store) as usize;
        if pages_needed > current_pages {
            let delta = (pages_needed - current_pages) as u64;
            self.memory
                .grow(&mut self.store, delta)
                .map_err(PluginError::Other)?;
        }
        Ok(current_size)
    }
}

// ---------------------------------------------------------------------------
// WasmPluginToolWrapper — implements ToolImpl for a single plugin tool
// ---------------------------------------------------------------------------

struct WasmPluginToolWrapper {
    plugin_name: String,
    tool_index: u32,
    tool_name: String,
    #[allow(dead_code)]
    tool_description: String,
}

impl std::fmt::Debug for WasmPluginToolWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmPluginToolWrapper")
            .field("plugin_name", &self.plugin_name)
            .field("tool_index", &self.tool_index)
            .field("tool_name", &self.tool_name)
            .finish()
    }
}

#[async_trait::async_trait]
impl ToolImpl for WasmPluginToolWrapper {
    fn definition(&self) -> &ToolDefinition {
        // Return a static definition — this is a simplified approach.
        // In production, the definition would be cached.
        use std::sync::OnceLock;
        static DEF: OnceLock<ToolDefinition> = OnceLock::new();
        DEF.get_or_init(|| ToolDefinition {
            name: self.tool_name.clone(),
            description: self.tool_description.clone(),
            parameters: crate::tools::JsonSchema::object(
                std::collections::HashMap::new(),
                Vec::new(),
            ),
            requires_approval: false,
            category: crate::tools::ToolCategory::General,
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> ToolResultValue<ToolResult> {
        // We need the wasm bytes to re-instantiate. This simplified wrapper
        // doesn't have access to them. In production, the wrapper would store
        // the wasm bytes and re-instantiate on each call.
        Err(crate::tools::ToolError::ExecutionFailed(
            self.tool_name.clone(),
            "WASM plugin tool requires re-instantiation — not yet implemented".into(),
        ))
    }

    fn name(&self) -> &str {
        &self.tool_name
    }
}

// ---------------------------------------------------------------------------
// WasmPluginManager — manages multiple loaded plugins
// ---------------------------------------------------------------------------

/// Manages a collection of loaded WASM plugins.
pub struct WasmPluginManager {
    plugins: Vec<WasmPlugin>,
}

impl WasmPluginManager {
    /// Create a new empty plugin manager.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Load a plugin from a `.wasm` file and add it to the manager.
    pub fn load(&mut self, path: &std::path::Path) -> Result<&WasmPlugin, PluginError> {
        let plugin = WasmPlugin::load(path)?;
        self.plugins.push(plugin);
        Ok(self.plugins.last().unwrap())
    }

    /// Load a plugin from bytes and add it to the manager.
    pub fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<&WasmPlugin, PluginError> {
        let plugin = WasmPlugin::load_from_bytes(bytes)?;
        self.plugins.push(plugin);
        Ok(self.plugins.last().unwrap())
    }

    /// Get a reference to a loaded plugin by name.
    pub fn get(&self, name: &str) -> Option<&WasmPlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    /// Get a mutable reference to a loaded plugin by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut WasmPlugin> {
        self.plugins.iter_mut().find(|p| p.name == name)
    }

    /// Unload (remove) a plugin by name.
    pub fn unload(&mut self, name: &str) -> bool {
        let idx = self.plugins.iter().position(|p| p.name == name);
        if let Some(i) = idx {
            self.plugins.remove(i);
            true
        } else {
            false
        }
    }

    /// Iterate over all loaded plugins.
    pub fn iter(&self) -> impl Iterator<Item = &WasmPlugin> {
        self.plugins.iter()
    }

    /// Register all tools from all plugins into a `ToolRegistry`.
    pub fn register_all(&mut self, registry: &mut ToolRegistry) {
        for plugin in self.plugins.iter_mut() {
            plugin.register_into_registry(registry);
        }
    }

    /// Number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns `true` if no plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for WasmPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WasmPluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmPluginManager")
            .field("plugin_count", &self.plugins.len())
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal "hello" plugin in WAT format.
    const HELLO_PLUGIN_WAT: &str = r#"
    (module
      (memory (export "memory") 1 256)
      (data (i32.const 0) "Hello from WASM!\00")
      (data (i32.const 32) "0.1.0\00")
      (data (i32.const 64) "A friendly WASM plugin\00")
      (data (i32.const 128) "hello\00")
      (data (i32.const 160) "Returns a friendly greeting\00")
      (func (export "plugin_name") (result i32) i32.const 0)
      (func (export "plugin_version") (result i32) i32.const 32)
      (func (export "plugin_description") (result i32) i32.const 64)
      (func (export "plugin_tools_count") (result i32) i32.const 1)
      (func (export "plugin_tool_name") (param i32) (result i32)
        i32.const 128)
      (func (export "plugin_tool_description") (param i32) (result i32)
        i32.const 160)
      (func (export "plugin_tool_execute") (param i32 i32) (result i32)
        i32.const 0)
    )
    "#;

    /// A plugin with two tools: "hello" and "echo".
    const ECHO_PLUGIN_WAT: &str = r#"
    (module
      (memory (export "memory") 1 256)
      (data (i32.const 0) "Echo Plugin\00")
      (data (i32.const 32) "0.1.0\00")
      (data (i32.const 64) "A plugin that echoes input\00")
      (data (i32.const 128) "echo\00")
      (data (i32.const 160) "Echoes back the input string\00")
      (data (i32.const 192) "hello\00")
      (data (i32.const 224) "Returns a friendly greeting\00")
      (func (export "plugin_name") (result i32) i32.const 0)
      (func (export "plugin_version") (result i32) i32.const 32)
      (func (export "plugin_description") (result i32) i32.const 64)
      (func (export "plugin_tools_count") (result i32) i32.const 2)
      (func (export "plugin_tool_name") (param i32) (result i32)
        local.get 0
        if (result i32)
          i32.const 128
        else
          i32.const 192
        end)
      (func (export "plugin_tool_description") (param i32) (result i32)
        local.get 0
        if (result i32)
          i32.const 160
        else
          i32.const 224
        end)
      (func (export "plugin_tool_execute") (param i32 i32) (result i32)
        local.get 1)
    )
    "#;

    fn compile_wat(wat: &str) -> Vec<u8> {
        wat::parse_str(wat).expect("Failed to parse WAT")
    }

    #[test]
    fn test_plugin_load() {
        let wasm = compile_wat(HELLO_PLUGIN_WAT);
        let plugin = WasmPlugin::load_from_bytes(&wasm).expect("Failed to load plugin");
        assert_eq!(plugin.name, "Hello from WASM!");
        assert_eq!(plugin.version, "0.1.0");
        assert_eq!(plugin.description, "A friendly WASM plugin");
    }

    #[test]
    fn test_plugin_tools() {
        let wasm = compile_wat(HELLO_PLUGIN_WAT);
        let plugin = WasmPlugin::load_from_bytes(&wasm).expect("Failed to load plugin");
        assert_eq!(plugin.tools.len(), 1);
        assert_eq!(plugin.tools[0].name, "hello");
        assert_eq!(plugin.tools[0].description, "Returns a friendly greeting");
    }

    #[test]
    fn test_plugin_execute_hello() {
        let wasm = compile_wat(HELLO_PLUGIN_WAT);
        let mut plugin = WasmPlugin::load_from_bytes(&wasm).expect("Failed to load plugin");
        let result = plugin.execute_tool(0, "").expect("Failed to execute tool");
        assert_eq!(result, "Hello from WASM!");
    }

    #[test]
    fn test_plugin_execute_echo() {
        let wasm = compile_wat(ECHO_PLUGIN_WAT);
        let mut plugin = WasmPlugin::load_from_bytes(&wasm).expect("Failed to load plugin");
        let result = plugin
            .execute_tool(1, "Hello world")
            .expect("Failed to execute tool");
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_plugin_manager() {
        let wasm = compile_wat(HELLO_PLUGIN_WAT);
        let mut manager = WasmPluginManager::new();
        manager.load_from_bytes(&wasm).expect("Failed to load");
        assert_eq!(manager.len(), 1);
        let plugin = manager.get("Hello from WASM!").expect("Plugin not found");
        assert_eq!(plugin.tools.len(), 1);
    }

    #[test]
    fn test_plugin_manager_execute() {
        let wasm = compile_wat(ECHO_PLUGIN_WAT);
        let mut manager = WasmPluginManager::new();
        manager.load_from_bytes(&wasm).expect("Failed to load");
        let plugin = manager.get_mut("Echo Plugin").expect("Plugin not found");
        let result = plugin
            .execute_tool(0, "test echo")
            .expect("Failed to execute");
        assert_eq!(result, "test echo");
    }

    #[test]
    fn test_plugin_unload() {
        let wasm = compile_wat(HELLO_PLUGIN_WAT);
        let mut manager = WasmPluginManager::new();
        manager.load_from_bytes(&wasm).expect("Failed to load");
        assert_eq!(manager.len(), 1);
        assert!(manager.unload("Hello from WASM!"));
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_plugin_unload_nonexistent() {
        let mut manager = WasmPluginManager::new();
        assert!(!manager.unload("nonexistent"));
    }

    #[test]
    fn test_plugin_version_mismatch() {
        let wat = r#"
        (module
          (memory (export "memory") 1)
          (data (i32.const 0) "Empty\00")
          (data (i32.const 32) "\00")
          (data (i32.const 64) "No version\00")
          (func (export "plugin_name") (result i32) i32.const 0)
          (func (export "plugin_version") (result i32) i32.const 32)
          (func (export "plugin_description") (result i32) i32.const 64)
          (func (export "plugin_tools_count") (result i32) i32.const 0)
        )
        "#;
        let wasm = compile_wat(wat);
        let result = WasmPlugin::load_from_bytes(&wasm);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, PluginError::VersionMismatch { .. }),
            "Expected VersionMismatch, got {err}"
        );
    }

    #[test]
    fn test_plugin_missing_export() {
        let wat = r#"
        (module
          (memory (export "memory") 1)
        )
        "#;
        let wasm = compile_wat(wat);
        let result = WasmPlugin::load_from_bytes(&wasm);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, PluginError::MissingExport { .. }),
            "Expected MissingExport, got {err}"
        );
    }

    #[test]
    fn test_plugin_default_manager() {
        let manager = WasmPluginManager::default();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }
}
