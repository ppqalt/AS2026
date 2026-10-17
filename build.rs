fn main() {
    #[cfg(feature = "gui")]
    {
        // Slint 1.16:n oma tekstikentän valikko käyttää ContextMenuInternal-
        // tyyppiä. Lippu sallii sen paikallisessa vakiokomponenttikopiossa.
        // SAFETY: rakennusskripti on tässä vielä yksisäikeinen; ympäristöä
        // muutetaan ennen Slint-kääntäjän tai sen työntekijöiden käynnistystä.
        unsafe { std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1") };
        let manifest_dir = std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").expect("Projektihakemisto puuttuu"),
        );
        let config = slint_build::CompilerConfiguration::new()
            .with_style("fluent".into())
            .with_include_paths(vec![manifest_dir.join("ui/fluent/style")])
            .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles)
            .with_bundled_translations("translations")
            .with_default_translation_context(slint_build::DefaultTranslationContext::None);
        slint_build::compile_with_config("ui/app.slint", config)
            .expect("AS2026:n käyttöliittymän kääntäminen epäonnistui");
    }
}
