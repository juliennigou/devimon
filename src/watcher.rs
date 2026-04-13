use crate::xp::{XpEvent, append_event};
use chrono::Utc;
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

/// Watch the given directory and append XP events for source code modifications.
/// Only files with recognised programming-language extensions earn XP.
pub fn watch(path: &Path) -> notify::Result<()> {
    println!("👀 Watching {} — press Ctrl+C to stop.", path.display());
    watch_inner(path)
}

/// Same as [`watch`] but without the stdout banner — used when the watcher
/// runs inside the TUI as a background thread.
pub fn watch_silent(path: &Path) -> notify::Result<()> {
    watch_inner(path)
}

fn watch_inner(path: &Path) -> notify::Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    let canonical_root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let mut last_logged: std::collections::HashMap<String, Instant> =
        std::collections::HashMap::new();
    let debounce = Duration::from_secs(2);

    for res in rx {
        match res {
            Ok(event) => {
                if !matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    continue;
                }
                for p in event.paths {
                    // Only process events inside the watched directory.
                    let canonical = p.canonicalize().unwrap_or_else(|_| p.clone());
                    if !canonical.starts_with(&canonical_root) {
                        continue;
                    }
                    if should_ignore(&p) || !is_source_file(&p) {
                        continue;
                    }
                    let path_str = p.to_string_lossy().to_string();
                    // Debounce repeated writes to the same file.
                    let now = Instant::now();
                    if let Some(t) = last_logged.get(&path_str) {
                        if now.duration_since(*t) < debounce {
                            continue;
                        }
                    }
                    last_logged.insert(path_str.clone(), now);

                    let ev = XpEvent {
                        kind: "file_modified".to_string(),
                        path: path_str,
                        timestamp: Utc::now(),
                    };
                    if let Err(e) = append_event(&ev) {
                        eprintln!("warn: failed to append event: {}", e);
                    }
                }
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }
    Ok(())
}

fn should_ignore(path: &Path) -> bool {
    let ignored_dirs = [
        ".git",
        "target",
        "node_modules",
        ".devimon",
        "dist",
        "build",
        ".next",
        ".cache",
    ];
    let path_str = path.as_os_str().to_string_lossy();
    let mut segments = path_str
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty());

    if segments
        .clone()
        .any(|segment| ignored_dirs.contains(&segment))
    {
        return true;
    }

    // Hidden files and editor swap files.
    if let Some(name) = segments.next_back() {
        if name.starts_with('.') || name.ends_with('~') || name.ends_with(".swp") {
            return true;
        }
    }
    false
}

/// Returns `true` if the file has a recognised source-code extension.
fn is_source_file(path: &Path) -> bool {
    let ext = match path.extension() {
        Some(e) => e.to_string_lossy().to_ascii_lowercase(),
        None => return false,
    };

    matches!(
        ext.as_str(),
        // ── Systems ──────────────────────────────────────────────────────
        "rs"    // Rust
        | "c"   // C
        | "h"   // C/C++ header
        | "cpp" // C++
        | "cc"  // C++
        | "cxx" // C++
        | "hpp" // C++ header
        | "hxx" // C++ header
        | "hh"  // C++ header
        | "m"   // Objective-C
        | "mm"  // Objective-C++
        | "go"  // Go
        | "zig" // Zig
        | "nim" // Nim
        | "d"   // D
        | "v"   // V / Verilog
        | "sv"  // SystemVerilog
        | "vhd" // VHDL
        | "vhdl"// VHDL
        | "s"   // Assembly
        | "asm" // Assembly
        // ── JVM ──────────────────────────────────────────────────────────
        | "java"    // Java
        | "kt"      // Kotlin
        | "kts"     // Kotlin script
        | "scala"   // Scala
        | "sc"      // Scala script
        | "groovy"  // Groovy
        | "clj"     // Clojure
        | "cljs"    // ClojureScript
        | "cljc"    // Clojure common
        | "edn"     // Clojure data
        // ── .NET / Microsoft ─────────────────────────────────────────────
        | "cs"   // C#
        | "fs"   // F#
        | "fsx"  // F# script
        | "vb"   // Visual Basic
        | "xaml" // XAML
        | "razor"// Razor
        | "cshtml" // Razor pages
        | "csx"    // C# script
        | "ps1"    // PowerShell
        | "psm1"   // PowerShell module
        | "psd1"   // PowerShell data
        // ── Web / JavaScript ─────────────────────────────────────────────
        | "js"   // JavaScript
        | "mjs"  // ES module
        | "cjs"  // CommonJS
        | "jsx"  // React JSX
        | "ts"   // TypeScript
        | "mts"  // TypeScript ES module
        | "cts"  // TypeScript CommonJS
        | "tsx"  // React TSX
        | "vue"  // Vue
        | "svelte" // Svelte
        | "astro"  // Astro
        | "html"   // HTML
        | "htm"    // HTML
        | "css"    // CSS
        | "scss"   // SCSS
        | "sass"   // Sass
        | "less"   // Less
        | "styl"   // Stylus
        | "coffee" // CoffeeScript
        | "wasm"   // WebAssembly
        | "wat"    // WebAssembly text
        // ── Python ───────────────────────────────────────────────────────
        | "py"   // Python
        | "pyi"  // Python stub
        | "pyx"  // Cython
        | "pxd"  // Cython declaration
        | "pyw"  // Python (Windows)
        | "ipynb"// Jupyter notebook
        // ── Ruby ─────────────────────────────────────────────────────────
        | "rb"   // Ruby
        | "erb"  // ERB template
        | "rake" // Rakefile
        | "gemspec" // Gemspec
        // ── PHP ──────────────────────────────────────────────────────────
        | "php"  // PHP
        | "phtml"// PHP template
        | "blade" // Laravel Blade
        // ── Shell / scripting ────────────────────────────────────────────
        | "sh"   // Bash/Shell
        | "bash" // Bash
        | "zsh"  // Zsh
        | "fish" // Fish
        | "bat"  // Windows batch
        | "cmd"  // Windows command
        | "awk"  // AWK
        | "sed"  // Sed
        // ── Functional ──────────────────────────────────────────────────
        | "hs"   // Haskell
        | "lhs"  // Literate Haskell
        | "ml"   // OCaml
        | "mli"  // OCaml interface
        | "re"   // ReasonML
        | "rei"  // ReasonML interface
        | "res"  // ReScript
        | "resi" // ReScript interface
        | "elm"  // Elm
        | "erl"  // Erlang
        | "hrl"  // Erlang header
        | "ex"   // Elixir
        | "exs"  // Elixir script
        | "heex" // Phoenix LiveView
        | "sml"  // Standard ML
        | "sig"  // SML signature
        | "rkt"  // Racket
        | "scm"  // Scheme
        | "ss"   // Scheme
        | "lisp" // Common Lisp
        | "lsp"  // Lisp
        | "cl"   // Common Lisp
        | "el"   // Emacs Lisp
        | "fnl"  // Fennel
        // ── Apple / mobile ──────────────────────────────────────────────
        | "swift"// Swift
        | "dart" // Dart / Flutter
        // ── Data / config as code ───────────────────────────────────────
        | "sql"     // SQL
        | "graphql" // GraphQL
        | "gql"     // GraphQL
        | "proto"   // Protocol Buffers
        | "thrift"  // Thrift
        | "avsc"    // Avro schema
        // ── Config / IaC ────────────────────────────────────────────────
        | "json"    // JSON
        | "jsonc"   // JSON with comments
        | "json5"   // JSON5
        | "yaml"    // YAML
        | "yml"     // YAML
        | "toml"    // TOML
        | "xml"     // XML
        | "xsl"     // XSLT
        | "xslt"    // XSLT
        | "ini"     // INI
        | "cfg"     // Config
        | "conf"    // Config
        | "tf"      // Terraform
        | "tfvars"  // Terraform vars
        | "hcl"     // HashiCorp HCL
        | "nix"     // Nix
        | "dhall"   // Dhall
        // ── Markup / docs ───────────────────────────────────────────────
        | "md"      // Markdown
        | "mdx"     // MDX
        | "rst"     // reStructuredText
        | "tex"     // LaTeX
        | "latex"   // LaTeX
        | "typ"     // Typst
        | "adoc"    // AsciiDoc
        | "org"     // Org mode
        | "wiki"    // Wiki markup
        // ── DevOps / containers ─────────────────────────────────────────
        | "dockerfile" // Dockerfile (extension-based)
        | "containerfile" // Containerfile
        // ── GPU / shaders ───────────────────────────────────────────────
        | "glsl"    // GLSL
        | "vert"    // Vertex shader
        | "frag"    // Fragment shader
        | "hlsl"    // HLSL
        | "metal"   // Metal
        | "wgsl"    // WebGPU
        | "cu"      // CUDA
        | "cuh"     // CUDA header
        // ── Game engines ────────────────────────────────────────────────
        | "gd"      // GDScript (Godot)
        | "gdshader"// Godot shader
        | "tres"    // Godot resource
        | "tscn"    // Godot scene
        | "lua"     // Lua
        | "moon"    // MoonScript
        | "wren"    // Wren
        | "squirrel"// Squirrel
        | "nut"     // Squirrel
        // ── Scientific / data ───────────────────────────────────────────
        | "r"       // R
        | "rmd"     // R Markdown
        | "jl"      // Julia
        | "mat"     // MATLAB data
        | "f"       // Fortran
        | "f90"     // Fortran 90
        | "f95"     // Fortran 95
        | "f03"     // Fortran 2003
        | "f08"     // Fortran 2008
        | "for"     // Fortran
        | "sas"     // SAS
        | "do"      // Stata
        | "ado"     // Stata
        | "nb"      // Mathematica
        | "wl"      // Wolfram Language
        // ── Misc / emerging ─────────────────────────────────────────────
        | "pl"      // Perl
        | "pm"      // Perl module
        | "t"       // Perl test
        | "p6"      // Raku (Perl 6)
        | "raku"    // Raku
        | "tcl"     // Tcl
        | "tk"      // Tk
        | "cr"      // Crystal
        | "hx"      // Haxe
        | "pony"    // Pony
        | "odin"    // Odin
        | "jai"     // Jai
        | "vale"    // Vale
        | "move"    // Move
        | "sol"     // Solidity
        | "vy"      // Vyper
        | "cairo"   // Cairo
        | "fe"      // Fe
        | "ab"      // Amber
        | "mojo"    // Mojo
        | "gleam"   // Gleam
        | "unison"  // Unison
        | "pkl"     // Pkl
        | "bsv"     // Bluespec
        | "cue"     // CUE
        | "jsonnet" // Jsonnet
        | "libsonnet" // Jsonnet lib
        | "starlark"  // Starlark
        | "bzl"       // Bazel/Starlark
        | "buck"      // Buck
        | "cmake"     // CMake
        | "make"      // Makefile (extension)
        | "mk"        // Makefile
        | "gradle"    // Gradle
        | "sbt"       // SBT
        | "cabal"     // Cabal
        | "pro"       // Qt project / Prolog
        | "prisma"    // Prisma
        | "rego"      // Rego (OPA)
        | "polar"     // Polar (Oso)
        | "pest"      // Pest (Rust parser)
        | "g4"        // ANTLR
        | "peg"       // PEG grammar
        // ── Template engines ────────────────────────────────────────────
        | "j2"      // Jinja2
        | "jinja"   // Jinja
        | "jinja2"  // Jinja2
        | "hbs"     // Handlebars
        | "mustache"// Mustache
        | "ejs"     // EJS
        | "pug"     // Pug
        | "slim"    // Slim
        | "haml"    // Haml
        | "twig"    // Twig
        | "liquid"  // Liquid
        | "njk"     // Nunjucks
        | "eta" // Eta
    )
}

#[cfg(test)]
mod tests {
    use super::{is_source_file, should_ignore};
    use std::path::Path;

    #[test]
    fn ignores_common_directories_with_unix_paths() {
        assert!(should_ignore(Path::new("/tmp/project/.git/config")));
        assert!(should_ignore(Path::new(
            "/tmp/project/node_modules/react/index.js"
        )));
    }

    #[test]
    fn ignores_common_directories_with_windows_paths() {
        assert!(should_ignore(Path::new(
            r"C:\Users\dev\project\.git\config"
        )));
        assert!(should_ignore(Path::new(
            r"C:\Users\dev\project\node_modules\react\index.js"
        )));
    }

    #[test]
    fn ignores_hidden_and_swap_files() {
        assert!(should_ignore(Path::new("/tmp/project/.env")));
        assert!(should_ignore(Path::new("/tmp/project/main.rs.swp")));
    }

    #[test]
    fn keeps_normal_source_files() {
        assert!(!should_ignore(Path::new("/tmp/project/src/main.rs")));
    }

    #[test]
    fn recognises_source_files() {
        assert!(is_source_file(Path::new("main.rs")));
        assert!(is_source_file(Path::new("app.tsx")));
        assert!(is_source_file(Path::new("script.py")));
        assert!(is_source_file(Path::new("index.html")));
        assert!(is_source_file(Path::new("query.sql")));
        assert!(is_source_file(Path::new("shader.glsl")));
    }

    #[test]
    fn rejects_non_source_files() {
        assert!(!is_source_file(Path::new("photo.jpg")));
        assert!(!is_source_file(Path::new("document.pdf")));
        assert!(!is_source_file(Path::new("data.bin")));
        assert!(!is_source_file(Path::new("archive.zip")));
        assert!(!is_source_file(Path::new("no_extension")));
    }
}
