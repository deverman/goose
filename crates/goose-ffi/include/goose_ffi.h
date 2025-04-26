#ifndef GOOSE_FFI_H
#define GOOSE_FFI_H

/* Goose FFI - C interface for the Goose AI agent framework */


#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/*
 Role enum for message participants
 */
enum goose_MessageRole {
  /*
   User message role
   */
  goose_MessageRole_User = 0,
  /*
   Assistant message role
   */
  goose_MessageRole_Assistant = 1,
  /*
   System message role
   */
  goose_MessageRole_System = 2,
};
typedef uint32_t goose_MessageRole;

/*
 Provider Type enumeration
 Currently only Databricks is supported
 */
enum goose_ProviderType {
  /*
   Databricks AI provider
   */
  goose_ProviderType_Databricks = 0,
};
typedef uint32_t goose_ProviderType;

/*
 Result type for async operations

 - succeeded: true if the operation succeeded, false otherwise
 - error_message: Error message if succeeded is false, NULL otherwise
 */
typedef struct goose_AsyncResult {
  bool succeeded;
  char *error_message;
} goose_AsyncResult;

/*
 Pointer type for the agent
 */
typedef goose_Agent *goose_AgentPtr;

/*
 Provider configuration used to initialize an AI provider

 - provider_type: Provider type (0 = Databricks, other values will produce an error)
 - api_key: Provider API key (null for default from environment variables)
 - model_name: Model name to use (null for provider default)
 - host: Provider host URL (null for default from environment variables)
 */
typedef struct goose_ProviderConfigFFI {
  goose_ProviderType provider_type;
  const char *api_key;
  const char *model_name;
  const char *host;
} goose_ProviderConfigFFI;

/*
 Completion response structure

 - content: JSON string containing the completion response
 - succeeded: true if the operation succeeded, false otherwise
 - error_message: Error message if succeeded is false, NULL otherwise
 */
typedef struct goose_CompletionResponseFFI {
  char *content;
  bool succeeded;
  char *error_message;
} goose_CompletionResponseFFI;

/*
 Message structure for agent interactions

 - role: Message role (User, Assistant, or System)
 - content: Text content of the message
 */
typedef struct goose_MessageFFI {
  goose_MessageRole role;
  const char *content;
} goose_MessageFFI;

/*
 Tool definition for use with completion

 - name: Tool name
 - description: Tool description
 - input_schema_json: JSON schema for the tool's input parameters
 */
typedef struct goose_ToolFFI {
  const char *name;
  const char *description;
  const char *input_schema_json;
} goose_ToolFFI;

/*
 Extension definition for use with completion

 - name: Extension name
 - instructions: Optional instructions for the extension (can be NULL)
 - tools: Array of ToolFFI structures
 - tool_count: Number of tools in the array
 */
typedef struct goose_ExtensionFFI {
  const char *name;
  const char *instructions;
  const struct goose_ToolFFI *tools;
  uintptr_t tool_count;
} goose_ExtensionFFI;

/*
 Free an async result structure

 This function frees the memory allocated for an AsyncResult structure,
 including any error message it contains.

 # Safety

 The result pointer must be a valid pointer returned by a goose FFI function,
 or NULL.
 */
void goose_free_async_result(struct goose_AsyncResult *result);

/*
 Create a new agent with the given provider configuration

 # Parameters

 - config: Provider configuration

 # Returns

 A new agent pointer, or a null pointer if creation failed

 # Safety

 The config pointer must be valid or NULL. The resulting agent must be freed
 with goose_agent_free when no longer needed.
 */
goose_AgentPtr goose_agent_new(const struct goose_ProviderConfigFFI *config);

/*
 Free an agent

 This function frees the memory allocated for an agent.

 # Parameters

 - agent_ptr: Agent pointer returned by goose_agent_new

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new,
 or have a null internal pointer. The agent_ptr must not be used after
 calling this function.
 */
void goose_agent_free(goose_AgentPtr agent_ptr);

/*
 Send a message to the agent and get the response

 This function sends a message to the agent and returns the response.
 Tool handling is not yet supported and will be implemented in a future commit
 so this may change significantly

 # Parameters

 - agent_ptr: Agent pointer
 - message: Message to send

 # Returns

 A C string with the agent's response, or NULL on error.
 This string must be freed with goose_free_string when no longer needed.

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The message must be a valid C string.
 */
char *goose_agent_send_message(goose_AgentPtr agent_ptr, const char *message);

/*
 Free a string allocated by goose FFI functions

 This function frees memory allocated for strings returned by goose FFI functions.

 # Parameters

 - s: String to free

 # Safety

 The string must have been allocated by a goose FFI function, or be NULL.
 The string must not be used after calling this function.
 */
void goose_free_string(char *s);

/*
 Free a completion response structure

 This function frees the memory allocated for a CompletionResponseFFI structure,
 including any content and error message it contains.

 # Safety

 The response pointer must be a valid pointer returned by a goose FFI function,
 or NULL.
 */
void goose_free_completion_response(struct goose_CompletionResponseFFI *response);

/*
 Perform a completion request

 This function sends a completion request to the specified provider and returns
 the response.

 # Parameters

 - provider: Provider name (e.g., "databricks", "anthropic")
 - model_name: Model name to use
 - host: Provider host URL (NULL for default from environment variables)
 - api_key: Provider API key (NULL for default from environment variables)
 - system_preamble: System preamble text
 - messages: Array of MessageFFI structures
 - message_count: Number of messages in the array
 - extensions: Array of ExtensionFFI structures
 - extension_count: Number of extensions in the array

 # Returns

 A CompletionResponseFFI structure containing the response or error.
 This must be freed with goose_free_completion_response when no longer needed.

 # Safety

 All string parameters must be valid C strings or NULL.
 The messages array must contain valid MessageFFI structures.
 The extensions array must contain valid ExtensionFFI structures.
 */
struct goose_CompletionResponseFFI *goose_completion(const char *provider,
                                                     const char *model_name,
                                                     const char *host,
                                                     const char *api_key,
                                                     const char *system_preamble,
                                                     const struct goose_MessageFFI *messages_ptr,
                                                     uintptr_t message_count,
                                                     const struct goose_ExtensionFFI *extensions_ptr,
                                                     uintptr_t extension_count);

#endif // GOOSE_FFI_H
