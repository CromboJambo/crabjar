# Spec: Complete REA-Style Analyzer Crates for crabjar-analyze

## Overview

Implement the remaining analyzer crates to make `crabjar-analyze` a complete reverse engineering toolkit. Currently only binary analysis (ELF) works; six more analyzers need implementation.

## Architecture

Each analyzer is a separate crate that implements a common trait and registers itself with the CLI dispatcher:

```hemlock
trait Analyzer {
    fn analyze(&self, path: &Path, options: AnalyzeOptions) -> Result<AnalysisResult>;
    fn supported_extensions(&self) -> Vec<&str>;
}

// Registration via macro or manual dispatch table
ANALYZERS = [
    BinaryAnalyzer {},
    HarAnalyzer {},
    JsAnalyzer {},
    ApkAnalyzer {},
    FirmwareAnalyzer {},
    DotnetAnalyzer {},
];
```

## Crate Structure

```hemlock
crabjar/
├── crabjar-analyze/              # CLI binary + dispatcher (done)
├── analyze-common/               # Shared types, evidence model, Analyzer trait
├── analyze-native/               # ELF/Mach-O/PE parsing (partially done)
├── analyze-har/                  # HAR file parsing
├── analyze-js/                   # JavaScript/Electron inspection
├── analyze-apk/                  # Android APK teardown
├── analyze-firmware/             # Firmware extraction + analysis
└── analyze-dotnet/               # .NET assembly inspection
```

## Task 1: analyze-common Crate

**Goal:** Define shared types, evidence model, and Analyzer trait.

### Types to Define

```hemlock
// Analysis result envelope
struct AnalysisResult {
    status: "success" | "error",
    elapsed_ms: u64,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

// Provenance tracking (what tool produced this)
struct Provenance {
    analyzer: String,       // "analyze-har", "analyze-native", etc.
    version: String,        // Crate version
    timestamp: DateTime<Utc>,
    input_path: PathBuf,
}

// Doubt block — limitations of the analysis
struct DoubtBlock {
    confidence: f32,        // 0.0-1.0
    limitations: Vec<String>,
    assumptions: Vec<String>,
}

// Analyzer trait
trait Analyzer {
    fn analyze(&self, path: &Path, options: AnalyzeOptions) -> Result<AnalysisResult>;
    fn supported_extensions(&self) -> Vec<&str>;
    fn name(&self) -> &'static str;
}

// Common options
struct AnalyzeOptions {
    verbose: bool,
    include_strings: bool,
    include_symbols: bool,
    // ... analyzer-specific flags via enum variant or serde_json::Value
}
```

### Dependencies
- serde, serde_json (workspace)
- chrono (timestamp)
- anyhow (error handling)

## Task 2: Complete analyze-native Crate

**Goal:** Finish binary analysis features. Currently only ELF symbol extraction works.

### Features to Implement

1. **String extraction (`--strings` flag)**
   - Scan .rodata and other sections for printable strings
   - Minimum length filter (default 4 chars)
   - Encoding detection (UTF-8 vs ASCII)
   - Return top N longest/most interesting strings

2. **Section enumeration (`--sections` flag)**
   - List all ELF sections with sizes, types, permissions
   - Flag suspicious sections (writable code, executable data)
   - Detect stripped binaries (missing .symtab)

3. **Import/Export analysis**
   - Parse PLT/GOT for imported functions
   - Identify dynamic library dependencies (DT_NEEDED entries)
   - Cross-reference with known libc symbols

4. **Basic entropy analysis**
   - Calculate Shannon entropy per section
   - Flag high-entropy sections (possible encrypted/packed code)

### Dependencies
- None new (pure Rust ELF parsing)

## Task 3: analyze-har Crate

**Goal:** Parse HTTP Archive files and extract request/response patterns.

### Features to Implement

1. **HAR file parsing**
   - Load and validate HAR JSON structure
   - Extract log entries (pages, resources, timings)
   - Handle HAR 1.2 format specification

2. **Request/Response analysis**
   - Count requests by domain
   - Identify suspicious endpoints (/api/v1/users, /admin, etc.)
   - Detect hardcoded credentials in URLs or bodies
   - Flag unusual HTTP methods (PATCH, OPTIONS, TRACE)

3. **Timing analysis**
   - Calculate average response times per endpoint
   - Identify slowest requests
   - Detect timing anomalies (possible DoS patterns)

4. **Content inspection**
   - Parse JSON payloads for sensitive data patterns
   - Extract API version strings
   - Identify framework-specific headers (X-Powered-By, etc.)

### Dependencies
- serde_json (HAR parsing)
- regex (pattern matching)

## Task 4: analyze-js Crate

**Goal:** Inspect JavaScript/Electron applications for structure and patterns.

### Features to Implement

1. **Bundle analysis**
   - Identify bundled frameworks (React, Vue, Angular signatures)
   - Detect webpack/Vite/rollup bundles by magic strings
   - Count modules in bundle

2. **Dependency graph extraction**
   - Parse package.json for dependencies
   - Identify dev vs production deps
   - Flag outdated or known-vulnerable packages (basic version check)

3. **Electron inspection**
   - Detect Electron main process code
   - Identify IPC patterns (ipcMain, ipcRenderer usage)
   - Extract native module references

4. **API endpoint extraction**
   - Find fetch/axios calls in source
   - Extract URL patterns and method types
   - Flag hardcoded API keys or tokens

### Dependencies
- regex (pattern matching)
- Possibly a JS parser crate for AST analysis (optional, complex)

## Task 5: analyze-apk Crate

**Goal:** Tear down Android APK packages.

### Features to Implement

1. **APK structure inspection**
   - List files in APK without full extraction
   - Identify manifest location and version
   - Check signature validity (basic check)

2. **Manifest parsing**
   - Extract package name, version code/name
   - List declared activities, services, receivers
   - Identify permissions requested
   - Find exported components (attack surface)

3. **Class enumeration**
   - Parse classes.dex for class names
   - Identify obfuscated vs real class names
   - Count methods per class (basic complexity metric)

4. **Resource inventory**
   - List image resources
   - Identify string resources
   - Flag large assets

### Dependencies
- zip crate (APK is a ZIP file)
- AndroidManifest.xml parsing (XML parser)

## Task 6: analyze-firmware Crate

**Goal:** Extract and analyze firmware images.

### Features to Implement

1. **File magic detection**
   - Identify firmware format by header bytes
   - Support common formats: raw, FIT image, U-Boot, etc.
   - Detect compression (gzip, lzma, xz)

2. **Region identification**
   - Parse FIT image headers for partition layout
   - Identify bootloader vs kernel vs rootfs regions
   - Extract region offsets and sizes

3. **Integration with external tools**
   - Spawn binwalk for deep firmware analysis
   - Parse binwalk output and present structured results
   - Handle timeouts and error states

4. **Extraction handoffs**
   - Write extracted regions to temp files
   - Provide paths for downstream analyzers (e.g., analyze-native on extracted binaries)

### Dependencies
- tempfile crate
- Process spawning for external tools
- Possibly binwalk-rs or similar library

## Task 7: analyze-dotnet Crate

**Goal:** Inspect .NET assemblies.

### Features to Implement

1. **PE header parsing**
   - Validate PE structure
   - Extract machine type, entry point
   - Identify CLR version required

2. **Metadata table enumeration**
   - List types (classes, interfaces, structs)
   - Extract method signatures
   - Identify assembly references

3. **IL instruction inspection**
   - Basic disassembly of critical methods
   - Identify string literals in IL
   - Detect reflection usage patterns

### Dependencies
- PE parsing library (pefile-rs or similar)
- Possibly IKVM.NET for deeper analysis (complex)

## Integration with CLI

Each analyzer crate registers its Analyzer implementation with the main CLI binary. The dispatcher matches file extensions to analyzers:

```hemlock
// In crabjar-analyze/src/main.rs
let analyzers = vec![
    Box::new(BinaryAnalyzer {}),
    Box::new(HarAnalyzer {}),
    Box::new(JsAnalyzer {}),
    // ... etc
];

let analyzer = find_analyzer(&path, &analyzers)?;
let result = analyzer.analyze(&path, options)?;
```

## Success Criteria

1. All six analyzers implement the Analyzer trait and register with CLI
2. Each analyzer handles its file types correctly (tested with real samples)
3. Evidence model (Provenance + DoubtBlock) included in all results
4. Structured JSON output consistent across all analyzers
5. Guard system integration for external tool invocations

## Testing Strategy

For each analyzer, create test fixtures:
- `analyze-har`: Sample HAR files from real web traffic
- `analyze-js`: Bundled React app, Electron app
- `analyze-apk`: Sample APKs (free Android apps)
- `analyze-firmware`: Public firmware images (router firmware)
- `analyze-dotnet`: Simple .NET console app assemblies

Run analyzers against fixtures and validate output structure.
