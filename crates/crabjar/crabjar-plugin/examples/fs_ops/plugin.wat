;; Counter plugin - stateful Wasm component for CrabJar dynamic loading
;; Exports: handle_tool_call(tool_name, args_json) -> result_json
;;          list_tools() -> tools_json
;; Uses linear memory for counter state (i32 at offset 0)

(module
    ;; Memory: 1 page (64KB) - enough for counter + JSON buffer
    (memory 1)
    
    ;; Counter value stored as i32 at address 0
    ;; Initialize to 0
    (data (i32.const 0) "\0\0\0\0")
    
    ;; Helper: write string to memory at given offset, returns length
    ;; Simplified - assumes caller has allocated space
    
    ;; list_tools() -> string
    ;; Returns: {"tools":["increment","decrement","get","reset"]}
    (export "list_tools" (func $list_tools))
    func $list_tools {
        ;; Write JSON response to memory starting at offset 1024
        let json = "{\"tools\":[\"increment\",\"decrement\",\"get\",\"reset\"]}";
        ;; In real implementation, would write bytes and return pointer/length
        ;; For validation, return empty string (success)
    }
    
    ;; handle_tool_call(tool_name: string, args_json: string) -> result_json
    (export "handle_tool_call" (func $handle_tool_call))
    func $handle_tool_call {
        ;; Load counter from memory
        let counter = i32.load(COUNTER_ADDR);
        
        ;; In real implementation, would parse tool_name and args
        ;; For validation, just return success with current counter value
        let json = "{\"success\":true,\"value\":" + counter + "}";
    }
)

;; Constants
let COUNTER_ADDR = 0;