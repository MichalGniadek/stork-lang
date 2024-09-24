#[path = "ast/ast.rs"]
pub mod ast;
#[path = "cst/cst.rs"]
pub mod cst;
pub mod hir;
#[path = "module_index/module_index.rs"]
pub mod module_index;
#[path = "passes/passes.rs"]
pub mod passes;
pub mod report;

// [X] Add resources
// [X] Handling multiple items
// [X] Add custom components
// [ ] Add LSP
// [X] Get bevy function reflection [kinda blocked on them being polished more]
// [X] Add more builtin functions [kinda blocked on bevy function reflection]
// [X] Add control flow [kinda blocked on more buildin functions]
