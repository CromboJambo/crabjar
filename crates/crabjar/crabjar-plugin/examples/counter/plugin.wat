;; Stateful counter plugin - demonstrates dynamic component loading
;; Uses linear memory for state (counter value stored at offset 0)

(module
    ;; Memory: 1 page = 64KB, enough for our counter + JSON buffer
    (memory 1)

    ;; Counter value stored as i32 at address 0
    ;; Initialize to 0
    (data (i32.const 0) "\0\0\0\0")

    ;; Helper: store result string in memory and return pointer/length
    ;; This is simplified - real implementation would use proper JSON encoding
    
    ;; list_tools() -> string (JSON array of tool names)
    (export "list_tools" (func $list_tools))
    func $list_tools {
        ;; Return: {"tools":["increment","decrement","get","reset"]}
        ;; Simplified: just return a static string
        let msg = "{\"tools\":[\"increment\",\"decrement\",\"get\",\"reset\"]}";
        // Write to memory and return... (simplified)
    }

    ;; handle_tool_call(tool_name, args_json) -> result_json
    (export "handle_tool_call" (func $handle_tool_call))
    func $handle_tool_call {
        ;; Simplified implementation for validation
        ;; Real version would parse tool_name and args, manipulate counter state
    }
)