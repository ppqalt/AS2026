//! Versionhallittujen tiedostojen tulkkikielikielto. Dokumenttien historiakuvaukset sallitaan.
use std::{fs, path::Path, process::Command};

fn forbidden_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    [
        ".py", ".pyw", ".pyi", ".ipynb", ".pyc", ".pyo", ".whl", ".pyd",
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix))
        || [
            "pyproject.toml",
            "pipfile",
            "pipfile.lock",
            "setup.cfg",
            "tox.ini",
            "poetry.lock",
            "uv.lock",
            ".python-version",
        ]
        .contains(&name.as_str())
        || (name.starts_with("requirements") && name.ends_with(".txt"))
}
fn interpreter(word: &str) -> bool {
    let word = word
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '/' | '_' | '-'));
    let word = word
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(word)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    ["python", "pip", "pypy", "pytest"].iter().any(|prefix| {
        word.strip_prefix(prefix).is_some_and(|tail| {
            tail.is_empty() || tail.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
    })
}
fn command(line: &str) -> bool {
    line.split([';', '|', '&']).any(|part| {
        let words: Vec<_> = part.split_whitespace().collect();
        if words
            .first()
            .is_some_and(|word| matches!(*word, "echo" | "printf" | "Write-Output" | "Write-Host"))
        {
            return false;
        }
        words.iter().any(|word| interpreter(word))
    })
}
fn violations(path: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let extension = Path::new(path)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("");
    let docs = matches!(extension, "md" | "txt" | "rst");
    let rust = extension == "rs";
    let workflow = path.starts_with(".github/workflows/");
    let shell = matches!(extension, "sh" | "bash" | "ps1" | "cmd" | "bat" | "fish")
        || path.starts_with("scripts/");
    let mut run_indent = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#!") && command(trimmed) {
            errors.push(format!(
                "{path}:{}: kielletty tulkki shebang-rivillä",
                index + 1
            ));
        }
        if docs || trimmed.starts_with(['#', '/']) || trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if run_indent.is_some_and(|n| indent <= n) {
            run_indent = None;
        }
        let run = trimmed.strip_prefix("run:").map(str::trim);
        if workflow && matches!(run, Some("|" | ">" | "|-" | ">-")) {
            run_indent = Some(indent);
        }
        let config_command = ["command", "script", "runner", "entrypoint", "exec"]
            .iter()
            .any(|key| {
                trimmed.starts_with(&format!("{key} =")) || trimmed.starts_with(&format!("{key}:"))
            });
        if (shell || (workflow && (run.is_some() || run_indent.is_some())) || config_command)
            && command(run.unwrap_or(trimmed))
        {
            errors.push(format!(
                "{path}:{}: kielletty suorituskomento: {trimmed}",
                index + 1
            ));
        }
        // Muun nimiseksi piilotettu skripti tunnistetaan lähdekielen tunnusomaisista riveistä.
        if !rust
            && !workflow
            && !matches!(extension, "toml" | "json" | "yaml" | "yml" | "po")
            && ((trimmed.starts_with("from ") && trimmed.contains(" import "))
                || (trimmed.strip_prefix("import ").is_some_and(|tail| tail.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')) && !trimmed.contains(" from "))
                || (trimmed.starts_with("def ") && trimmed.ends_with(':'))
                || trimmed.contains("__name__ ==")
                || trimmed.contains("__name__=="))
        {
            errors.push(format!("{path}:{}: kielletty lähdekieli", index + 1));
        }
    }
    if rust {
        let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        for prefix in ["Command::new(\"", ".arg(\""] {
            for tail in compact.split(prefix).skip(1) {
                if interpreter(tail.split('"').next().unwrap_or("")) {
                    errors.push(format!("{path}: kielletty tulkin käynnistys Rust-koodissa"));
                }
            }
        }
    }
    errors
}
pub fn check(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["ls-files", "-z", "--cached"])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("Versionhallittuja tiedostoja ei voitu luetella".into());
    }
    let mut errors = Vec::new();
    for bytes in output.stdout.split(|b| *b == 0).filter(|b| !b.is_empty()) {
        let path = std::str::from_utf8(bytes).map_err(|_| "Tiedostopolku ei ole UTF-8-muodossa")?;
        if forbidden_path(path) {
            errors.push(format!("{path}: kielletty tiedosto"));
        }
        let data = fs::read(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
        if let Ok(text) = std::str::from_utf8(&data) {
            errors.extend(violations(path, text));
        }
    }
    if errors.is_empty() {
        println!(
            "Rust-työkaluketjun tarkistus läpäisty: ei Python-tiedostoja tai suorituskomentoja."
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_and_executions_are_rejected_but_history_and_rust_literals_are_allowed() {
        for path in [
            "test.PY",
            "x.pyw",
            "x.pyi",
            "x.ipynb",
            "pyproject.toml",
            "Pipfile",
            "setup.py",
            "setup.cfg",
            "requirements-dev.txt",
            "tox.ini",
            "uv.lock",
        ] {
            assert!(forbidden_path(path), "{path}");
        }
        for (path, text) in [
            ("scripts/run", "#!/usr/bin/env python3"),
            ("tools/hidden", "from pathlib import Path\ndef run():"),
            (".github/workflows/test.yml", "run: python3 test"),
            (
                ".github/workflows/test.yml",
                "run: |\n  xvfb-run -a python test",
            ),
            ("run.sh", "uv run python test"),
            ("run.ps1", "& python test"),
            ("test.rs", "Command::new(\"python3\")"),
        ] {
            assert!(!violations(path, text).is_empty(), "{path}: {text}");
        }
        assert!(violations("docs/history.md", "Vanha komento:\npython3 old.py").is_empty());
        assert!(violations("guard.rs", "let fixture = \"import example\";").is_empty());
        assert!(violations("test.sh", "echo Pythonin kielto\ncargo test").is_empty());
    }
}
