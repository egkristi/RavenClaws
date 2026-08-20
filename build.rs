fn main() {
    // Compile the Slint GUI only when the `gui` feature is enabled, so the
    // default build stays small and doesn't require the Slint toolchain.
    #[cfg(feature = "gui")]
    slint_build::compile("ui/main.slint").expect("Slint build failed");
}
