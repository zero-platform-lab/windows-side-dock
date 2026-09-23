//! レジストリに書かれたコマンド行を、実行ファイルと引数に分ける。

/// `"C:\Tools\procexp64.exe" /t` なら `("C:\Tools\procexp64.exe", "/t")`、
/// `taskmgr.exe` なら `("taskmgr.exe", "")`。前後の空白は取り除く。
pub fn split_command(command: &str) -> (String, String) {
    let command = command.trim();
    let (file, rest) = match command.strip_prefix('"') {
        Some(quoted) => match quoted.split_once('"') {
            Some((file, rest)) => (file, rest),
            None => (quoted, ""),
        },
        None => command.split_once(' ').unwrap_or((command, "")),
    };
    (file.to_owned(), rest.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_and_plain_commands() {
        assert_eq!(
            split_command(r#" "E:\Tools\Process Explorer\procexp64.exe" /t "#),
            (
                r"E:\Tools\Process Explorer\procexp64.exe".into(),
                "/t".into()
            )
        );
        assert_eq!(
            split_command(r#""C:\a.exe""#),
            (r"C:\a.exe".into(), String::new())
        );
        assert_eq!(
            split_command(r#""C:\unterminated.exe"#),
            (r"C:\unterminated.exe".into(), String::new())
        );
        assert_eq!(
            split_command("taskmgr.exe"),
            ("taskmgr.exe".into(), String::new())
        );
        assert_eq!(
            split_command("cmd.exe /k echo"),
            ("cmd.exe".into(), "/k echo".into())
        );
    }
}
