fn main() {
    #[cfg(feature = "gui")]
    {
        let config = slint_build::CompilerConfiguration::new()
            .with_style("fluent".into())
            .with_bundled_translations("translations")
            .with_default_translation_context(slint_build::DefaultTranslationContext::None);
        slint_build::compile_with_config("ui/app.slint", config)
            .expect("AS2026:n käyttöliittymän kääntäminen epäonnistui");
    }
}
