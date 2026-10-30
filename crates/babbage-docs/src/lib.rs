//! Compiles and runs every Rust sample in the developer guide (`docs/dev/*.md`) as a doctest, so the guide cannot
//! show code that no longer builds or answers that the engine no longer gives. There is nothing to use here.

#[cfg(doctest)]
mod guide {
    #[doc = include_str!("../../../docs/dev/01-getting-started.md")]
    mod getting_started {}
    #[doc = include_str!("../../../docs/dev/02-evaluating-lines.md")]
    mod evaluating_lines {}
    #[doc = include_str!("../../../docs/dev/03-showing-answers.md")]
    mod showing_answers {}
    #[doc = include_str!("../../../docs/dev/04-sheets.md")]
    mod sheets {}
    #[doc = include_str!("../../../docs/dev/05-settings-and-time.md")]
    mod settings_and_time {}
    #[doc = include_str!("../../../docs/dev/06-errors-and-guarantees.md")]
    mod errors_and_guarantees {}
    #[doc = include_str!("../../../docs/dev/07-recipes.md")]
    mod recipes {}
}
