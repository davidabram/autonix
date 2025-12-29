use std::fmt::Write;

pub fn escape_nix_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '$' if matches!(chars.peek(), Some('{')) => out.push_str("\\$"),
            _ => out.push(ch),
        }
    }

    out
}

pub fn write_nix_string_binding(buf: &mut String, indent: &str, name: &str, value: &str) {
    let escaped = escape_nix_string(value);
    writeln!(buf, "{indent}{name} = \"{escaped}\";").unwrap();
}

pub fn write_attr_with_fallback(
    buf: &mut String,
    indent: &str,
    bind_name: &str,
    want_attr_var: &str,
    scope: &str,
    fallback: &str,
) {
    let fallback_escaped = escape_nix_string(fallback);
    writeln!(
        buf,
        "{indent}{bind_name} = if builtins.hasAttr {want_attr_var} {scope} then {want_attr_var} else \"{fallback_escaped}\";"
    )
    .unwrap();
}

pub fn write_optional_package(buf: &mut String, indent: &str, package_var: &str) {
    writeln!(
        buf,
        "{indent}++ lib.optional ({package_var} != null) {package_var}"
    )
    .unwrap();
}

pub struct NoticeListBuilder {
    indent: String,
}

impl NoticeListBuilder {
    pub fn new(indent: &str) -> Self {
        Self {
            indent: indent.to_string(),
        }
    }

    pub fn build(&self, notice: Option<&str>) -> String {
        let mut buf = String::new();
        let child_indent = format!("{}  ", self.indent);

        if let Some(msg) = notice {
            let escaped = escape_nix_string(msg);
            writeln!(buf, "{}notices = [", self.indent).unwrap();
            writeln!(buf, "{child_indent}\"{escaped}\"").unwrap();
            writeln!(buf, "{}];", self.indent).unwrap();
        } else {
            writeln!(buf, "{}notices = [];", self.indent).unwrap();
        }

        buf
    }
}

pub struct CheckDerivationBuilder {
    key: String,
    derivation_name: String,
    display: String,
    required_exec: String,
    command: String,
    workdir: String,
}

impl CheckDerivationBuilder {
    pub fn new(
        key: String,
        derivation_name: String,
        display: String,
        required_exec: String,
        command: String,
        workdir: String,
    ) -> Self {
        Self {
            key,
            derivation_name,
            display,
            required_exec,
            command,
            workdir,
        }
    }

    pub fn build(&self) -> String {
        let mut out = String::new();

        let key_escaped = escape_nix_string(&self.key);
        let drv_escaped = escape_nix_string(&self.derivation_name);
        let display_escaped = escape_nix_string(&self.display);
        let required_exec_escaped = escape_nix_string(&self.required_exec);
        let cmd_escaped = escape_nix_string(&self.command);
        let workdir_escaped = escape_nix_string(&self.workdir);

        writeln!(out, "  \"{key_escaped}\" = let").unwrap();
        writeln!(out, "    cmd = \"{cmd_escaped}\";").unwrap();
        writeln!(out, "    requiredExec = \"{required_exec_escaped}\";").unwrap();
        writeln!(out, "    workdir = \"{workdir_escaped}\";").unwrap();
        writeln!(out, "    display = \"{display_escaped}\";").unwrap();
        writeln!(out, "  in pkgs.runCommand \"{drv_escaped}\" {{").unwrap();
        out.push_str("    nativeBuildInputs = devPackages;\n");
        out.push_str("  } ''\n");
        out.push_str("    set -euo pipefail\n");
        out.push_str("    export HOME=\"$TMPDIR/home\"\n");
        out.push_str("    mkdir -p \"$HOME\"\n");
        out.push('\n');
        out.push_str("    echo \"autonix: running ${display}\"\n");
        out.push_str("    if ! command -v \"${requiredExec}\" >/dev/null 2>&1; then\n");
        out.push_str(
            "      echo \"autonix: missing required executable in PATH: ${requiredExec}\" >&2\n",
        );
        out.push_str("      exit 1\n");
        out.push_str("    fi\n");
        out.push('\n');
        out.push_str("    cp -r ${projectRoot} source\n");
        out.push_str("    chmod -R u+w source\n");
        out.push_str("    cd \"source/${workdir}\"\n");
        out.push('\n');
        out.push_str("    ${pkgs.bash}/bin/bash -lc ${lib.escapeShellArg cmd}\n");
        out.push('\n');
        out.push_str("    mkdir -p $out\n");
        out.push_str("    echo ok > $out/result\n");
        out.push_str("  '';\n\n");

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_nix_string_quotes() {
        assert_eq!(escape_nix_string(r#"hello "world""#), r#"hello \"world\""#);
    }

    #[test]
    fn test_escape_nix_string_dollar_brace() {
        assert_eq!(escape_nix_string("${foo}"), r"\${foo}");
        assert_eq!(escape_nix_string("$foo"), "$foo");
    }

    #[test]
    fn test_escape_nix_string_backslash() {
        assert_eq!(escape_nix_string(r"C:\path\to\file"), r"C:\\path\\to\\file");
    }

    #[test]
    fn test_escape_nix_string_newlines() {
        assert_eq!(
            escape_nix_string("line1\nline2\r\nline3"),
            r"line1\nline2\r\nline3"
        );
    }

    #[test]
    fn test_write_nix_string_binding() {
        let mut buf = String::new();
        write_nix_string_binding(&mut buf, "  ", "myVar", "hello world");
        assert_eq!(buf, "  myVar = \"hello world\";\n");
    }

    #[test]
    fn test_write_nix_string_binding_with_escaping() {
        let mut buf = String::new();
        write_nix_string_binding(&mut buf, "", "cmd", r#"echo "${VAR}""#);
        assert_eq!(buf, "cmd = \"echo \\\"\\${VAR}\\\"\";\n");
    }

    #[test]
    fn test_notice_list_builder_with_notice() {
        let builder = NoticeListBuilder::new("  ");
        let result = builder.build(Some("Test notice"));
        assert!(result.contains("notices = ["));
        assert!(result.contains("\"Test notice\""));
    }

    #[test]
    fn test_notice_list_builder_empty() {
        let builder = NoticeListBuilder::new("  ");
        let result = builder.build(None);
        assert_eq!(result, "  notices = [];\n");
    }

    #[test]
    fn test_check_derivation_builder() {
        let builder = CheckDerivationBuilder::new(
            "test-key".to_string(),
            "check-test-key".to_string(),
            "npm run test".to_string(),
            "npm".to_string(),
            "npm run test".to_string(),
            ".".to_string(),
        );
        let result = builder.build();

        assert!(result.contains("\"test-key\" = let"));
        assert!(result.contains("cmd = \"npm run test\";"));
        assert!(result.contains("requiredExec = \"npm\";"));
        assert!(result.contains("pkgs.runCommand \"check-test-key\""));
    }
}
