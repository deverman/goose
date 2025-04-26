use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use goose::message::Message;
use goose::model::ModelConfig;
use goose_llm::{completion, Extension};
use mcp_core::tool::Tool;
use once_cell::sync::OnceCell;
use serde_json::json;
use tokio::runtime::Runtime;

// Thread-safe global runtime
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

/// Get or initialize the global runtime
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        // Multi-threaded runtime with all features enabled
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

/// Provider configuration used to initialize an AI provider
#[repr(C)]
pub struct GooseFfi2Config {
    /// Provider name (e.g., "openai", "anthropic", "databricks")
    pub provider: *const c_char,
    /// Model name to use (e.g., "gpt-4", "claude-3-opus")
    pub model_name: *const c_char,
    /// System preamble text
    pub system_preamble: *const c_char,
}

/// Message role enum
#[repr(u32)]
pub enum MessageRole {
    User = 0,
    Assistant = 1,
    System = 2,
}

/// Tool parameter used for constructing tools
#[repr(C)]
pub struct ToolParameter {
    /// Parameter name
    pub name: *const c_char,
    /// Parameter description
    pub description: *const c_char,
    /// Parameter type (string, number, boolean, array, object)
    pub param_type: *const c_char,
    /// Whether the parameter is required
    pub required: bool,
}

/// Tool definition used for constructing tools
#[repr(C)]
pub struct ToolDefinition {
    /// Tool name
    pub name: *const c_char,
    /// Tool description
    pub description: *const c_char,
    /// Parameters array (NULL-terminated)
    pub parameters: *const ToolParameter,
    /// Number of parameters
    pub parameter_count: c_int,
}

/// Extension that contains tools
#[repr(C)]
pub struct GooseFfi2Extension {
    /// Extension name
    pub name: *const c_char,
    /// Extension instructions (can be null)
    pub instructions: *const c_char,
    /// Tools array (NULL-terminated)
    pub tools: *const ToolDefinition,
    /// Number of tools
    pub tool_count: c_int,
}

/// Message structure from C/Kotlin to Rust
#[repr(C)]
pub struct GooseFfi2Message {
    /// Message role (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: *const c_char,
}

/// Result structure for completion operations
#[repr(C)]
#[derive(Debug)]
pub struct GooseFfi2Result {
    /// True if the operation succeeded, false if it failed
    pub success: bool,
    /// Response data if success is true, null otherwise
    pub data: *mut c_char,
    /// Error message if success is false, null otherwise
    pub error: *mut c_char,
}

/// Safe conversion from C strings to Rust Strings
unsafe fn c_str_to_string(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Null pointer provided".to_string());
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(String::from)
        .map_err(|e| format!("Invalid UTF-8 string: {}", e))
}

/// Convert Rust string to C string (caller must free with goose_ffi2_free_string)
fn string_to_c_char(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Create a tool from the C tool definition
unsafe fn create_tool(tool_def: &ToolDefinition) -> Result<Tool, String> {
    let name = c_str_to_string(tool_def.name)?;
    let description = c_str_to_string(tool_def.description)?;
    
    // Create properties object for the schema
    let mut properties = json!({});
    let mut required = Vec::new();
    
    // Add parameters to the schema
    for i in 0..tool_def.parameter_count {
        let param = &*tool_def.parameters.add(i as usize);
        let param_name = c_str_to_string(param.name)?;
        let param_description = c_str_to_string(param.description)?;
        let param_type = c_str_to_string(param.param_type)?;
        
        // Add to required array if needed
        if param.required {
            required.push(param_name.clone());
        }
        
        // Add to properties object
        let param_schema = json!({
            "type": param_type,
            "description": param_description
        });
        
        // Add the parameter to properties
        if let Some(props) = properties.as_object_mut() {
            props.insert(param_name, param_schema);
        }
    }
    
    // Construct the full schema
    let schema = json!({
        "type": "object",
        "required": required,
        "properties": properties
    });
    
    // Create and return the tool
    Ok(Tool::new(name, description, schema, None))
}

/// Create an extension from the C extension definition
unsafe fn create_extension(ext_def: &GooseFfi2Extension) -> Result<Extension, String> {
    let name = c_str_to_string(ext_def.name)?;
    
    // Instructions are optional
    let instructions = if ext_def.instructions.is_null() {
        None
    } else {
        Some(c_str_to_string(ext_def.instructions)?)
    };
    
    // Create tools
    let mut tools = Vec::new();
    for i in 0..ext_def.tool_count {
        let tool_def = &*ext_def.tools.add(i as usize);
        let tool = create_tool(tool_def)?;
        tools.push(tool);
    }
    
    Ok(Extension::new(name, instructions, tools))
}

/// Create a Message from the C message structure
unsafe fn create_message(msg: &GooseFfi2Message) -> Result<Message, String> {
    let content = c_str_to_string(msg.content)?;
    
    let message = match msg.role {
        MessageRole::User => Message::user().with_text(&content),
        MessageRole::Assistant => Message::assistant().with_text(&content),
        MessageRole::System => {
            // System role is not supported in mcp_core::role::Role
            // We'll use User role as a workaround
            Message::user().with_text(&content)
        },
    };
    
    Ok(message)
}

/// Perform a completion with the given configuration, messages, and extensions
///
/// # Safety
///
/// This function uses raw pointers and should be called with valid pointers.
/// - config must be a valid pointer to a GooseFfi2Config structure
/// - messages must be a valid array of GooseFfi2Message structures with count message_count
/// - extensions must be a valid array of GooseFfi2Extension structures with count extension_count
#[no_mangle]
pub unsafe extern "C" fn goose_ffi2_completion(
    config: *const GooseFfi2Config,
    messages: *const GooseFfi2Message,
    message_count: c_int,
    extensions: *const GooseFfi2Extension,
    extension_count: c_int,
) -> GooseFfi2Result {
    println!("Starting completion...");
    // Validate inputs
    if config.is_null() {
        return GooseFfi2Result {
            success: false,
            data: ptr::null_mut(),
            error: string_to_c_char("Configuration is null".to_string()),
        };
    }
    if messages.is_null() && message_count > 0 {
        return GooseFfi2Result {
            success: false,
            data: ptr::null_mut(),
            error: string_to_c_char("Messages array is null but count is non-zero".to_string()),
        };
    }
    if extensions.is_null() && extension_count > 0 {
        return GooseFfi2Result {
            success: false,
            data: ptr::null_mut(),
            error: string_to_c_char("Extensions array is null but count is non-zero".to_string()),
        };
    }
    // Convert config
    let config = &*config;
    let provider = match c_str_to_string(config.provider) {
        Ok(s) => s,
        Err(e) => {
            return GooseFfi2Result {
                success: false,
                data: ptr::null_mut(),
                error: string_to_c_char(format!("Invalid provider: {}", e)),
            };
        }
    };
    let model_name = match c_str_to_string(config.model_name) {
        Ok(s) => s,
        Err(e) => {
            return GooseFfi2Result {
                success: false,
                data: ptr::null_mut(),
                error: string_to_c_char(format!("Invalid model name: {}", e)),
            };
        }
    };
    let system_preamble = match c_str_to_string(config.system_preamble) {
        Ok(s) => s,
        Err(e) => {
            return GooseFfi2Result {
                success: false,
                data: ptr::null_mut(),
                error: string_to_c_char(format!("Invalid system preamble: {}", e)),
            };
        }
    };
    // Convert messages
    let mut rust_messages = Vec::new();
    for i in 0..message_count {
        let msg = &*messages.add(i as usize);
        match create_message(msg) {
            Ok(message) => rust_messages.push(message),
            Err(e) => {
                return GooseFfi2Result {
                    success: false,
                    data: ptr::null_mut(),
                    error: string_to_c_char(format!("Failed to process message {}: {}", i, e)),
                };
            }
        }
    }
    
    // Convert extensions
    let mut rust_extensions = Vec::new();
    for i in 0..extension_count {
        let ext = &*extensions.add(i as usize);
        match create_extension(ext) {
            Ok(extension) => rust_extensions.push(extension),
            Err(e) => {
                return GooseFfi2Result {
                    success: false,
                    data: ptr::null_mut(),
                    error: string_to_c_char(format!("Failed to process extension {}: {}", i, e)),
                };
            }
        }
    }
    
    let model_config = ModelConfig::new(model_name);
    
    // Run the completion using our Tokio runtime
    let result = get_runtime().block_on(async {
        match completion(
            &provider,
            model_config,
            &system_preamble,
            &rust_messages,
            &rust_extensions,
        )
        .await
        {
            Ok(response) => {
                // Serialize to JSON for FFI boundary crossing
                match serde_json::to_string(&response) {
                    Ok(json) => GooseFfi2Result {
                        success: true,
                        data: string_to_c_char(json),
                        error: ptr::null_mut(),
                    },
                    Err(e) => GooseFfi2Result {
                        success: false,
                        data: ptr::null_mut(),
                        error: string_to_c_char(format!("Failed to serialize response: {}", e)),
                    },
                }
            }
            Err(e) => GooseFfi2Result {
                success: false,
                data: ptr::null_mut(),
                error: string_to_c_char(format!("Completion error: {}", e)),
            },
        }
    });
    result
}

/// Free a string allocated by the FFI layer
///
/// # Safety
///
/// This function uses raw pointers and should be called with pointers returned
/// by this library. The pointer should not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn goose_ffi2_free_string(string: *mut c_char) {
    if !string.is_null() {
        let _ = CString::from_raw(string);
    }
}

/// Free a result structure allocated by the FFI layer
///
/// # Safety
///
/// This function uses raw pointers and should be called with result structs returned
/// by this library. The result should not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn goose_ffi2_free_result(result: GooseFfi2Result) {
    goose_ffi2_free_string(result.data);
    goose_ffi2_free_string(result.error);
}

/// Version information
#[no_mangle]
pub extern "C" fn goose_ffi2_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    string_to_c_char(format!("goose-ffi2 version {}", version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_string_conversion() {
        let test_str = "Hello, World!";
        let c_str = CString::new(test_str).unwrap();
        let c_ptr = c_str.as_ptr();
        
        unsafe {
            let converted = c_str_to_string(c_ptr).unwrap();
            assert_eq!(converted, test_str);
        }
    }
    
    #[test]
    fn test_runtime_initialization() {
        // Just verify we can get the runtime without panic
        let _runtime = get_runtime();
    }
    
    #[test]
    fn test_version() {
        let version_ptr = goose_ffi2_version();
        assert!(!version_ptr.is_null());
        
        unsafe {
            let version_str = CStr::from_ptr(version_ptr).to_string_lossy();
            assert!(version_str.contains("goose-ffi2 version"));
            goose_ffi2_free_string(version_ptr);
        }
    }
    
    #[test]
    fn test_thread_safety() {
        // Test with multiple threads calling the version function
        let mut handles = Vec::new();
        
        for _ in 0..5 {
            let handle = thread::spawn(|| {
                unsafe {
                    let version = goose_ffi2_version();
                    assert!(!version.is_null());
                    
                    let version_str = CStr::from_ptr(version).to_string_lossy();
                    assert!(version_str.contains("goose-ffi2 version"));
                    
                    goose_ffi2_free_string(version);
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    // This is a simpler version of the ffi_simple example
    #[test]
    fn test_simple_ffi_usage() {
        unsafe {
            let version = goose_ffi2_version();
            assert!(!version.is_null());
            
            let version_str = CStr::from_ptr(version).to_string_lossy();
            assert!(version_str.contains("goose-ffi2 version"));
            
            goose_ffi2_free_string(version);
        }
    }
}
