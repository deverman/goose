#ifndef GOOSE_FFI2_H
#define GOOSE_FFI2_H

/* Goose FFI2 - C interface for the Goose LLM completion API */
/* This library is thread-safe and can be called from multiple threads */


#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/*
 Message role enum
 */
enum goose_ffi2_MessageRole {
  goose_ffi2_MessageRole_User = 0,
  goose_ffi2_MessageRole_Assistant = 1,
  goose_ffi2_MessageRole_System = 2,
};
typedef uint32_t goose_ffi2_MessageRole;

/*
 Result structure for completion operations
 */
typedef struct goose_ffi2_GooseFfi2Result {
  /*
   True if the operation succeeded, false if it failed
   */
  bool success;
  /*
   Response data if success is true, null otherwise
   */
  char *data;
  /*
   Error message if success is false, null otherwise
   */
  char *error;
} goose_ffi2_GooseFfi2Result;

/*
 Provider configuration used to initialize an AI provider
 */
typedef struct goose_ffi2_GooseFfi2Config {
  /*
   Provider name (e.g., "openai", "anthropic", "databricks")
   */
  const char *provider;
  /*
   Model name to use (e.g., "gpt-4", "claude-3-opus")
   */
  const char *model_name;
  /*
   System preamble text
   */
  const char *system_preamble;
} goose_ffi2_GooseFfi2Config;

/*
 Message structure from C/Kotlin to Rust
 */
typedef struct goose_ffi2_GooseFfi2Message {
  /*
   Message role (user, assistant, system)
   */
  goose_ffi2_MessageRole role;
  /*
   Message content
   */
  const char *content;
} goose_ffi2_GooseFfi2Message;

/*
 Tool parameter used for constructing tools
 */
typedef struct goose_ffi2_ToolParameter {
  /*
   Parameter name
   */
  const char *name;
  /*
   Parameter description
   */
  const char *description;
  /*
   Parameter type (string, number, boolean, array, object)
   */
  const char *param_type;
  /*
   Whether the parameter is required
   */
  bool required;
} goose_ffi2_ToolParameter;

/*
 Tool definition used for constructing tools
 */
typedef struct goose_ffi2_ToolDefinition {
  /*
   Tool name
   */
  const char *name;
  /*
   Tool description
   */
  const char *description;
  /*
   Parameters array (NULL-terminated)
   */
  const struct goose_ffi2_ToolParameter *parameters;
  /*
   Number of parameters
   */
  int parameter_count;
} goose_ffi2_ToolDefinition;

/*
 Extension that contains tools
 */
typedef struct goose_ffi2_GooseFfi2Extension {
  /*
   Extension name
   */
  const char *name;
  /*
   Extension instructions (can be null)
   */
  const char *instructions;
  /*
   Tools array (NULL-terminated)
   */
  const struct goose_ffi2_ToolDefinition *tools;
  /*
   Number of tools
   */
  int tool_count;
} goose_ffi2_GooseFfi2Extension;

/*
 Perform a completion with the given configuration, messages, and extensions

 # Safety

 This function uses raw pointers and should be called with valid pointers.
 - config must be a valid pointer to a GooseFfi2Config structure
 - messages must be a valid array of GooseFfi2Message structures with count message_count
 - extensions must be a valid array of GooseFfi2Extension structures with count extension_count
 */
struct goose_ffi2_GooseFfi2Result goose_ffi2_completion(const struct goose_ffi2_GooseFfi2Config *config,
                                                        const struct goose_ffi2_GooseFfi2Message *messages,
                                                        int message_count,
                                                        const struct goose_ffi2_GooseFfi2Extension *extensions,
                                                        int extension_count);

/*
 Free a string allocated by the FFI layer

 # Safety

 This function uses raw pointers and should be called with pointers returned
 by this library. The pointer should not be used after this call.
 */
void goose_ffi2_free_string(char *string);

/*
 Free a result structure allocated by the FFI layer

 # Safety

 This function uses raw pointers and should be called with result structs returned
 by this library. The result should not be used after this call.
 */
void goose_ffi2_free_result(struct goose_ffi2_GooseFfi2Result result);

/*
 Version information
 */
char *goose_ffi2_version(void);

#endif // GOOSE_FFI2_H
