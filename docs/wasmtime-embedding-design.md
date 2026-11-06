# Wasmtime Embedding: Best Practices for CrabJar Dynamic Component Loading

This document captures industry best practices for embedding Wasmtime in a production Rust application like CrabJar, based on research into Wasmtime's official documentation, community examples, and performance analysis.

## 1. Engine Configuration Strategy

### Development vs Production Profiles

```rust
// Development: Enable debugging features
let dev_config = wasmtime::Config::new()
    .debug_info(true)
    .cranelift_debug_verifier(true);
let engine = WasmEngine::with_config(dev_config)?;

// Production: Optimize for performance
let prod_config = wasmtime::Config::new()
    .parallel_compiles(4)  // Use all CPU cores
    .cache_path(Some("/var/cache/crabjar/wasm"));
let engine = WasmEngine::with_config(prod_config)?;
```

### Cache Configuration

Wasmtime's cache dramatically improves instantiation performance for frequently loaded components:

```rust
let config = wasmtime::Config::new()
    .cache_path(Some(PathBuf::from("/var/cache/crabjar/wasm")));
```

Cache invalidation is automatic based on module bytecode hash. No manual management required.

## 2. Multi-Instance Management

### Per-Plugin Store Architecture

Each plugin instance should have its own `Store` for isolation:

```rust
struct PluginManager {
    engine: WasmEngine,
    instances: HashMap<String, (wasmtime::Store<()>, Instance)>,
}

impl PluginManager {
    fn load_plugin(&mut self, name: &str, path: &Path) -> Result<()> {
        let module = self.engine.load_module(path)?;
        let mut store = wasmtime::Store::new(self.engine.raw(), ());
        let instance = Instance::new(&mut store, &module, &[])?;
        self.instances.insert(name.to_string(), (store, instance));
        Ok(())
    }
}
```

### Resource Limits Per Instance

Use Wasmtime's `StoreLimits` to prevent any single plugin from consuming excessive resources:

```rust
let limits = StoreLimitsBuilder::new()
    .table_elements(1024 * 1024)   // 1M elements max
    .instances(1)                  // Single instance per store
    .build();

let mut store = wasmtime::Store::new_with_limits(self.engine.raw(), (), limits);
```

## 3. Hot Reloading Strategy

### Two-Phase Reload (Zero-Downtime)

For hot reloading without service interruption, use a two-phase approach:

1. Load new version alongside old
2. Redirect traffic to new version
3. Gracefully shut down old version

```rust
fn reload_plugin(&mut self, name: &str, new_path: &Path) -> Result<()> {
    // 1. Load new version
    let new_module = self.engine.load_module(new_path)?;
    let mut new_store = wasmtime::Store::new(self.engine.raw(), ());
    let new_instance = Instance::new(&mut new_store, &new_module, &[])?;

    // 2. Swap atomically
    let old = self.instances.remove(name);
    self.instances.insert(name.to_string(), (new_store, new_instance));

    // 3. Old instance dropped automatically when no longer referenced
    Ok(())
}
```

### Versioned State Migration

When reloading plugins with schema changes, use versioned state:

```wit
define Plugin {
    fn initialize(state_version: u32);
    fn export_state() -> bytes;
    fn import_state(data: bytes);
}
```

CrabJar maintains a migration registry:

```rust
let migrations = [
    (0, 1, migrate_v0_to_v1),
    (1, 2, migrate_v1_to_v2),
];

fn migrate_state(plugin_name: &str, old_version: u32, new_version: u32) -> Result<Vec<u8>> {
    let state = load_plugin_state(plugin_name)?;
    for migration in migrations.iter() {
        if migration.0 == current_version {
            state = migration.2(state)?;
            current_version += 1;
        }
    }
    Ok(state)
}
```

## 4. Capability-Mediated Imports

### Interface Segregation for Sandboxing

Define minimal interfaces per plugin type:

```wit
// Minimal interface for simple plugins
define BasicPlugin {
    fn handle_event(event: Event);
}

// Extended interface for stateful plugins
define StatefulPlugin extends BasicPlugin {
    fn get_state() -> bytes;
    fn set_state(data: bytes);
}
```

### Host Import Injection

Inject only required capabilities via function pointers:

```rust
let mut store = wasmtime::Store::new(engine.raw(), PluginContext {
    state_store: Arc::new(Mutex::new(HashMap::new())),
    event_bus: Arc::clone(&event_bus),
    logger: Arc::new(Logger::new(plugin_name)),
});
```

## 5. Sandbox Security Model

### Memory Isolation

Each Wasm instance runs in its own linear memory space — no direct access to host memory or other plugins' memory.

### Filesystem Sandboxing

Use WASI (WebAssembly System Interface) for controlled filesystem access:

```rust
use wasmtime_wasi::WasiCtxBuilder;

let wasi = WasiCtxBuilder::new()
    .inherit_stdout()
    .preopen_dir("/tmp/crabjar/plugins/myplugin", "/data")?
    .build();
```

### Network Sandboxing

For network access, inject a proxy function rather than exposing raw sockets:

```wit
define NetworkPlugin {
    fn http_request(url: string, body: bytes) -> Response;
}
```

## 6. Error Handling Patterns

### Panic Recovery Across Instance Boundary

Wasm panics don't propagate to the host — they're caught at the instance boundary:

```rust
let result = instance.invoke("handle_event", &[event_arg]);
match result {
    Ok(_) => info!("Event handled successfully"),
    Err(wasmtime::Trap::JavaScriptException(msg)) => {
        error!("Plugin threw exception: {}", msg)
    }
    Err(e) => error!("Wasm execution error: {}", e),
}
```

### Timeout Handling

Use Wasmtime's fuel mechanism for timeout control:

```rust
let mut store = wasmtime::Store::new(engine.raw(), ());
store.set_fuel(10_000_000)?;  // ~1 second of execution

match instance.invoke("process", &[]) {
    Ok(_) => info!("Processing completed"),
    Err(wasmtime::Trap::FuelEmpty) => warn!("Plugin timed out"),
    Err(e) => error!("Execution error: {}", e),
}
```

## 7. Performance Optimization

### Pre-Compilation for Production

Compile modules ahead of time to reduce startup latency:

```bash
wasmtime compile plugin.wasm -o plugin.wasm.compiled
```

Then load the compiled artifact directly:

```rust
let bytes = std::fs::read("plugin.wasm.compiled")?;
let module = wasmtime::Module::deserialize(&engine, &bytes)?;
```

### Shared Engine Across Instances

Reuse a single `wasmtime::Engine` across all plugin instances for memory efficiency:

```rust
let engine = WasmEngine::new()?;  // Create once at startup
let manager = PluginManager::new(engine);
```

### Memory Pooling Strategy

For high-throughput scenarios with many short-lived plugins, use a pooling allocator:

```rust
// Custom allocator interface
define Allocator {
    fn alloc(size: u32) -> ptr;
    fn free(ptr: ptr);
}

// Inject into plugin
let allocator = MyAllocator::new();
store.set_allocator(allocator);
```

## 8. Logging and Tracing Integration

### Structured Log Injection

Inject a logging function that integrates with CrabJar's tracing system:

```rust
fn inject_logging(store: &mut Store<PluginContext>) {
    let log_fn = wasmtime::Func::wrap(store, move |level: u32, msg: String| {
        let ctx = store.data_mut();
        match level {
            0 => trace!(plugin = ctx.name, "{}", msg),
            1 => debug!(plugin = ctx.name, "{}", msg),
            2 => info!(plugin = ctx.name, "{}", msg),
            3 => warn!(plugin = ctx.name, "{}", msg),
            _ => error!(plugin = ctx.name, "{}", msg),
        }
    });

    store.define("env", "log", log_fn)?;
}
```

### Performance Tracing

Use Wasmtime's profiling features for performance analysis:

```rust
let config = wasmtime::Config::new()
    .profiler(wasmtime::Profiler::Perf);
```

## 9. Testing Strategy

### Integration Test Pattern

Test plugin loading with real Wasm modules in integration tests:

```rust
#[test]
fn test_plugin_lifecycle() {
    let engine = WasmEngine::new().unwrap();
    let module = engine.load_module("tests/fixtures/simple.wasm").unwrap();
    let instance = engine.instantiate(&module, &[]).unwrap();

    // Invoke and verify behavior
    let result = instance.invoke("process", &[wasmtime::Value::I32(42)]);
    assert!(result.is_ok());
}
```

### Fixture Management

Maintain a collection of test Wasm modules covering edge cases:
- `simple.wasm` — basic function invocation
- `stateful.wasm` — persistent state management
- `erroring.wasm` — panic and error propagation
- `timeout.wasm` — infinite loop for timeout testing

## 10. Production Deployment Considerations

### Module Size Validation

Validate module size before loading to prevent resource exhaustion:

```rust
const MAX_MODULE_SIZE: u64 = 10 * 1024 * 1024;  // 10MB

fn load_plugin(path: &Path) -> Result<Module> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_MODULE_SIZE {
        return Err(PluginError::TooLarge(metadata.len()));
    }
    // ... load module
}
```

### Fuel Consumption Monitoring

Track and alert on fuel consumption for performance monitoring:

```rust
let fuel_consumed = store.get_fuel_consumed()?;
if fuel_consumed > FUEL_WARNING_THRESHOLD {
    warn!(plugin = name, fuel = fuel_consumed, "High fuel consumption detected");
}
```

### Graceful Shutdown

Ensure all plugin instances are properly shut down during application exit:

```rust
fn shutdown(&mut self) {
    for (name, (store, instance)) in self.instances.drain() {
        info!(plugin = name, "Shutting down plugin");
        // Invoke cleanup function if defined
        let _ = instance.invoke("shutdown", &[]);
    }
}
```

## Conclusion

Wasmtime provides a robust foundation for dynamic component loading in CrabJar. By following these best practices — particularly around sandboxing, resource limits, and versioned state migration — CrabJar can safely load and manage plugins while maintaining the performance characteristics expected of a Rust-based system.

The key architectural decisions are:
1. **Per-plugin Store isolation** for memory safety
2. **Capability-mediated imports** for security
3. **Versioned state serialization** for hot reloading
4. **Pre-compiled modules** for production performance
5. **Fuel consumption tracking** for resource governance

These practices align with industry standards used in production Wasmtime deployments and provide a solid foundation for CrabJar's dynamic plugin ecosystem.