//! Discovers every `#[story]` under `src/stories` and emits the registry.
//!
//! Adding a file there is all it takes to add stories — there is no `mod.rs`
//! to maintain, because the generated file declares the modules too.

fn main() {
    let index = dioxus_storybook_build::index("src/stories").expect("story indexing failed");
    println!("cargo:warning=indexed {} stories", index.len());
}
